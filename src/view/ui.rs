use crate::message::RvcdMsg;
use crate::radix::Radix;
use crate::view::cursor::WaveCursor;
use crate::view::signal::SIGNAL_HEIGHT_DEFAULT;
use crate::view::{
    WaveView, BG_MULTIPLY, LINE_WIDTH, UI_WIDTH_OFFSET, ZOOM_SIZE_MAX_SCALE, ZOOM_SIZE_MIN,
};
use crate::wave::{Wave, WaveDataItem, WaveInfo};
use egui::*;
use egui_extras::{Column, TableBuilder};
use std::ops::RangeInclusive;
use tracing::{debug, warn};

impl WaveView {
    /// Paint view menu
    pub fn menu(&mut self, ui: &mut Ui) {
        ui.menu_button("View", |ui| {
            ui.menu_button(format!("Default Radix: {:?}", self.default_radix), |ui| {
                use Radix::*;
                let data = [Hex, Oct, Dec, Bin];
                data.into_iter().for_each(|r| {
                    if ui.button(format!("{:?}", r)).clicked() {
                        self.default_radix = r;
                        ui.close_menu();
                    }
                });
            });
            ui.menu_button(format!("Align: {:?}", self.align), |ui| {
                let data = [
                    super::SignalViewAlign::Left,
                    super::SignalViewAlign::Center,
                    super::SignalViewAlign::Right,
                ];
                data.into_iter().for_each(|a| {
                    if ui.button(format!("{:?}", a)).clicked() {
                        self.align = a;
                        ui.close_menu();
                    }
                });
            });
            if ui.checkbox(&mut self.background, "Background").clicked() {
                ui.close_menu();
            }
            if ui.checkbox(&mut self.show_text, "Show Text").clicked() {
                ui.close_menu();
            }
            ui.horizontal(|ui| {
                ui.label("Value font size ");
                DragValue::new(&mut self.signal_font_size)
                    .clamp_range(10.0..=20.0)
                    .speed(0.05)
                    .suffix(" px")
                    .ui(ui);
            });
        });
    }
    /// Paint toolbar above wave panel
    pub fn toolbar(&mut self, ui: &mut Ui, info: &WaveInfo) {
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            if ui.button("⛔ Clear").clicked() {
                self.signals.clear();
            }
            if ui.button("↩ Reset View").clicked() {
                self.range = (info.range.0 as f32, info.range.1 as f32);
            }
            if ui.button("🔄 Reload File").clicked() {
                if let Some(tx) = &self.tx {
                    debug!("reload msg sent");
                    tx.send(RvcdMsg::Reload).unwrap();
                } else {
                    warn!("no tx in view!");
                }
            }
            ui.label(format!(
                "view range: #[{:.2} ~ {:.2}]",
                self.range.0, self.range.1
            ));
            const EDIT_WIDTH: f32 = 100.0;
            ui.label("From:");
            let mut from_text = format!("{}", self.range.0 as i64);
            ui.with_layout(Layout::default(), |ui| {
                ui.set_width(EDIT_WIDTH);
                ui.text_edit_singleline(&mut from_text);
            });
            ui.label("To:");
            let mut to_text = format!("{}", self.range.1 as i64);
            ui.with_layout(Layout::default(), |ui| {
                ui.set_width(EDIT_WIDTH);
                ui.text_edit_singleline(&mut to_text);
            });
            if let Ok(value) = from_text.parse::<u64>() {
                let value = value as f32;
                let d = self.range.1 - value;
                if d > ZOOM_SIZE_MIN
                    && d < ZOOM_SIZE_MAX_SCALE * (info.range.1 - info.range.0) as f32
                {
                    self.range.0 = value;
                }
            }
            if let Ok(value) = to_text.parse::<u64>() {
                let value = value as f32;
                let d = value - self.range.0;
                if d > ZOOM_SIZE_MIN
                    && d < ZOOM_SIZE_MAX_SCALE * (info.range.1 - info.range.0) as f32
                {
                    self.range.1 = value;
                }
            }
        });
    }
    /// Paint wave panel
    pub fn panel(&mut self, ui: &mut Ui, wave: &Wave) {
        let info: &WaveInfo = &wave.info;
        let wave_data: &[WaveDataItem] = &wave.data;
        if self.range.0 == 0.0 && self.range.1 == 0.0 {
            self.range = (info.range.0 as f32, info.range.1 as f32);
        }
        TopBottomPanel::top("wave_top")
            .resizable(false)
            .show_inside(ui, |ui| {
                self.toolbar(ui, &wave.info);
            });
        // bugs by: https://github.com/emilk/egui/issues/2430
        let use_rect = ui.max_rect();
        const DEFAULT_MIN_SIGNAL_WIDTH: f32 = 150.0;
        let fix_width = f32::max(
            self.signals
                .iter()
                .map(|x| x.s.name.len())
                .max()
                .unwrap_or(0) as f32
                * 8.0,
            DEFAULT_MIN_SIGNAL_WIDTH,
        );
        self.wave_width = use_rect.width() - fix_width;
        let table = TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            // .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .cell_layout(Layout::centered_and_justified(Direction::TopDown))
            .column(Column::exact(fix_width).resizable(false))
            .column(Column::exact(self.wave_width).resizable(false));
        // .column(Column::auto())
        // .column(Column::remainder());
        let mut wave_left: f32 = fix_width + use_rect.left() + UI_WIDTH_OFFSET;
        let mut pos = None;
        let mut drag_started = false;
        let mut drag_release = false;
        let mut drag_by_primary = false;
        let mut drag_by_secondary = false;
        let mut drag_by_middle = false;
        let mut new_range = self.range.clone();
        table
            .header(SIGNAL_HEIGHT_DEFAULT, |mut header| {
                header.col(|ui| {
                    ui.strong(format!(
                        "Time #{}~#{} {}{}",
                        info.range.0, info.range.1, info.timescale.0, info.timescale.1
                    ));
                });
                header.col(|ui| {
                    self.time_bar(ui, info, wave_left);
                });
            })
            .body(|body| {
                body.heterogeneous_rows(
                    self.signals.iter().map(|x| x.height),
                    |row_index, mut row| {
                        let signal = self.signals.get(row_index);
                        if let Some(signal) = signal {
                            row.col(|ui| self.ui_signal_label(signal, ui));
                            row.col(|ui| {
                                let response = self.ui_signal_wave(signal, wave_data, info, ui);
                                let pos_hover = response.hover_pos();
                                wave_left = ui.available_rect_before_wrap().left();
                                if let Some(pointer_pos) = response.interact_pointer_pos() {
                                    pos = Some(pos2(pointer_pos.x - wave_left, pointer_pos.y));
                                    drag_started = response.drag_started();
                                    drag_release = response.drag_released();
                                    if response.dragged_by(PointerButton::Primary) {
                                        drag_by_primary = true;
                                    }
                                    if response.dragged_by(PointerButton::Secondary) {
                                        drag_by_secondary = true;
                                    }
                                    if response.dragged_by(PointerButton::Middle) {
                                        drag_by_middle = true;
                                    }
                                }
                                // catch mouse wheel events
                                if ui.rect_contains_pointer(use_rect) {
                                    let scroll = ui
                                        .ctx()
                                        .input()
                                        .events
                                        .iter()
                                        .find(|x| match x {
                                            Event::Scroll(_) => true,
                                            _ => false,
                                        })
                                        .map(|x| match x {
                                            Event::Scroll(v) => Some(*v),
                                            _ => None,
                                        })
                                        .flatten();
                                    let zoom = ui
                                        .ctx()
                                        .input()
                                        .events
                                        .iter()
                                        .find(|x| match x {
                                            Event::Zoom(_) => true,
                                            _ => false,
                                        })
                                        .map(|x| match x {
                                            Event::Zoom(v) => Some(*v),
                                            _ => None,
                                        })
                                        .flatten();
                                    if let Some(zoom) = zoom {
                                        let zoom = 1.0 / zoom;
                                        if let Some(pos) = pos_hover {
                                            let painter = ui.painter();
                                            let pos = pos2(pos.x - wave_left, pos.y);
                                            // zoom from this pos
                                            let center_pos = (pos.x
                                                * (self.range.1 - self.range.0)
                                                / self.wave_width
                                                + self.range.0)
                                                .clamp(info.range.0 as f32, info.range.1 as f32);
                                            painter.debug_rect(
                                                Rect::from_center_size(
                                                    pos2(
                                                        self.fpos_to_x(center_pos) + wave_left,
                                                        pos.y,
                                                    ),
                                                    vec2(4.0, 4.0),
                                                ),
                                                Color32::RED,
                                                "Center",
                                            );
                                            let left = (center_pos - self.range.0) * zoom;
                                            let right = (self.range.1 - center_pos) * zoom;
                                            let new_range_check =
                                                (center_pos - left, center_pos + right);
                                            let d = new_range_check.1 - new_range_check.0;
                                            if d > ZOOM_SIZE_MIN
                                                && d < ZOOM_SIZE_MAX_SCALE
                                                    * (info.range.1 - info.range.0) as f32
                                            {
                                                new_range = new_range_check;
                                            }
                                        }
                                    } else if let Some(scroll) = scroll {
                                        let x_delta = -scroll.x;
                                        let pos_delta = self.x_to_fpos(x_delta) - self.range.0;
                                        new_range =
                                            (self.range.0 + pos_delta, self.range.1 + pos_delta);
                                    }
                                }
                            });
                        }
                    },
                );
            });
        self.range = new_range;
        // info!("fix_width = {}, ui left = {}, wave_left = {}", fix_width, ui.max_rect().left(), wave_left);
        // info!("(fix_width + ui left) - wave_left = {}", fix_width + ui.max_rect().left() - wave_left);
        if let Some(pos) = pos {
            let painter = ui.painter();
            painter.text(
                pos + vec2(wave_left, 0.0),
                Align2::RIGHT_BOTTOM,
                format!("{:?}", pos),
                Default::default(),
                Color32::YELLOW,
            );
            if drag_by_primary {
                self.marker_temp.set_pos_valid(
                    self.x_to_pos(pos.x)
                        .clamp(self.range.0 as u64, self.range.1 as u64),
                );
            }
            if drag_release && self.marker_temp.valid {
                self.marker.set_pos_valid(
                    self.marker_temp
                        .pos
                        .clamp(self.range.0 as u64, self.range.1 as u64),
                );
            }
            if !drag_by_primary {
                self.marker_temp.valid = false;
            }
        }
        self.paint_span(ui, wave_left, info, pos, &self.marker, &self.marker_temp);
        // remove unavailable spans
        self.spans = self
            .spans
            .iter()
            .map(|x| x.clone())
            .filter(|s| self.cursors_exists_id(s.0) && self.cursors_exists_id(s.1))
            .collect();
        for span in &self.spans {
            if let Some(a) = self.cursors_get(span.0) {
                if let Some(b) = self.cursors_get(span.1) {
                    self.paint_span(ui, wave_left, info, None, a, b);
                }
            }
        }
        if self.marker.valid {
            self.paint_cursor(ui, wave_left, info, &self.marker);
        }
        if self.marker_temp.valid {
            self.paint_cursor(ui, wave_left, info, &self.marker_temp);
        }
        for cursor in &self.cursors {
            self.paint_cursor(ui, wave_left, info, cursor);
        }
    }
    /// Paint span between two cursors
    pub fn paint_span(
        &self,
        ui: &mut Ui,
        offset: f32,
        info: &WaveInfo,
        pos: Option<Pos2>,
        a: &WaveCursor,
        b: &WaveCursor,
    ) {
        let paint_rect = ui.max_rect();
        let painter = ui.painter();
        if a.valid && b.valid {
            let (a, b) = if a.pos < b.pos { (a, b) } else { (b, a) };
            let (x_a, x_b) = (self.pos_to_x(a.pos) + offset, self.pos_to_x(b.pos) + offset);
            let rect = Rect::from_min_max(pos2(x_a, paint_rect.min.y), pos2(x_b, paint_rect.max.y));
            painter.rect(
                rect,
                0.0,
                Color32::BLUE.linear_multiply(BG_MULTIPLY),
                (LINE_WIDTH, Color32::BLUE),
            );
            let y = match pos {
                None => paint_rect.top(),
                Some(pos) => pos.y,
            };
            painter.hline(
                RangeInclusive::new(x_a, x_b),
                y,
                (LINE_WIDTH, Color32::BLUE),
            );
            let time = self.pos_to_time(&info.timescale, b.pos - a.pos);
            painter.text(
                pos2((x_a + x_b) / 2.0, y),
                Align2::CENTER_BOTTOM,
                if a.pos <= b.pos {
                    format!("+{}", time)
                } else {
                    format!("-{}", time)
                },
                Default::default(),
                ui.visuals().strong_text_color(),
            );
        }
    }
}
