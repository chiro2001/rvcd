pub mod cursor;
pub mod signal;
pub mod ui;

use egui::*;
use crate::message::RvcdMsg;
use crate::radix::Radix;
use crate::view::cursor::WaveCursor;
use std::sync::mpsc;
use tracing::*;
use crate::view::signal::{SignalView, SignalViewAlign};
use crate::wave::{WaveInfo, WaveTimescaleUnit};

const LINE_WIDTH: f32 = 1.5;
const TEXT_ROUND_OFFSET: f32 = 4.0;
const MIN_SIGNAL_WIDTH: f32 = 2.0;
const BG_MULTIPLY: f32 = 0.05;
const TEXT_BG_MULTIPLY: f32 = 0.4;
const CURSOR_NEAREST: f32 = 20.0;
const UI_WIDTH_OFFSET: f32 = 8.0;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(default)]
pub struct WaveView {
    pub signals: Vec<SignalView>,
    pub range: (u64, u64),
    pub align: SignalViewAlign,
    pub background: bool,
    pub show_text: bool,
    pub default_radix: Radix,
    #[serde(skip)]
    pub tx: Option<mpsc::Sender<RvcdMsg>>,
    pub cursors: Vec<WaveCursor>,
    pub marker: WaveCursor,
    pub marker_temp: WaveCursor,
    #[serde(skip)]
    pub dragging_cursor_id: Option<i32>,
    #[serde(skip)]
    pub wave_width: f32,
    pub signal_font_size: f32,
    #[serde(skip)]
    pub right_click_pos: Option<Pos2>,
}

impl Default for WaveView {
    fn default() -> Self {
        Self {
            signals: vec![],
            range: (0, 0),
            align: Default::default(),
            background: true,
            show_text: true,
            default_radix: Radix::Hex,
            tx: None,
            cursors: vec![],
            marker: WaveCursor::from_string(-1, "Main Cursor"),
            marker_temp: WaveCursor::from_string(-2, ""),
            dragging_cursor_id: None,
            wave_width: 100.0,
            signal_font_size: 12.0,
            right_click_pos: None,
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
        debug!("signals: {} => {}", self.signals.len(), signals.len());
        self.signals = signals;
    }
    pub fn x_to_pos(&self, x: f32) -> u64 {
        (x * (self.range.1 - self.range.0) as f32 / self.wave_width) as u64 + self.range.0
        // x as u64
    }
    pub fn x_to_pos_delta(&self, x: f32) -> i64 {
        (x * (self.range.1 - self.range.0) as f32 / self.wave_width) as i64 + self.range.0 as i64
        // x as u64
    }
    pub fn pos_to_x(&self, pos: u64) -> f32 {
        (pos - self.range.0) as f32 * self.wave_width / (self.range.1 - self.range.0) as f32
        // pos as f32
    }
    pub fn pos_to_time(&self, timescale: &(u64, WaveTimescaleUnit), pos: u64) -> String {
        format!("{}{}", pos * timescale.0, timescale.1)
    }
    pub fn pos_to_time_fmt(&self, timescale: &(u64, WaveTimescaleUnit), pos: u64) -> String {
        let mut v = pos * timescale.0;
        let mut u = timescale.1;
        while v > 10 && u.larger().is_some() {
            v = v / 10;
            u = u.larger().unwrap();
        }
        format!("{}{}", v, u)
    }
    pub fn find_cursor(&self, x: f32) -> Option<i32> {
        let judge = |c: &WaveCursor| {
            let cursor_x = self.pos_to_x(c.pos);
            f32::abs(x - cursor_x)
        };
        // find dragging cursor to drag
        // marker_temp cannot drag
        match self.dragging_cursor_id {
            None => self
                .cursors
                .iter()
                .chain([&self.marker /*&self.marker_temp*/])
                .map(|c| (judge(c), c))
                .filter(|x| x.0 <= CURSOR_NEAREST)
                .reduce(|a, b| if a.0 < b.0 { a } else { b })
                .map(|x| x.1.id),
            Some(id) => match id {
                -1 => Some(self.marker.id),
                // -2 => Some(self.marker_temp.id),
                id => self.cursors.iter().find(|x| x.id == id).map(|x| x.id),
            },
        }
    }
    fn next_cursor_id(&self) -> i32 {
        self.cursors
            .iter()
            .map(|x| x.id)
            .max()
            .map(|x| x + 1)
            .unwrap_or(0)
    }
    pub fn reset(&mut self) -> Self {
        info!("reset view");
        let tx = self.tx.take();
        Self {
            tx,
            ..Default::default()
        }
    }
}
