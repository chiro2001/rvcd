use crate::radix::Radix;
use crate::view::{WaveView, BG_MULTIPLY, LINE_WIDTH, MIN_SIGNAL_WIDTH, TEXT_ROUND_OFFSET};
use crate::wave::{WaveDataItem, WaveDataValue, WaveInfo, WaveSignalInfo, WireValue};
use egui::*;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::ops::RangeInclusive;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default)]
pub enum SignalViewMode {
    #[default]
    Default,
    Number(Radix),
    Analog,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Default, Debug, Clone)]
pub enum SignalViewAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq, Debug, Clone)]
pub struct SignalView {
    pub s: WaveSignalInfo,
    pub height: f32,
    pub mode: SignalViewMode,
}
pub const SIGNAL_HEIGHT_DEFAULT: f32 = 30.0;
impl SignalView {
    pub fn from_id(id: u64, info: &WaveInfo) -> Self {
        let d = ("unknown".to_string(), 0);
        let name_width = info.code_name_width.get(&id).unwrap_or(&d).clone();
        Self {
            s: WaveSignalInfo {
                id,
                name: name_width.0,
                width: name_width.1,
            },
            height: SIGNAL_HEIGHT_DEFAULT,
            mode: Default::default(),
        }
    }
}

impl WaveView {
    /// Paint a signal wave, return this response
    pub(crate) fn ui_signal_wave(
        &self,
        signal: &SignalView,
        wave_data: &[WaveDataItem],
        info: &WaveInfo,
        ui: &mut Ui,
    ) -> Response {
        let (response, painter) =
            ui.allocate_painter(ui.available_size_before_wrap(), Sense::click_and_drag());
        let items = wave_data.iter().filter(|i| i.id == signal.s.id);
        let text_color = ui.visuals().strong_text_color();
        let signal_rect = response.rect;
        let mut it = items;
        let mut item_last: Option<&WaveDataItem> = None;
        let mut ignore_x_start = -1.0;
        let mut ignore_has_x = false;
        let mut paint_signal = |item_now: &WaveDataItem, item_next: &WaveDataItem| {
            let single: bool = match &item_now.value {
                WaveDataValue::Comp(_) => {
                    let d = ("".to_string(), 0);
                    let (_v, w) = info.code_name_width.get(&signal.s.id).unwrap_or(&d);
                    *w == 1
                }
                WaveDataValue::Raw(v) => v.len() == 1,
            };
            let width = signal_rect.width();
            let height = signal_rect.height();
            let percent_rect_left =
                (item_now.timestamp - info.range.0) as f32 / (self.range.1 - self.range.0) as f32;
            let percent_rect_right =
                (item_next.timestamp - info.range.0) as f32 / (self.range.1 - self.range.0) as f32;
            let percent_text = (((item_now.timestamp + item_next.timestamp) as f32 / 2.0)
                - info.range.0 as f32)
                / (self.range.1 - self.range.0) as f32;
            let rect = Rect::from_min_max(
                pos2(
                    signal_rect.left() + width * percent_rect_left,
                    signal_rect.top(),
                ),
                pos2(
                    signal_rect.left() + width * percent_rect_right,
                    signal_rect.top() + height,
                ),
            );
            let radix = match &signal.mode {
                SignalViewMode::Default => self.default_radix.clone(),
                SignalViewMode::Number(r) => r.clone(),
                SignalViewMode::Analog => Radix::Hex,
            };
            let text = item_now.value.as_radix(radix);
            if rect.width() > MIN_SIGNAL_WIDTH {
                if ignore_x_start >= 0.0 {
                    // paint a rect as ignored data
                    painter.rect_filled(
                        Rect::from_x_y_ranges(
                            RangeInclusive::new(ignore_x_start, rect.left()),
                            rect.y_range(),
                        ),
                        0.0,
                        if ignore_has_x {
                            Color32::DARK_RED
                        } else {
                            Color32::GREEN
                        },
                    );
                    ignore_x_start = -1.0;
                    ignore_has_x = false;
                }
                let paint_x = || {
                    painter.rect(
                        rect,
                        0.0,
                        if self.background {
                            Color32::DARK_RED.linear_multiply(BG_MULTIPLY)
                        } else {
                            Color32::TRANSPARENT
                        },
                        (LINE_WIDTH, Color32::RED),
                    )
                };
                let paint_z = || painter.rect_stroke(rect, 0.0, (LINE_WIDTH, Color32::DARK_RED));
                if single {
                    let value = match &item_now.value {
                        WaveDataValue::Comp(v) => match BigUint::from_bytes_le(v).is_one() {
                            true => WireValue::V1,
                            false => WireValue::V0,
                        },
                        WaveDataValue::Raw(v) => v[0],
                    };
                    match value {
                        WireValue::V0 => {
                            painter.hline(
                                rect.x_range(),
                                rect.bottom(),
                                (LINE_WIDTH, Color32::GREEN),
                            );
                            painter.vline(
                                rect.left(),
                                rect.y_range(),
                                (LINE_WIDTH, Color32::GREEN),
                            );
                        }
                        WireValue::V1 => {
                            painter.hline(rect.x_range(), rect.top(), (LINE_WIDTH, Color32::GREEN));
                            painter.vline(
                                rect.left(),
                                rect.y_range(),
                                (LINE_WIDTH, Color32::GREEN),
                            );
                        }
                        WireValue::X => paint_x(),
                        WireValue::Z => paint_z(),
                    };
                } else {
                    let number: Option<BigUint> = (&item_now.value).into();
                    if text.contains('x') {
                        paint_x();
                    } else {
                        if text.contains('z') {
                            paint_z();
                        } else {
                            match number {
                                Some(n) if n.is_zero() => {
                                    painter.hline(
                                        rect.x_range(),
                                        rect.bottom(),
                                        (LINE_WIDTH, Color32::GREEN),
                                    );
                                }
                                _ => {
                                    painter.rect(
                                        rect,
                                        0.0,
                                        if self.background {
                                            Color32::GREEN.linear_multiply(BG_MULTIPLY)
                                        } else {
                                            Color32::TRANSPARENT
                                        },
                                        (LINE_WIDTH, Color32::GREEN),
                                    );
                                }
                            }
                        }
                    }
                    if self.show_text {
                        let text_min_rect = painter.text(
                            Pos2::ZERO,
                            Align2::RIGHT_BOTTOM,
                            "+",
                            FontId::monospace(self.signal_font_size),
                            Color32::TRANSPARENT,
                        );
                        if rect.width() >= text_min_rect.width() + TEXT_ROUND_OFFSET {
                            let pos = match self.align {
                                SignalViewAlign::Left => {
                                    rect.left_center() + vec2(TEXT_ROUND_OFFSET, 0.0)
                                }
                                SignalViewAlign::Center => {
                                    rect.left_center() + vec2(width * percent_text, 0.0)
                                }
                                SignalViewAlign::Right => {
                                    rect.right_center() - vec2(TEXT_ROUND_OFFSET, 0.0)
                                }
                            };
                            // pre-paint to calculate size
                            let text_rect = painter.text(
                                pos,
                                match self.align {
                                    SignalViewAlign::Left => Align2::LEFT_CENTER,
                                    SignalViewAlign::Center => Align2::CENTER_CENTER,
                                    SignalViewAlign::Right => Align2::RIGHT_CENTER,
                                },
                                text.as_str(),
                                FontId::monospace(self.signal_font_size),
                                Color32::TRANSPARENT,
                            );
                            let paint_text =
                                if rect.width() >= text_rect.width() + TEXT_ROUND_OFFSET {
                                    text
                                } else {
                                    let text_mono_width = text_rect.width() / text.len() as f32;
                                    let text_len = text.len();
                                    let remains = &text[0..(text_len
                                        - ((text_rect.width() + TEXT_ROUND_OFFSET - rect.width())
                                            / text_mono_width)
                                            as usize)];
                                    if remains.len() <= 1 {
                                        "+".to_string()
                                    } else {
                                        let len = remains.len();
                                        format!("{}+", &remains[0..(len - 2)])
                                    }
                                };
                            painter.text(
                                pos,
                                match self.align {
                                    SignalViewAlign::Left => Align2::LEFT_CENTER,
                                    SignalViewAlign::Center => Align2::CENTER_CENTER,
                                    SignalViewAlign::Right => Align2::RIGHT_CENTER,
                                },
                                paint_text,
                                // Default::default(),
                                FontId::monospace(self.signal_font_size),
                                text_color,
                            );
                        }
                    }
                }
            } else {
                // ignore this paint, record start pos
                if ignore_x_start < 0.0 {
                    ignore_x_start = rect.left();
                }
                if text.contains('x') || text.contains('z') {
                    ignore_has_x = true;
                }
            }
        };
        while let Some(item) = it.next() {
            if let Some(item_last) = item_last {
                paint_signal(item_last, item);
            }
            item_last = Some(item);
        }
        if let Some(item_last) = item_last {
            paint_signal(
                item_last,
                &WaveDataItem {
                    timestamp: u64::min(info.range.1, self.range.1),
                    ..WaveDataItem::default()
                },
            );
        }
        // draw last
        if ignore_x_start >= 0.0 {
            let right_pos = self
                .x_to_pos(signal_rect.right())
                .clamp(0, u64::min(self.range.1, info.range.1));
            painter.rect_filled(
                Rect::from_x_y_ranges(
                    RangeInclusive::new(ignore_x_start, self.pos_to_x(right_pos)),
                    signal_rect.y_range(),
                ),
                0.0,
                if ignore_has_x {
                    Color32::DARK_RED
                } else {
                    Color32::GREEN
                },
            )
        }
        response
    }
    /// Paint signal label
    pub(crate) fn ui_signal_label(&self, signal: &SignalView, ui: &mut Ui) {
        let text = signal.s.to_string();
        ui.scope(|ui| {
            ui.set_height(signal.height);
            ui.centered_and_justified(|ui| {
                ui.add(Label::new(text).wrap(false));
            });
        });
    }
}
