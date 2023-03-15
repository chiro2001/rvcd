use crate::message::RvcdMsg;
use crate::radix::Radix;
use crate::utils::{execute, get_text_size};
use crate::verilog::VerilogGotoSource;
use crate::view::{
    WaveView, BG_MULTIPLY, LINE_WIDTH, MIN_SIGNAL_WIDTH, SIGNAL_HEIGHT_DEFAULT, TEXT_ROUND_OFFSET,
};
use crate::wave::{WaveDataItem, WaveDataValue, WaveInfo, WaveSignalInfo, WireValue};
use num_bigint::BigUint;
use num_traits::{One, ToPrimitive, Zero};
use once_cell::sync::Lazy;
use std::fmt::{Display, Formatter};
use std::ops::RangeInclusive;
use egui::{Align, Align2, Color32, color_picker, DragValue, FontId, Label, Layout, pos2, Rect, Response, Sense, Ui, vec2, Widget};
use tracing::info;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default)]
pub enum AnalogDisplayType {
    #[default]
    Interpolated,
    Step,
}

impl Display for AnalogDisplayType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default)]
pub enum SignalViewMode {
    #[default]
    Default,
    Number(Radix),
    Analog(AnalogDisplayType),
}
impl Display for SignalViewMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalViewMode::Default => write!(f, "default"),
            SignalViewMode::Number(r) => write!(f, "{r}"),
            SignalViewMode::Analog(r) => write!(f, "Analog({:?})", r),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Default, Debug, Clone)]
pub enum SignalViewAlign {
    #[default]
    Left,
    Center,
    Right,
}

static SIGNAL_COLOR_BG_DEFAULT: Lazy<Color32> =
    // Lazy::new(|| Color32::GREEN.linear_multiply(BG_MULTIPLY));
    Lazy::new(|| Color32::GREEN);

#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq, Debug, Clone)]
pub struct SignalView {
    pub s: WaveSignalInfo,
    pub height: f32,
    pub mode: SignalViewMode,
    pub color: Color32,
}
impl SignalView {
    pub fn from_id(id: u64, info: &WaveInfo) -> Self {
        let d = WaveSignalInfo::default();
        let signal_info = info.code_signal_info.get(&id).unwrap_or(&d).clone();
        Self {
            s: signal_info,
            height: SIGNAL_HEIGHT_DEFAULT,
            mode: Default::default(),
            color: SIGNAL_COLOR_BG_DEFAULT.clone(),
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
        // let items = wave_data.iter().filter(|i| i.id == signal.s.id);
        let items = wave_data.iter();
        // let start_pos = self.range.0.ceil() as u64;
        let start_pos = self.range.0 as u64;
        let start_index = wave_data.binary_search_by_key(&start_pos, |x| x.timestamp);
        let start_items = match start_index {
            Ok(index) | Err(index) => {
                if index > 0 {
                    items.skip(index - 1)
                } else {
                    items.skip(0)
                }
            }
        };
        let text_color = ui.visuals().strong_text_color();
        let signal_rect_raw = response.rect;
        // strange but works...
        let wave_range_start_x = self.fpos_to_x(self.range.0 * 2.0);
        let signal_rect = Rect::from_min_max(
            signal_rect_raw.min - vec2(wave_range_start_x, 0.0),
            signal_rect_raw.max - vec2(wave_range_start_x, 0.0),
        );
        // painter.vline(
        //     signal_rect.left(),
        //     response.rect.y_range(),
        //     (LINE_WIDTH, Color32::WHITE),
        // );
        // painter.rect_stroke(signal_rect, 3.0, (LINE_WIDTH * 2.0, Color32::WHITE));
        // painter.rect_filled(
        //     Rect::from_min_max(
        //         signal_rect.min + vec2(wave_range_start_x, 0.0),
        //         signal_rect.max + vec2(wave_range_start_x, 0.0),
        //     ),
        //     3.0,
        //     Color32::WHITE,
        // );

        // painter.vline(
        //     self.pos_to_x(start_pos) + signal_rect.left() + wave_range_start_x,
        //     response.rect.y_range(),
        //     (LINE_WIDTH, Color32::RED),
        // );
        let mut item_last: Option<&WaveDataItem> = None;
        let mut ignore_x_start = -1.0;
        let mut ignore_has_x = false;
        let mut paint_signal = |item_now: &WaveDataItem, item_next: &WaveDataItem| {
            let single: bool = match &item_now.value {
                WaveDataValue::Comp(_) => {
                    let d = Default::default();
                    let s = info.code_signal_info.get(&signal.s.id).unwrap_or(&d);
                    s.width == 1
                }
                WaveDataValue::Raw(v) => v.len() == 1,
            };
            let width = signal_rect.width();
            let height = signal_rect.height();
            let percent_rect_left =
                (item_now.timestamp - info.range.0) as f32 / (self.range.1 - self.range.0);
            let percent_rect_right =
                (item_next.timestamp - info.range.0) as f32 / (self.range.1 - self.range.0);
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
            // painter.rect_filled(rect, 3.0, Color32::GRAY);
            if !ui.is_rect_visible(rect) {
                return Rect::NOTHING;
            }
            match signal.mode {
                SignalViewMode::Analog(_) => {
                    return rect;
                }
                _ => {}
            };
            let text = item_now.value.as_radix(self.get_radix(signal));
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
                            signal.color.clone()
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
                                (LINE_WIDTH, signal.color.clone()),
                            );
                            painter.vline(
                                rect.left(),
                                rect.y_range(),
                                (LINE_WIDTH, signal.color.clone()),
                            );
                        }
                        WireValue::V1 => {
                            painter.hline(
                                rect.x_range(),
                                rect.top(),
                                (LINE_WIDTH, signal.color.clone()),
                            );
                            painter.vline(
                                rect.left(),
                                rect.y_range(),
                                (LINE_WIDTH, signal.color.clone()),
                            );
                        }
                        WireValue::X => paint_x(),
                        WireValue::Z => paint_z(),
                    };
                } else {
                    let number: Option<BigUint> = (&item_now.value).into();
                    if text.contains('x') {
                        paint_x();
                    } else if text.contains('z') {
                        paint_z();
                    } else {
                        match number {
                            Some(n) if n.is_zero() => {
                                painter.hline(
                                    rect.x_range(),
                                    rect.bottom(),
                                    (LINE_WIDTH, signal.color.clone()),
                                );
                            }
                            _ => {
                                painter.rect(
                                    rect,
                                    0.0,
                                    if self.background {
                                        signal.color.linear_multiply(BG_MULTIPLY)
                                    } else {
                                        Color32::TRANSPARENT
                                    },
                                    (LINE_WIDTH, signal.color.clone()),
                                );
                            }
                        }
                    }
                    let value_font = FontId::monospace(self.signal_font_size);
                    let text_min_size = get_text_size(ui, "+", value_font.clone());
                    if self.show_text && rect.width() >= text_min_size.x + TEXT_ROUND_OFFSET {
                        let pos = match self.align {
                            SignalViewAlign::Left => {
                                rect.left_center() + vec2(TEXT_ROUND_OFFSET, 0.0)
                            }
                            SignalViewAlign::Center => rect.center(),
                            SignalViewAlign::Right => {
                                rect.right_center() - vec2(TEXT_ROUND_OFFSET, 0.0)
                            }
                        };
                        // pre-paint to calculate size
                        let text_size = get_text_size(ui, text.as_str(), value_font.clone());
                        let paint_text = if rect.width() >= text_size.x + TEXT_ROUND_OFFSET {
                            text
                        } else {
                            let text_mono_width = text_size.x / text.len() as f32;
                            let text_len = text.len();
                            let remains = &text[0..(text_len
                                - ((text_size.x + TEXT_ROUND_OFFSET - rect.width())
                                    / text_mono_width) as usize)];
                            if remains.len() <= 1 {
                                "+".to_string()
                            } else {
                                let len = remains.len();
                                format!("{}+", &remains[0..(len - 2)])
                            }
                        };
                        // let text_font = FontId::monospace(self.signal_font_size);
                        // TODO: limit text position
                        // let text_rect = painter.text(
                        //     Pos2::ZERO,
                        //     Align2::RIGHT_BOTTOM,
                        //     paint_text.as_str(),
                        //     text_font.clone(),
                        //     Color32::TRANSPARENT,
                        // );
                        // let pos = pos2(
                        //     pos.x.clamp(
                        //         rect.left(),
                        //         rect.right() - text_rect.width(),
                        //     ),
                        //     pos.y,
                        // );
                        painter.text(
                            pos,
                            match self.align {
                                SignalViewAlign::Left => Align2::LEFT_CENTER,
                                SignalViewAlign::Center => Align2::CENTER_CENTER,
                                SignalViewAlign::Right => Align2::RIGHT_CENTER,
                            },
                            paint_text,
                            value_font,
                            text_color,
                        );
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
            rect
        };
        // TODO: Reduce horizontal value painting
        // let mut done_early = false;
        let mut paint_items = vec![];
        let is_analog = match signal.mode {
            SignalViewMode::Analog(_) => true,
            _ => false,
        };
        for item in start_items {
            // let mut done = false;
            if is_analog {
                paint_items.push(item);
            }
            if let Some(item_last) = item_last {
                let value_rect = paint_signal(item_last, item);
                if value_rect == Rect::NOTHING || value_rect.left() > response.rect.right() {
                    break;
                }
                // if !ui.is_rect_visible(_value_rect) {
                //     painter.rect_filled(_value_rect, 0.0, Color32::RED);
                //     done = true;
                //     done_early = true;
                // }
            }
            item_last = Some(item);
            // if done {
            //     break;
            // }
        }
        // if done_early {
        //     let mut paint_it = || {
        //         if let Some(item) = it.next() {
        //             let r = if let Some(item_last) = item_last {
        //                 let _ = paint_signal(item_last, item);
        //                 true
        //             } else {
        //                 false
        //             };
        //             item_last = Some(item);
        //             r
        //         } else {
        //             false
        //         }
        //     };
        //     paint_it();
        //     paint_it();
        //     paint_it();
        // } else {
        if let Some(item_last) = item_last {
            let _ = paint_signal(
                item_last,
                &WaveDataItem {
                    timestamp: u64::min(info.range.1 + 1, self.range.1 as u64 + 1),
                    ..WaveDataItem::default()
                },
            );
        }
        // }
        // draw last
        if ignore_x_start >= 0.0 {
            let right_pos = (self.x_to_pos(signal_rect.right()) + 1)
                .clamp(0, u64::min(self.range.1 as u64, info.range.1 + 1));
            painter.rect_filled(
                Rect::from_x_y_ranges(
                    RangeInclusive::new(ignore_x_start, self.pos_to_x(right_pos)),
                    signal_rect.y_range(),
                ),
                0.0,
                if ignore_has_x {
                    Color32::DARK_RED
                } else {
                    signal.color.clone()
                },
            )
        }
        if is_analog {
            // draw analog
            let mut item_last_analog: Option<&WaveDataItem> = None;
            let mut min_value = None;
            let mut max_value = None;
            let mut analog_no_value = true;
            for item in &paint_items {
                match &item.value {
                    WaveDataValue::Comp(v) => {
                        analog_no_value = false;
                        let value = BigUint::from_bytes_le(&v);
                        let handle = |min_max_value: Option<Vec<u8>>, is_less: bool| {
                            if min_max_value.is_none() {
                                Some(v.clone())
                            } else {
                                let min_max_v =
                                    BigUint::from_bytes_le(&min_max_value.as_ref().unwrap());
                                if if is_less {
                                    value < min_max_v
                                } else {
                                    value > min_max_v
                                } {
                                    Some(v.clone())
                                } else {
                                    min_max_value
                                }
                            }
                        };
                        min_value = handle(min_value, true);
                        max_value = handle(max_value, false);
                    }
                    WaveDataValue::Raw(_) => {}
                }
            }
            if analog_no_value {
                painter.rect(
                    Rect::from_x_y_ranges(
                        RangeInclusive::new(
                            self.fpos_to_x(self.range.0),
                            self.fpos_to_x(self.range.1),
                        ),
                        signal_rect_raw.y_range(),
                    ),
                    0.0,
                    Color32::RED.linear_multiply(BG_MULTIPLY),
                    (LINE_WIDTH, Color32::RED),
                );
            } else {
                let paint_analog = |item_now: &WaveDataItem, item_next: &WaveDataItem| {
                    let width = signal_rect.width();
                    let height = signal_rect.height();
                    let percent_rect_left =
                        (item_now.timestamp - info.range.0) as f32 / (self.range.1 - self.range.0);
                    let percent_rect_right =
                        (item_next.timestamp - info.range.0) as f32 / (self.range.1 - self.range.0);
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
                    if !(min_value.is_some() && max_value.is_some()) {
                        painter.rect(
                            rect,
                            0.0,
                            Color32::RED.linear_multiply(BG_MULTIPLY),
                            (LINE_WIDTH, Color32::RED),
                        );
                    } else {
                        let min_value = BigUint::from_bytes_le(min_value.as_ref().unwrap());
                        let max_value = BigUint::from_bytes_le(max_value.as_ref().unwrap());
                        let value_now = match &item_now.value {
                            WaveDataValue::Comp(v) => BigUint::from_bytes_le(v),
                            WaveDataValue::Raw(_) => BigUint::zero(),
                        };
                        let value_next = match &item_next.value {
                            WaveDataValue::Comp(v) => BigUint::from_bytes_le(v),
                            WaveDataValue::Raw(_) => BigUint::zero(),
                        };
                        let rate = (rect.bottom() - rect.top())
                            / (max_value.clone() - min_value.clone()).to_f32().unwrap();
                        let y_now =
                            rate * (max_value.clone() - value_now).to_f32().unwrap() + rect.top();
                        let y_next =
                            rate * (max_value.clone() - value_next).to_f32().unwrap() + rect.top();
                        let step = match &signal.mode {
                            SignalViewMode::Analog(i) => match i {
                                AnalogDisplayType::Interpolated => false,
                                AnalogDisplayType::Step => true,
                            },
                            _ => false,
                        };
                        let y_right = if step { y_now } else { y_next };
                        painter.line_segment(
                            [pos2(rect.left(), y_now), pos2(rect.right(), y_right)],
                            (LINE_WIDTH, signal.color.clone()),
                        );
                        if step {
                            painter.vline(
                                rect.right(),
                                RangeInclusive::new(y_now, y_next),
                                (LINE_WIDTH, signal.color.clone()),
                            );
                        }
                    }
                };
                for item in paint_items {
                    if let Some(item_now) = item_last_analog {
                        let item_next = item;
                        paint_analog(item_now, item_next);
                    }
                    item_last_analog = Some(item)
                }
                // last analog value
                if let Some(item_last_analog) = item_last_analog {
                    paint_analog(
                        item_last_analog,
                        &WaveDataItem {
                            timestamp: u64::min(info.range.1 + 1, self.range.1 as u64 + 1),
                            ..WaveDataItem::default()
                        },
                    );
                }
            }
        }
        response
    }
    /// Paint signal label
    pub(crate) fn ui_signal_label(
        &self,
        signal: &SignalView,
        index: usize,
        ui: &mut Ui,
        info: &WaveInfo,
        highlight: bool,
    ) -> Option<(SignalView, usize, bool)> {
        let mut signal_new = signal.clone();
        let text = signal.s.to_string();
        let mut to_remove = false;
        ui.scope(|ui| {
            ui.set_height(signal.height);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if highlight {
                    ui.painter().rect_filled(
                        ui.available_rect_before_wrap(),
                        0.0,
                        Color32::YELLOW.linear_multiply(BG_MULTIPLY),
                    );
                }
                let response = ui.add(Label::new(text).wrap(false).sense(Sense::click_and_drag()));
                // TODO: drag signal order
                response.context_menu(|ui| {
                    if ui.button(t!("view.remove")).clicked() {
                        to_remove = true;
                        ui.close_menu();
                    }
                    ui.horizontal(|ui| {
                        ui.label(t!("view.height"));
                        DragValue::new(&mut signal_new.height)
                            .clamp_range(
                                (SIGNAL_HEIGHT_DEFAULT / 2.0)..=(SIGNAL_HEIGHT_DEFAULT * 4.0),
                            )
                            .speed(1.0)
                            .suffix("px")
                            .ui(ui);
                    });
                    ui.menu_button(t!("view.color"), |ui| {
                        color_picker::color_picker_color32(
                            ui,
                            &mut signal_new.color,
                            color_picker::Alpha::Opaque,
                        );
                    });
                    ui.menu_button(t!("view.mode.prefix", mode = signal.mode.to_string().as_str()), |ui| {
                        if ui.button(t!("view.default")).clicked() {
                            signal_new.mode = SignalViewMode::Default;
                            ui.close_menu();
                        }
                        ui.menu_button(t!("view.number"), |ui| {
                            use Radix::*;
                            let data = [Hex, Oct, Dec, Bin];
                            data.into_iter().for_each(|r| {
                                if ui.button(format!("{r:?}")).clicked() {
                                    signal_new.mode = SignalViewMode::Number(r);
                                    ui.close_menu();
                                }
                            });
                        });
                        ui.menu_button(t!("view.analog"), |ui| {
                            let v = [AnalogDisplayType::Interpolated, AnalogDisplayType::Step];
                            for i in v {
                                if ui.button(i.to_string()).clicked() {
                                    signal_new.mode = SignalViewMode::Analog(i);
                                    ui.close_menu();
                                }
                            }
                        });
                    });
                    if !self.sources.is_empty() {
                        if ui.button(t!("view.to_source")).clicked() {
                            let id = signal.s.id;
                            if let Some(path) = info.code_paths.get(&id) {
                                let mut path = path.clone();
                                path.push(signal.s.name.to_string());
                                self.do_source_goto(path);
                            }
                            ui.close_menu();
                        }
                    }
                });
            });
        });
        if to_remove || signal_new != *signal {
            Some((signal_new, index, to_remove))
        } else {
            None
        }
    }
    pub fn get_radix(&self, signal: &SignalView) -> Radix {
        match &signal.mode {
            SignalViewMode::Default => self.default_radix.clone(),
            SignalViewMode::Number(r) => r.clone(),
            SignalViewMode::Analog(_) => Radix::Hex,
        }
    }
    pub fn do_source_goto(&self, path: Vec<String>) {
        let tx = self.tx.clone();
        if let Some(tx) = tx {
            let sources = self.sources.clone();
            execute(async move {
                info!("got path: {:?}", path);
                let mut results = vec![];
                for source in &sources {
                    info!("searching: {:#?}", source);
                    let result = source.search_path(&path);
                    let result = result
                        .into_iter()
                        .map(|x| (debug_ignore::DebugIgnore(source), x))
                        .collect::<Vec<_>>();
                    results.extend_from_slice(&result);
                }
                info!("got search result: {:?}", results);
                // select longest match
                results.sort_by_key(|x| x.1 .0.len());
                results.reverse();
                let mut results = results
                    .into_iter()
                    .map(|result| VerilogGotoSource {
                        file: result.0.source_path.clone(),
                        path: result.1 .0,
                        location: result.1 .1,
                    })
                    .collect::<Vec<_>>();
                if !results.is_empty() {
                    if results.len() == 1 {
                        let result = results.pop().unwrap();
                        tx.send(RvcdMsg::CallGotoSources(result)).unwrap();
                    } else {
                        tx.send(RvcdMsg::SetAlternativeGotoSources(results))
                            .unwrap();
                    }
                } else {
                    tx.send(RvcdMsg::GotNoSource).unwrap();
                }
            });
        }
    }
    pub fn do_signal_goto(&self, path: Vec<String>, info: &WaveInfo) {
        let tx = self.tx.clone();
        if let Some(tx) = tx {
            let paths: Vec<(u64, Vec<String>)> =
                info.code_paths.clone().into_iter().collect::<Vec<_>>();
            let match_num = |a: &Vec<String>, b: &Vec<String>| {
                let mut a = a.iter().rev();
                let mut b = b.iter().rev();
                let mut n = 0usize;
                loop {
                    let v1 = a.next();
                    let v2 = b.next();
                    if v1.is_some() && v2.is_some() && v1.unwrap().as_str() == v2.unwrap().as_str()
                    {
                        n += 1;
                    } else {
                        break;
                    }
                }
                n
            };
            execute(async move {
                let mut matches = paths
                    .iter()
                    .map(|x| (x.0, &x.1, match_num(&x.1, &path)))
                    .filter(|x| x.2 > 0)
                    .collect::<Vec<_>>();
                matches.sort_by_key(|x| x.2);
                matches.reverse();
                info!("matches: {:?}", matches);
                tx.send(RvcdMsg::SetGotoSignals(
                    matches.into_iter().map(|x| x.0).collect::<Vec<_>>(),
                ))
                .unwrap();
            });
        }
    }
}
