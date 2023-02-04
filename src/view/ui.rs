use crate::message::RvcdMsg;
use crate::radix::Radix;
use crate::utils::get_text_size;
use crate::view::cursor::WaveCursor;
use crate::view::{
    WaveView, BG_MULTIPLY, LINE_WIDTH, SIGNAL_HEIGHT_DEFAULT, UI_WIDTH_OFFSET, WAVE_MARGIN_TOP,
    WAVE_MARGIN_TOP2, ZOOM_SIZE_MAX_SCALE, ZOOM_SIZE_MIN,
};
use crate::wave::{Wave, WaveInfo};
use egui::*;
use egui_extras::{Column, TableBuilder};
use num_traits::Float;
use std::ops::RangeInclusive;
use tracing::{debug, warn};

#[derive(Default, Debug, Clone)]
pub struct ResponsePointerState {
    pub drag_started: bool,
    pub drag_release: bool,
    pub drag_by_primary: bool,
    pub drag_by_secondary: bool,
    pub drag_by_middle: bool,
    pub move_drag_start_pos: Option<Pos2>,
    pub move_drag_pos: Option<Pos2>,
}
impl ResponsePointerState {
    pub fn handle_pointer_response(&mut self, response: &Response, wave_left: f32) {
        if let Some(_pointer_pos) = response.interact_pointer_pos() {
            self.drag_started = response.drag_started();
            self.drag_release = response.drag_released();
            if response.dragged_by(PointerButton::Primary) {
                self.drag_by_primary = true;
            }
            if response.dragged_by(PointerButton::Secondary) {
                self.drag_by_secondary = true;
            }
            if response.dragged_by(PointerButton::Middle) {
                self.drag_by_middle = true;
            }
        }
        if response.dragged_by(PointerButton::Middle) {
            let p = response
                .interact_pointer_pos()
                .map(|p| pos2(p.x - wave_left, p.y));
            self.move_drag_pos = p;
            self.move_drag_start_pos = p;
        }
    }
}

#[derive(Default, Debug)]
pub struct ResponseHandleState {
    pub pointer: ResponsePointerState,
    pub new_range: (f32, f32),
}
impl ResponseHandleState {
    pub fn new(old_range: (f32, f32)) -> Self {
        Self {
            new_range: old_range,
            ..Default::default()
        }
    }
}

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
            if ui
                .checkbox(&mut self.use_top_margin, "Wave Panel Top Margin")
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .checkbox(&mut self.round_pointer, "Round Pointer")
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
        // if self.use_top_margin {
        //     ui.allocate_space(vec2(1.0, WAVE_MARGIN_TOP));
        // }
    }
    pub fn handle_response(
        &self,
        ui: &mut Ui,
        response: &Response,
        wave_left: f32,
        info: &WaveInfo,
        old_range: (f32, f32),
    ) -> ResponseHandleState {
        let mut state = ResponseHandleState::new(old_range);
        // catch mouse wheel events
        if ui.rect_contains_pointer(response.rect) {
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
                if let Some(pos) = response.hover_pos() {
                    let painter = ui.painter();
                    let pos = pos2(pos.x - wave_left, pos.y);
                    // zoom from this pos
                    let center_pos = (pos.x * (self.range.1 - self.range.0) / self.wave_width
                        + self.range.0)
                        .clamp(info.range.0 as f32, info.range.1 as f32);
                    painter.debug_rect(
                        Rect::from_center_size(
                            pos2(self.fpos_to_x(center_pos) + wave_left, pos.y),
                            vec2(4.0, 4.0),
                        ),
                        Color32::RED,
                        "Center",
                    );
                    let left = (center_pos - self.range.0) * zoom;
                    let right = (self.range.1 - center_pos) * zoom;
                    let new_range_check = (center_pos - left, center_pos + right);
                    let d = new_range_check.1 - new_range_check.0;
                    if d > ZOOM_SIZE_MIN
                        && d < ZOOM_SIZE_MAX_SCALE * (info.range.1 - info.range.0) as f32
                    {
                        state.new_range = new_range_check;
                    }
                }
            } else if let Some(scroll) = scroll {
                state.new_range = self.move_horizontal(-scroll.x, info);
            }
        }
        if self.limit_range_left && state.new_range.0 < 0.0 {
            state.new_range.0 = 0.0;
        }
        state
    }
    /// Paint wave panel
    pub fn panel(&mut self, ui: &mut Ui, wave: &Wave) {
        let info: &WaveInfo = &wave.info;
        if self.range.0 == 0.0 && self.range.1 == 0.0 {
            self.range = (info.range.0 as f32, info.range.1 as f32);
        }
        TopBottomPanel::top(format!("wave_top_{}", self.id))
            .resizable(false)
            .show_inside(ui, |ui| {
                self.toolbar(ui, &wave.info);
            });
        CentralPanel::default().show_inside(ui, |ui| {
            // bugs by: https://github.com/emilk/egui/issues/2430
            let use_rect = ui.max_rect();
            const DEFAULT_MIN_SIGNAL_WIDTH: f32 = 150.0;
            let fix_width = f32::max(
                self.signals
                    .iter()
                    .map(|x| get_text_size(ui, x.s.to_string().as_str(), Default::default()).x)
                    .reduce(f32::max)
                    .unwrap_or(0.0),
                DEFAULT_MIN_SIGNAL_WIDTH,
            );
            self.wave_width = use_rect.width() - fix_width;
            let mut wave_left: f32 = fix_width + use_rect.left() + UI_WIDTH_OFFSET;
            let mut new_signals = vec![];
            let mut last_paint_row_index = None;
            let mut dragging_pos = None;
            let mut pointer_state = ResponsePointerState::default();

            let max_rect = ui.max_rect();
            let inner_response = ui.allocate_ui(max_rect.size(), |ui| {
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
                                        if let Some(signal_new) =
                                            self.ui_signal_label(signal, row_index, ui)
                                        {
                                            new_signals.push(signal_new);
                                        }
                                    });
                                    row.col(|ui| {
                                        if let Some(data) = wave.data.get(&signal.s.id) {
                                            let response =
                                                self.ui_signal_wave(signal, data, info, ui);
                                            if let Some(pos) = response.interact_pointer_pos() {
                                                dragging_pos = Some(pos - vec2(wave_left, 0.0));
                                            }
                                            wave_left = ui.available_rect_before_wrap().left();
                                            pointer_state
                                                .handle_pointer_response(&response, wave_left);
                                        }
                                    });
                                }
                            },
                        );
                    });
                let response = ui.allocate_response(
                    ui.available_rect_before_wrap().size(),
                    Sense::click_and_drag(),
                );
                if let Some(pos) = response.interact_pointer_pos() {
                    dragging_pos = Some(pos - vec2(wave_left, 0.0));
                }
                pointer_state.handle_pointer_response(&response, wave_left);
            });
            let global_response = inner_response.response;
            let state = self.handle_response(
                ui,
                &global_response,
                wave_left,
                &wave.info,
                self.range.clone(),
            );
            // update signal information
            let signals_updated = self
                .signals
                .iter()
                .map(|x| x.clone())
                .enumerate()
                .map(|x| match new_signals.iter().find(|c| c.1 == x.0) {
                    None => Some(x.1),
                    Some(c) => match c.2 {
                        true => None,
                        false => Some(c.0.clone()),
                    },
                })
                .filter(|x| x.is_some())
                .map(|x| x.unwrap())
                .collect();
            self.signals = signals_updated;
            self.range = state.new_range;
            // info!("fix_width = {}, ui left = {}, wave_left = {}", fix_width, ui.max_rect().left(), wave_left);
            // info!("(fix_width + ui left) - wave_left = {}", fix_width + ui.max_rect().left() - wave_left);
            if let Some(pos) = dragging_pos {
                let painter = ui.painter();
                painter.text(
                    pos + vec2(wave_left, 0.0),
                    Align2::RIGHT_BOTTOM,
                    format!("{:?}", pos),
                    Default::default(),
                    Color32::YELLOW,
                );
                if pointer_state.drag_by_primary || pointer_state.drag_by_secondary {
                    let fpos = self.x_to_fpos(pos.x);
                    let p = if self.round_pointer {
                        fpos.round() as u64
                    } else {
                        fpos as u64
                    }
                    .clamp(self.range.0 as u64, self.range.1 as u64);
                    self.marker_temp.set_pos_valid(p);
                    if pointer_state.drag_by_secondary && !self.range_seek_started {
                        self.marker.set_pos_valid(p);
                        self.range_seek_started = true;
                    }
                }
                if pointer_state.move_drag_start_pos.is_some() && self.move_drag_start_pos.is_none()
                {
                    self.move_drag_start_pos = pointer_state.move_drag_start_pos;
                }
                if pointer_state.drag_release {
                    self.move_drag_start_pos = None;
                }
                if let Some(move_drag_start_pos) = self.move_drag_start_pos {
                    if let Some(move_drag_pos) = pointer_state.move_drag_pos {
                        let delta = move_drag_pos - move_drag_start_pos;
                        if let Some(move_drag_last_pos) = self.move_drag_last_pos {
                            // Handle drag move
                            let delta = move_drag_pos - move_drag_last_pos;
                            let dx = -delta.x;
                            self.range = self.move_horizontal(dx, info);
                        }
                        // natural direction
                        let dy = -delta.y;
                        // Handle right drag
                        if let Some(last_paint_row_index) = last_paint_row_index {
                            // simply use const
                            if dy < -SIGNAL_HEIGHT_DEFAULT {
                                let index = i64::max(last_paint_row_index as i64 - 2, 0) as usize;
                                debug!("to last signal: {}", index);
                                self.scrolling_next_index = Some(index);
                                self.move_drag_start_pos = Some(move_drag_pos);
                            }
                            if dy > SIGNAL_HEIGHT_DEFAULT {
                                let index =
                                    usize::min(last_paint_row_index, self.signals.len() - 1);
                                debug!("to next signal: {}", index);
                                self.scrolling_next_index = Some(index);
                                self.move_drag_start_pos = Some(move_drag_pos);
                            }
                        }
                    }
                }
                if pointer_state.move_drag_pos.is_some() {
                    self.move_drag_last_pos = pointer_state.move_drag_pos;
                }
                if pointer_state.drag_release {
                    self.move_drag_last_pos = None;
                }
                if pointer_state.drag_release && self.marker_temp.valid {
                    // scale to range
                    if self.last_pointer_state.drag_by_secondary {
                        let (a, b) = if self.marker.pos < self.marker_temp.pos {
                            (&self.marker, &self.marker_temp)
                        } else {
                            (&self.marker_temp, &self.marker)
                        };
                        let range_new = (a.pos as f32, b.pos as f32);
                        debug!("range_new: {:?}", range_new);
                        if range_new.1 - range_new.0 > 1.0 {
                            self.range = range_new;
                        }
                    }
                    self.marker.set_pos_valid(
                        self.marker_temp
                            .pos
                            .clamp(self.range.0 as u64, self.range.1 as u64),
                    );
                }
                if !pointer_state.drag_by_primary && !pointer_state.drag_by_secondary {
                    self.marker_temp.valid = false;
                }
                if pointer_state.drag_release {
                    self.range_seek_started = false;
                }
            }
            self.paint_span(
                ui,
                wave_left,
                info,
                dragging_pos,
                &self.marker,
                &self.marker_temp,
            );
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
            self.last_pointer_state = pointer_state;
        });
    }
    pub fn move_horizontal(&self, dx: f32, info: &WaveInfo) -> (f32, f32) {
        let pos_delta = self.x_to_fpos(dx) - self.range.0;
        let new_range_check = (self.range.0 + pos_delta, self.range.1 + pos_delta);
        if new_range_check.0 < 0.0 {
            if self.limit_range_left {
                (0.0, new_range_check.1 - new_range_check.0)
            } else {
                new_range_check
            }
        } else {
            let max_right = ZOOM_SIZE_MAX_SCALE * (info.range.1 - info.range.0) as f32;
            if new_range_check.1 > max_right {
                (
                    max_right - (new_range_check.1 - new_range_check.0),
                    max_right,
                )
            } else {
                new_range_check
            }
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
        let paint_rect = Rect::from_min_max(
            paint_rect.min
                + vec2(
                    0.0,
                    if self.use_top_margin {
                        WAVE_MARGIN_TOP + WAVE_MARGIN_TOP2
                    } else {
                        0.0
                    },
                ),
            paint_rect.max,
        );
        let painter = ui.painter();
        if a.valid && b.valid {
            let (a, b) = if a.pos < b.pos { (a, b) } else { (b, a) };
            let (x_a, x_b) = (
                self.pos_to_x(a.pos).clamp(0.0, self.wave_width) + offset,
                self.pos_to_x(b.pos).clamp(0.0, self.wave_width) + offset,
            );
            let rect = Rect::from_min_max(pos2(x_a, paint_rect.min.y), pos2(x_b, paint_rect.max.y));
            let color_bg = Color32::BLUE.linear_multiply(BG_MULTIPLY);
            // painter.rect(rect, 0.0, color_bg, (LINE_WIDTH, Color32::BLUE));
            painter.rect_filled(rect, 0.0, color_bg);
            let y = match pos {
                None => paint_rect.top(),
                Some(pos) => pos.y,
            };
            painter.hline(RangeInclusive::new(x_a, x_b), y, (LINE_WIDTH, color_bg));
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
