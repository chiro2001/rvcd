use crate::message::RvcdMsg;
use crate::radix::Radix;
use crate::wave::{WaveDataItem, WaveDataValue, WaveInfo, WaveSignalInfo, WireValue};
use eframe::emath::Align;
use egui::{pos2, vec2, Align2, Color32, Layout, Rect, ScrollArea, Sense, Ui};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::sync::mpsc;
use tracing::{debug, warn};

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone)]
pub enum SignalViewMode {
    Number(Radix),
    Analog,
}

impl Default for SignalViewMode {
    fn default() -> Self {
        Self::Number(Radix::Hex)
    }
}
#[derive(serde::Deserialize, serde::Serialize, PartialEq, Default, Debug)]
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

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(default)]
pub struct WaveView {
    pub signals: Vec<SignalView>,
    pub range: (u64, u64),
    pub align: SignalViewAlign,
    pub background: bool,
    #[serde(skip)]
    pub tx: Option<mpsc::Sender<RvcdMsg>>,
}

impl Default for WaveView {
    fn default() -> Self {
        Self {
            signals: vec![],
            range: (0, 0),
            align: Default::default(),
            background: true,
            tx: None,
        }
    }
}

impl WaveView {
    pub fn new(tx: mpsc::Sender<RvcdMsg>) -> Self {
        Self {
            tx: Some(tx),
            ..Default::default()
        }
    }
    pub fn set_tx(&mut self, tx: mpsc::Sender<RvcdMsg>) {
        self.tx = Some(tx);
    }
    pub fn signals_clean_unavailable(&mut self, info: &WaveInfo) {
        let signals: Vec<SignalView> = self
            .signals
            .clone()
            .into_iter()
            .filter(|signal| info.code_name_width.contains_key(&signal.s.id))
            .collect();
        self.signals = signals;
    }
    pub fn menu(&mut self, ui: &mut Ui) {
        ui.menu_button("View", |ui| {
            ui.menu_button(format!("Align: {:?}", self.align), |ui| {
                use SignalViewAlign::*;
                let data = [Left, Center, Right];
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
        });
    }
    pub fn toolbar(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            if ui.button("⛔ Clear").clicked() {
                self.signals.clear();
            }
            if ui.button("🔄 Refresh").clicked() {
                if let Some(tx) = &self.tx {
                    debug!("reload msg sent");
                    tx.send(RvcdMsg::Reload).unwrap();
                } else {
                    warn!("no tx in view!");
                }
            }
        });
    }
    pub fn panel(&mut self, ui: &mut Ui, info: &Option<WaveInfo>, wave_data: &[WaveDataItem]) {
        const LINE_WIDTH: f32 = 1.5;
        const MIN_TEXT_WIDTH: f32 = 6.0;
        if let Some(info) = info {
            if self.range.0 == 0 && self.range.1 == 0 {
                self.range = info.range;
            }
        }
        self.toolbar(ui);
        ScrollArea::vertical().show(ui, |ui| {
            egui::SidePanel::left("signals")
                .resizable(true)
                .show_inside(ui, |ui| {
                    if let Some(info) = info {
                        for signal in self.signals.iter() {
                            if let Some(_) = info.code_name_width.get(&signal.s.id) {
                                let text = signal.s.to_string();
                                ui.scope(|ui| {
                                    ui.set_height(signal.height);
                                    ui.centered_and_justified(|ui| {
                                        ui.add(egui::Label::new(text).wrap(false));
                                    });
                                });
                            }
                        }
                    }
                });
            egui::CentralPanel::default().show_inside(ui, |ui| {
                if let Some(info) = info {
                    for signal in self.signals.iter() {
                        ui.scope(|ui| {
                            ui.set_height(signal.height);
                            ui.centered_and_justified(|ui| {
                                let (response, painter) = ui.allocate_painter(
                                    ui.available_size_before_wrap(),
                                    Sense::hover(),
                                );
                                let items = wave_data.iter().filter(|i| i.id == signal.s.id);
                                let color = ui.visuals().strong_text_color();
                                let signal_rect = response.rect;
                                let mut it = items;
                                let mut item_last: Option<&WaveDataItem> = None;
                                let paint_signal =
                                    |item_now: &WaveDataItem, item_next: &WaveDataItem| {
                                        let single: bool = match &item_now.value {
                                            WaveDataValue::Comp(_) => {
                                                let d = ("".to_string(), 0);
                                                let (_v, w) = info
                                                    .code_name_width
                                                    .get(&signal.s.id)
                                                    .unwrap_or(&d);
                                                *w == 1
                                            }
                                            WaveDataValue::Raw(v) => v.len() == 1,
                                        };
                                        let width = signal_rect.width();
                                        let height = signal_rect.height();
                                        let percent_rect_left = (item_now.timestamp - info.range.0)
                                            as f32
                                            / (self.range.1 - self.range.0) as f32;
                                        let percent_rect_right =
                                            (item_next.timestamp - info.range.0) as f32
                                                / (self.range.1 - self.range.0) as f32;
                                        let percent_text =
                                            (((item_now.timestamp + item_next.timestamp) as f32
                                                / 2.0)
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
                                        let bg_multiply = 0.05;
                                        let paint_x = || {
                                            painter.rect(
                                                rect,
                                                0.0,
                                                if self.background {
                                                    Color32::DARK_RED.linear_multiply(bg_multiply)
                                                } else {
                                                    Color32::TRANSPARENT
                                                },
                                                (LINE_WIDTH, Color32::RED),
                                            )
                                        };
                                        let paint_z = || {
                                            painter.rect_stroke(
                                                rect,
                                                0.0,
                                                (LINE_WIDTH, Color32::DARK_RED),
                                            )
                                        };
                                        if single {
                                            let value = match &item_now.value {
                                                WaveDataValue::Comp(v) => {
                                                    match BigUint::from_bytes_le(v).is_one() {
                                                        true => WireValue::V1,
                                                        false => WireValue::V0,
                                                    }
                                                }
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
                                                    painter.hline(
                                                        rect.x_range(),
                                                        rect.top(),
                                                        (LINE_WIDTH, Color32::GREEN),
                                                    );
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
                                            let text = item_now.value.to_string();
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
                                                                    Color32::GREEN.linear_multiply(
                                                                        bg_multiply,
                                                                    )
                                                                } else {
                                                                    Color32::TRANSPARENT
                                                                },
                                                                (LINE_WIDTH, Color32::GREEN),
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            if rect.width() > MIN_TEXT_WIDTH {
                                                let pos = match self.align {
                                                    SignalViewAlign::Left => {
                                                        rect.left_center() + vec2(4.0, 0.0)
                                                    }
                                                    SignalViewAlign::Center => {
                                                        rect.left_center()
                                                            + vec2(width * percent_text, 0.0)
                                                    }
                                                    SignalViewAlign::Right => rect.right_center(),
                                                };
                                                painter.text(
                                                    pos,
                                                    match self.align {
                                                        SignalViewAlign::Left => {
                                                            Align2::LEFT_CENTER
                                                        }
                                                        SignalViewAlign::Center => {
                                                            Align2::CENTER_CENTER
                                                        }
                                                        SignalViewAlign::Right => {
                                                            Align2::RIGHT_CENTER
                                                        }
                                                    },
                                                    text,
                                                    Default::default(),
                                                    color,
                                                );
                                            }
                                        }
                                    };
                                while let Some(item) = it.next() {
                                    if let Some(item_last) = item_last {
                                        paint_signal(item_last, item);
                                    }
                                    item_last = Some(item);
                                }
                                // draw last
                                if let Some(item_last) = item_last {
                                    paint_signal(
                                        item_last,
                                        &WaveDataItem {
                                            timestamp: info.range.1,
                                            ..WaveDataItem::default()
                                        },
                                    );
                                }
                            });
                        });
                    }
                }
            });
        });
    }
    pub fn reset(&mut self) {
        self.range = (0, 0);
        self.signals.clear();
    }
}
