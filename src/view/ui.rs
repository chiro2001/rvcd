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
use num_traits::Float;
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
            if ui
                .checkbox(&mut self.limit_range_left, "Limit Left Range")
                .clicked()
            {
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
            const EDIT_WIDTH: f32 = 100.0;
            ui.label("From:");
            let speed_min = 0.1;
            let old_range = self.range.clone();
            let drag_value = DragValue::new(&mut self.range.0)
                .speed(f32::max((old_range.1 - old_range.0) / 100.0, speed_min));
            let range_right = f32::min(info.range.1 as f32 * ZOOM_SIZE_MAX_SCALE, old_range.1);
            let drag_value = if self.limit_range_left {
                drag_value.clamp_range(0.0..=range_right)
            } else {
                drag_value.clamp_range((-(info.range.1 as f32) * ZOOM_SIZE_MAX_SCALE)..=range_right)
            };
            drag_value.ui(ui);
            ui.label("To:");
            DragValue::new(&mut self.range.1)
                .speed(f32::max((old_range.1 - old_range.0) / 100.0, speed_min))
                .clamp_range(
                    (old_range.0 + ZOOM_SIZE_MIN)..=(info.range.1 as f32 * ZOOM_SIZE_MAX_SCALE),
                )
                .ui(ui);
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
            .column(Column::exact(self.wave_width).resizable(false))
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::infinity());
        let table = if let Some(scrolling_last_index) = self.scrolling_last_index.take() {
            table.scroll_to_row(scrolling_last_index, Some(Align::TOP))
        } else {
            table
        };
        let table = if let Some(scrolling_next_index) = self.scrolling_next_index.take() {
            table.scroll_to_row(scrolling_next_index, Some(Align::BOTTOM))
        } else {
            table
        };
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
        let mut right_drag_start_pos = None;
        let mut right_drag_pos = None;
        let mut last_paint_row_index = None;
        let mut new_signals = vec![];
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
                        last_paint_row_index = Some(row_index);
                        if let Some(signal) = signal {
                            row.col(|ui| {
                                if let Some(signal_new) = self.ui_signal_label(signal, ui) {
                                    new_signals.push(signal_new);
                                }
                            });
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
                                if response.dragged_by(PointerButton::Secondary) {
                                    let p = response
                                        .interact_pointer_pos()
                                        .map(|p| pos2(p.x - wave_left, p.y));
                                    right_drag_pos = p;
                                    right_drag_start_pos = p;
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
                                        new_range = self.move_horizontal(-scroll.x);
                                    }
                                }
                            });
                        }
                    },
                );
            });
        // update signal information
        let signals_updated = self
            .signals
            .iter()
            .map(|x| x.clone())
            .map(|x| match new_signals.iter().find(|c| c.s.id == x.s.id) {
                None => x,
                Some(c) => c.clone(),
            })
            .collect();
        self.signals = signals_updated;
        if self.limit_range_left && new_range.0 < 0.0 {
            new_range.0 = 0.0;
        }
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
            if right_drag_start_pos.is_some() && self.right_drag_start_pos.is_none() {
                self.right_drag_start_pos = right_drag_start_pos;
            }
            if drag_release {
                self.right_drag_start_pos = None;
            }
            if let Some(right_drag_start_pos) = self.right_drag_start_pos {
                if let Some(right_drag_pos) = right_drag_pos {
                    let delta = right_drag_pos - right_drag_start_pos;
                    if let Some(right_drag_last_pos) = self.right_drag_last_pos {
                        // Handle drag move
                        let delta = right_drag_pos - right_drag_last_pos;
                        let dx = -delta.x;
                        self.range = self.move_horizontal(dx);
                    }
                    // natural direction
                    let dy = -delta.y;
                    // Handle right drag
                    // TODO: here we cannot get real `first_paint_row_index`, only to get from last
                    // simply use last paint index
                    if let Some(first_paint_row_index) = last_paint_row_index {
                        debug!("first_paint_row_index: {}", first_paint_row_index);
                        if let Some(signal) = self.signals.get(first_paint_row_index) {
                            if dy < -signal.height {
                                debug!("to last signal");
                                self.scrolling_next_index =
                                    Some(i64::max(first_paint_row_index as i64 - 1, 0) as usize);
                                self.right_drag_start_pos = Some(right_drag_pos);
                            }
                        }
                    }
                    if let Some(last_paint_row_index) = last_paint_row_index {
                        if let Some(signal) = self.signals.get(last_paint_row_index) {
                            if dy > signal.height {
                                debug!("to next signal");
                                self.scrolling_next_index = Some(usize::min(
                                    last_paint_row_index + 1,
                                    self.signals.len() - 1,
                                ));
                                self.right_drag_start_pos = Some(right_drag_pos);
                            }
                        }
                    }
                }
            }
            if right_drag_pos.is_some() {
                self.right_drag_last_pos = right_drag_pos;
            }
            if drag_release {
                self.right_drag_last_pos = None;
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
    pub fn move_horizontal(&self, dx: f32) -> (f32, f32) {
        let pos_delta = self.x_to_fpos(dx) - self.range.0;
        let new_range_check = (self.range.0 + pos_delta, self.range.1 + pos_delta);
        if new_range_check.0 < 0.0 {
            if self.limit_range_left {
                (0.0, new_range_check.1 - new_range_check.0)
            } else {
                new_range_check
            }
        } else {
            new_range_check
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
