use crate::view::cursor::WaveCursor;
use crate::view::{WaveView, BG_MULTIPLY, LINE_WIDTH};
use crate::wave::WaveInfo;
use egui::*;
use std::ops::RangeInclusive;

impl WaveView {
    /// Paint time bar above the wave panel
    /// * `offset`: painting rect left
    pub fn time_bar(&mut self, ui: &mut Ui, info: &WaveInfo, offset: f32) {
        let rect = ui.max_rect();
        let (response, painter) = ui.allocate_painter(rect.size(), Sense::click_and_drag());
        let pos = response.interact_pointer_pos();
        let pos_new = pos.map(|pos| {
            self.x_to_pos(pos.x - offset)
                .clamp(self.range.0 as u64, self.range.1 as u64)
        });
        // allocate size for text
        let text_rect = painter.text(
            Pos2::ZERO,
            Align2::RIGHT_BOTTOM,
            "0",
            Default::default(),
            Color32::TRANSPARENT,
        );
        let line_stroke = (LINE_WIDTH, Color32::GREEN.linear_multiply(BG_MULTIPLY));
        painter.hline(rect.x_range(), rect.min.y + text_rect.height(), line_stroke);
        let mut step: u64 = (self.range.1 - self.range.0) as u64 / 10;
        while step as f32 * rect.width() / (self.range.1 - self.range.0) as f32 > 80.0 {
            step /= 10;
        }
        if step == 0 {
            step = 1;
        }
        let range = (self.range.0 as u64 / step * step)..(((self.range.1 as u64 / step) + 1) * step);
        // paint time stamp labels
        for pos in range.step_by(step as usize) {
            let time = info.timescale.0 * pos;
            let line_height_max = rect.height() - text_rect.height();
            let line_height = match time {
                time if time % (10 * step) == 0 => line_height_max,
                time if time % (5 * step) == 0 => line_height_max / 2.0,
                _ => line_height_max / 4.0,
            };
            let x = self.pos_to_x(pos) + offset;
            painter.vline(
                x,
                RangeInclusive::new(
                    rect.top() + text_rect.height(),
                    rect.top() + text_rect.height() + line_height,
                ),
                line_stroke,
            );
            match time {
                time if time % (5 * step) == 0 => {
                    // let time_text = self.pos_to_time_fmt(&info.timescale, pos);
                    let time_text = self.pos_to_time(&info.timescale, pos);
                    painter.text(
                        pos2(x, rect.top()),
                        Align2::LEFT_TOP,
                        time_text,
                        Default::default(),
                        ui.visuals().text_color(),
                    );
                }
                _ => {}
            };
        }
        let mut cursor_id: Option<i32> = None;
        if let Some(pos) = pos {
            cursor_id = self.find_cursor(pos.x - offset);
        }
        // handle operations to cursors
        // primary drag cursors
        if response.drag_released() {
            self.dragging_cursor_id = None;
        } else {
            if response.dragged_by(PointerButton::Primary) {
                if let Some(id) = cursor_id {
                    let cursor = match id {
                        -1 => Some(&mut self.marker),
                        // -2 => &mut self.marker_temp,
                        id => self.cursors.iter_mut().find(|x| x.id == id),
                    };
                    if let Some(cursor) = cursor {
                        self.dragging_cursor_id = Some(cursor.id);
                        // cursor.pos = (cursor.pos as i64 + delta_pos) as u64;
                        cursor.pos = pos_new.unwrap();
                    }
                }
            }
        }
        if response.clicked_by(PointerButton::Secondary)
            || response.dragged_by(PointerButton::Secondary)
        {
            self.right_click_pos = pos;
        }
        // pop up cursor menu
        response.context_menu(|ui| {
            ui.add_enabled_ui(self.right_click_pos.is_some(), |ui| {
                if ui.button("Add cursor").clicked() {
                    let pos_new = self.x_to_pos(self.right_click_pos.unwrap().x - offset);
                    self.cursors
                        .push(WaveCursor::new(self.next_cursor_id(), pos_new));
                    ui.close_menu();
                }
            });
            if let Some(right_click_pos) = self.right_click_pos {
                if let Some(id) = self.find_cursor(right_click_pos.x - offset) {
                    if ui.button("Remove cursor").clicked() {
                        if let Some(index) = self.cursors.iter().position(|x| x.id == id) {
                            self.cursors.remove(index);
                        }
                        ui.close_menu();
                    }
                    let mut span_to_add = None;
                    if let Some(cursor) = self.cursors.iter_mut().find(|x| x.id == id) {
                        let cursor_id = cursor.id;
                        let cursor_name = cursor.name.to_string();
                        if cursor.valid {
                            if ui.button("Disable cursor").clicked() {
                                cursor.valid = false;
                                ui.close_menu();
                            }
                        } else {
                            if ui.button("Enable cursor").clicked() {
                                cursor.valid = true;
                                ui.close_menu();
                            }
                        }
                        let mut linked_cursors = vec![];
                        ui.menu_button("Spans", |ui| {
                            let mut span_to_remove = None;
                            for span in self
                                .spans
                                .iter()
                                .filter(|x| x.0 == cursor_id || x.1 == cursor_id)
                            {
                                if self.cursors_exists_id(span.0) && self.cursors_exists_id(span.1)
                                {
                                    if let Some(a) = self.cursors_get(span.0) {
                                        if let Some(b) = self.cursors_get(span.1) {
                                            let mut linked = true;
                                            if a.id == cursor_id {
                                                linked_cursors.push(b.id);
                                            } else {
                                                linked_cursors.push(a.id);
                                            }
                                            if ui
                                                .checkbox(
                                                    &mut linked,
                                                    if a.id == cursor_id {
                                                        format!("{}-{}", a.name, b.name)
                                                    } else {
                                                        format!("{}-{}", b.name, a.name)
                                                    },
                                                )
                                                .clicked()
                                            {
                                                // remove this span
                                                span_to_remove = Some(span.clone());
                                                ui.close_menu();
                                            };
                                        }
                                    }
                                }
                            }
                            let mut cursors_unlinked = self
                                .cursors
                                .iter()
                                .filter(|x| x.id != cursor_id && !linked_cursors.contains(&x.id))
                                .collect::<Vec<_>>();
                            cursors_unlinked.sort();
                            for a in cursors_unlinked {
                                let mut linked = false;
                                if ui
                                    .checkbox(&mut linked, format!("{}-{}", cursor_name, a.name))
                                    .clicked()
                                {
                                    span_to_add = Some((cursor_id, a.id));
                                    ui.close_menu();
                                }
                            }
                            if let Some(span) = span_to_remove {
                                let spans_new = self
                                    .spans
                                    .iter()
                                    .map(|x| x.clone())
                                    .filter(|x| *x != span)
                                    .collect();
                                self.spans = spans_new;
                            }
                            if ui.button("Remove all relative spans").clicked() {
                                let spans_new = self
                                    .spans
                                    .iter()
                                    .filter(|x| x.0 != cursor_id && x.1 != cursor_id)
                                    .map(|x| x.clone())
                                    .collect();
                                self.spans = spans_new;
                                ui.close_menu();
                            }
                        });
                    }
                    if let Some(span) = span_to_add {
                        self.spans.push(span);
                    }
                }
            }
            if ui.button("Remove all cursor").clicked() {
                self.cursors.clear();
                self.marker.valid = false;
                self.marker_temp.valid = false;
                ui.close_menu();
            }
        });
    }
}
