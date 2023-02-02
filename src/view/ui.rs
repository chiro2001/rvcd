use crate::message::RvcdMsg;
use crate::radix::Radix;
use crate::view::signal::SIGNAL_HEIGHT_DEFAULT;
use crate::view::{WaveView, BG_MULTIPLY, LINE_WIDTH, UI_WIDTH_OFFSET};
use crate::wave::{WaveDataItem, WaveInfo};
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
    pub fn toolbar(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            if ui.button("⛔ Clear").clicked() {
                self.signals.clear();
            }
            if ui.button("🔄 Reload").clicked() {
                if let Some(tx) = &self.tx {
                    debug!("reload msg sent");
                    tx.send(RvcdMsg::Reload).unwrap();
                } else {
                    warn!("no tx in view!");
                }
            }
        });
    }
    /// Paint wave panel
    pub fn panel(&mut self, ui: &mut Ui, info: &Option<WaveInfo>, wave_data: &[WaveDataItem]) {
        if let Some(info) = info {
            if self.range.0 == 0 && self.range.1 == 0 {
                self.range = info.range;
            }
        }
        TopBottomPanel::top("wave_top")
            .resizable(false)
            .show_inside(ui, |ui| {
                self.toolbar(ui);
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
                    if let Some(info) = info {
                        ui.strong(format!(
                            "Time #{}~#{} {}{}",
                            info.range.0, info.range.1, info.timescale.0, info.timescale.1
                        ));
                    }
                });
                header.col(|ui| {
                    if let Some(info) = info {
                        self.time_bar(ui, info, wave_left);
                    }
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
                                if let Some(info) = info {
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
                                        let _scroll = ui
                                            .ctx()
                                            .input()
                                            .events
                                            .iter()
                                            .find(|x| match x {
                                                Event::Scroll(_) => true,
                                                _ => false,
                                            })
                                            .map(|x| match x {
                                                Event::Scroll(v) => Some(v),
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
                                            if let Some(_pos) = pos_hover {
                                                // TODO: zoom from this pos
                                                new_range = (
                                                    self.range.0,
                                                    ((self.range.1 as f32 * zoom) as u64).clamp(
                                                        info.range.0
                                                            + (info.range.1 - info.range.0) / 200,
                                                        info.range.1 * 2,
                                                    ),
                                                );
                                            }
                                        }
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
                self.marker_temp
                    .set_pos_valid(self.x_to_pos(pos.x).clamp(self.range.0, self.range.1));
            }
            if drag_release && self.marker_temp.valid {
                self.marker
                    .set_pos_valid(self.marker_temp.pos.clamp(self.range.0, self.range.1));
            }
            if !drag_by_primary {
                self.marker_temp.valid = false;
            }
        }
        if let Some(info) = info {
            self.paint_span(ui, wave_left, info, pos);
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
    }
    /// Paint span between `self.marker` and `self.marker_temp`
    /// TODO: paint span between any cursors
    pub fn paint_span(&self, ui: &mut Ui, offset: f32, info: &WaveInfo, pos: Option<Pos2>) {
        let paint_rect = ui.max_rect();
        let painter = ui.painter();
        if self.marker.valid && self.marker_temp.valid {
            let (a, b) = if self.marker.pos < self.marker_temp.pos {
                (&self.marker, &self.marker_temp)
            } else {
                (&self.marker_temp, &self.marker)
            };
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
                if self.marker.pos <= self.marker_temp.pos {
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
