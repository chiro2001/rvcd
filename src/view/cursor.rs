use crate::view::{
    WaveView, CURSOR_NEAREST, LINE_WIDTH, TEXT_BG_MULTIPLY, WAVE_MARGIN_TOP, WAVE_MARGIN_TOP2,
};
use crate::wave::WaveInfo;
use egui::*;

#[derive(
    serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default, Ord, PartialOrd, Eq,
)]
pub struct WaveCursor {
    pub id: i32,
    /// Wave position
    pub pos: u64,
    pub name: String,
    pub valid: bool,
}
impl WaveCursor {
    pub fn new(id: i32, pos: u64) -> Self {
        Self {
            id,
            pos,
            name: format!("Cursor{}", id),
            valid: true,
        }
    }
    pub fn from_string(id: i32, name: &str) -> Self {
        Self {
            id,
            pos: 0,
            name: name.to_string(),
            valid: false,
        }
    }
    /// Set position and set valid
    pub fn set_pos_valid(&mut self, pos: u64) {
        self.pos = pos;
        self.valid = true;
    }
}

impl WaveView {
    /// Find nearest cursor according to panel x position
    /// Will ignore `self.marker_temp` and distance larger than `CURSOR_NEAREST`
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
    /// Paint a cursor.
    /// * `offset`: wave panel ui left + signal width + padding(`UI_WIDTH_OFFSET`)
    pub fn paint_cursor(&self, ui: &mut Ui, offset: f32, info: &WaveInfo, cursor: &WaveCursor) {
        let paint_rect = ui.max_rect();
        let paint_rect = Rect::from_min_max(
            paint_rect.min + vec2(0.0, WAVE_MARGIN_TOP + WAVE_MARGIN_TOP2),
            paint_rect.max,
        );
        let painter = ui.painter();
        let bg_color = match cursor.valid {
            true => Color32::YELLOW,
            false => Color32::BLUE.linear_multiply(TEXT_BG_MULTIPLY),
        };
        let get_text_size = |text: &str| {
            painter
                .text(
                    Pos2::ZERO,
                    Align2::RIGHT_BOTTOM,
                    text,
                    Default::default(),
                    Color32::TRANSPARENT,
                )
                .size()
        };
        let x = self.pos_to_x(cursor.pos) + offset;
        if x >= offset && x <= offset + self.wave_width {
            painter.vline(x, paint_rect.y_range(), (LINE_WIDTH, bg_color));
        }
        let paint_text = |text: String, offset_y: f32, expect_width: f32| {
            painter.text(
                pos2(
                    x.clamp(offset, offset + self.wave_width - expect_width),
                    paint_rect.top() + offset_y,
                ),
                Align2::LEFT_TOP,
                text,
                Default::default(),
                Color32::BLACK,
            )
        };
        let time = self.pos_to_time(&info.timescale, cursor.pos);
        let time_rect = paint_text(time.to_string(), 0.0, get_text_size(&time).x);
        painter.rect_filled(time_rect, 0.0, bg_color.linear_multiply(TEXT_BG_MULTIPLY));
        paint_text(time.to_string(), 0.0, get_text_size(&time).x);
        if !cursor.name.is_empty() {
            let name_rect = paint_text(
                cursor.name.to_string(),
                time_rect.height(),
                get_text_size(&cursor.name).x,
            );
            painter.rect_filled(name_rect, 0.0, bg_color.linear_multiply(TEXT_BG_MULTIPLY));
            paint_text(
                cursor.name.to_string(),
                time_rect.height(),
                get_text_size(&cursor.name).x,
            );
        }
    }
    pub fn cursors_exists_id(&self, id: i32) -> bool {
        self.cursors.iter().any(|c| c.id == id)
    }
    pub fn cursors_get(&self, id: i32) -> Option<&WaveCursor> {
        self.cursors.iter().find(|c| c.id == id)
    }
}
