pub mod cursor;
pub mod signal;
pub mod time_bar;
pub mod ui;

use crate::message::RvcdMsg;
use crate::radix::Radix;
use crate::view::cursor::WaveCursor;
use crate::view::signal::{SignalView, SignalViewAlign};
use crate::view::ui::ResponsePointerState;
use crate::wave::{WaveInfo, WaveTimescaleUnit};
use egui::*;
use std::sync::mpsc;
use tracing::*;

pub const LINE_WIDTH: f32 = 1.5;
pub const TEXT_ROUND_OFFSET: f32 = 4.0;
pub const MIN_SIGNAL_WIDTH: f32 = 2.0;
pub const BG_MULTIPLY: f32 = 0.05;
pub const TEXT_BG_MULTIPLY: f32 = 0.4;
pub const CURSOR_NEAREST: f32 = 20.0;
// pub const UI_WIDTH_OFFSET: f32 = 8.0;
pub const UI_WIDTH_OFFSET: f32 = 16.0;
pub const ZOOM_SIZE_MIN: f32 = 12.0;
pub const ZOOM_SIZE_MAX_SCALE: f32 = 5.0;
pub const WAVE_MARGIN_TOP: f32 = 32.0;
pub const WAVE_MARGIN_TOP2: f32 = -6.0;
pub const SIGNAL_HEIGHT_DEFAULT: f32 = 30.0;
pub const SIGNAL_LEAF_HEIGHT_DEFAULT: f32 = 20.0;
pub const SIGNAL_TREE_HEIGHT_DEFAULT: f32 = 20.0;

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
#[serde(default)]
pub struct WaveView {
    pub id: usize,
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
    pub right_click_time_bar_pos: Option<Pos2>,
    #[serde(skip)]
    pub move_drag_start_pos: Option<Pos2>,
    #[serde(skip)]
    pub move_drag_last_pos: Option<Pos2>,
    #[serde(skip)]
    pub scrolling_next_index: Option<usize>,
    #[serde(skip)]
    pub scrolling_last_index: Option<usize>,
    #[serde(skip)]
    pub scroll_end: bool,
    pub limit_range_left: bool,
    pub use_top_margin: bool,
    pub round_pointer: bool,
    #[serde(skip)]
    pub last_pointer_state: ResponsePointerState,
    #[serde(skip)]
    pub range_seek_started: bool,
    #[serde(skip)]
    pub value_width_max: f32,
}

impl Default for WaveView {
    fn default() -> Self {
        Self {
            id: 0,
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
            right_click_time_bar_pos: None,
            move_drag_start_pos: None,
            move_drag_last_pos: None,
            scrolling_next_index: None,
            scrolling_last_index: None,
            scroll_end: false,
            limit_range_left: true,
            use_top_margin: true,
            round_pointer: true,
            last_pointer_state: Default::default(),
            range_seek_started: false,
            value_width_max: 0.0,
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
    pub fn set_id(&mut self, id: usize) {
        self.id = id;
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
            .filter(|signal| info.code_signal_info.contains_key(&signal.s.id))
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
            v /= 10;
            u = u.larger().unwrap();
        }
        format!("{v}{u}")
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
