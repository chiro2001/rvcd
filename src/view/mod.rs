pub mod cursor;
pub mod signal;
pub mod time_bar;
pub mod ui;

use crate::message::RvcdMsg;
use crate::radix::Radix;
use crate::view::cursor::WaveCursor;
use crate::view::signal::{SignalView, SignalViewAlign};
use crate::wave::{WaveInfo, WaveTimescaleUnit};
use egui::*;
use std::sync::mpsc;
use tracing::*;

const LINE_WIDTH: f32 = 1.5;
const TEXT_ROUND_OFFSET: f32 = 4.0;
const MIN_SIGNAL_WIDTH: f32 = 2.0;
const BG_MULTIPLY: f32 = 0.05;
const TEXT_BG_MULTIPLY: f32 = 0.4;
const CURSOR_NEAREST: f32 = 20.0;
const UI_WIDTH_OFFSET: f32 = 8.0;
const ZOOM_SIZE_MIN: f32 = 12.0;
const ZOOM_SIZE_MAX_SCALE: f32 = 5.0;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(default)]
pub struct WaveView {
    /// Signals added to viewer
    pub signals: Vec<SignalView>,
    /// Viewer range, smaller or bigger than data range, TODO: change to float for zooming
    pub range: (f32, f32),
    /// Text alignment, FIXME: center position error
    pub align: SignalViewAlign,
    /// Whether to show background in signals
    pub background: bool,
    /// Whether to show value text
    pub show_text: bool,
    pub default_radix: Radix,
    /// Message sender to main ui app
    #[serde(skip)]
    pub tx: Option<mpsc::Sender<RvcdMsg>>,
    pub cursors: Vec<WaveCursor>,
    /// Valid after dragging released
    pub marker: WaveCursor,
    /// Valid when dragging
    pub marker_temp: WaveCursor,
    /// Available spans
    pub spans: Vec<(i32, i32)>,
    /// Temporally use to store id
    #[serde(skip)]
    pub dragging_cursor_id: Option<i32>,
    /// remember display width to calculate position
    #[serde(skip)]
    pub wave_width: f32,
    /// Value text size
    pub signal_font_size: f32,
    /// Temporally use to store right click position
    #[serde(skip)]
    pub right_click_pos: Option<Pos2>,
    #[serde(skip)]
    pub middle_click_pos: Option<Pos2>,
    #[serde(skip)]
    pub scroll_start_pos: Pos2,
    #[serde(skip)]
    pub scroll_end: bool,
    #[serde(skip)]
    pub edit_range_from: String,
    #[serde(skip)]
    pub edit_range_to: String,
    pub limit_range_left: bool,
}

impl Default for WaveView {
    fn default() -> Self {
        Self {
            signals: vec![],
            range: (0.0, 0.0),
            align: Default::default(),
            background: true,
            show_text: true,
            default_radix: Radix::Hex,
            tx: None,
            cursors: vec![],
            marker: WaveCursor::from_string(-1, "Main Cursor"),
            marker_temp: WaveCursor::from_string(-2, ""),
            spans: vec![],
            dragging_cursor_id: None,
            wave_width: 100.0,
            signal_font_size: 12.0,
            right_click_pos: None,
            middle_click_pos: None,
            scroll_start_pos: Default::default(),
            scroll_end: false,
            edit_range_from: "0".to_string(),
            edit_range_to: "0".to_string(),
            limit_range_left: true,
        }
    }
}

impl WaveView {
    /// * `tx`: mpsc message sender from parent
    pub fn new(tx: mpsc::Sender<RvcdMsg>) -> Self {
        Self {
            tx: Some(tx),
            ..Default::default()
        }
    }
    pub fn set_tx(&mut self, tx: mpsc::Sender<RvcdMsg>) {
        self.tx = Some(tx);
    }
    /// Remove signals that not defined in wave info, used in `reload()`
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
    /// Convert paint pos to wave position
    /// * `x`: x position to wave panel
    pub fn x_to_pos(&self, x: f32) -> u64 {
        ((x * (self.range.1 - self.range.0) / self.wave_width) + self.range.0) as u64
    }
    pub fn x_to_fpos(&self, x: f32) -> f32 {
        (x * (self.range.1 - self.range.0) / self.wave_width) + self.range.0
    }
    pub fn x_to_pos_delta(&self, x: f32) -> i64 {
        ((x * (self.range.1 - self.range.0) / self.wave_width) + self.range.0) as i64
    }
    /// Convert wave position to paint pos
    /// * `pos`: wave position defined in wave info
    pub fn pos_to_x(&self, pos: u64) -> f32 {
        (pos as f32 - self.range.0) * self.wave_width / (self.range.1 - self.range.0)
    }
    pub fn fpos_to_x(&self, pos: f32) -> f32 {
        (pos - self.range.0) * self.wave_width / (self.range.1 - self.range.0)
    }
    /// Stringify wave position
    pub fn pos_to_time(&self, timescale: &(u64, WaveTimescaleUnit), pos: u64) -> String {
        format!("{}{}", pos * timescale.0, timescale.1)
    }
    /// Stringify wave position, normalized
    pub fn pos_to_time_fmt(&self, timescale: &(u64, WaveTimescaleUnit), pos: u64) -> String {
        let mut v = pos * timescale.0;
        let mut u = timescale.1;
        while v > 10 && u.larger().is_some() {
            v = v / 10;
            u = u.larger().unwrap();
        }
        format!("{}{}", v, u)
    }
    /// Get new id for cursor
    fn next_cursor_id(&self) -> i32 {
        self.cursors
            .iter()
            .map(|x| x.id)
            .max()
            .map(|x| x + 1)
            .unwrap_or(0)
    }
    /// Reset this view, return new view
    ///
    /// # Example
    ///
    /// ```rust
    /// use rvcd::view::WaveView;
    /// let mut view: WaveView = Default::default();
    /// let view = view.reset();
    /// ```
    pub fn reset(&mut self) -> Self {
        info!("reset view");
        let tx = self.tx.take();
        Self {
            tx,
            ..Default::default()
        }
    }
}
