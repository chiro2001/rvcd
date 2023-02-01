use crate::view::{WaveView, LINE_WIDTH, TEXT_BG_MULTIPLY, CURSOR_NEAREST};
use crate::wave::WaveInfo;
use egui::*;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default)]
pub struct WaveCursor {
    pub id: i32,
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
    pub fn set_pos_valid(&mut self, pos: u64) {
        self.pos = pos;
        self.valid = true;
    }
}

impl WaveView {
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
    pub fn paint_cursor(&self, ui: &mut Ui, offset: f32, info: &WaveInfo, cursor: &WaveCursor) {
        let paint_rect = ui.max_rect();
        let painter = ui.painter();
        let bg_color = match cursor.valid {
            true => Color32::YELLOW,
            false => Color32::BLUE.linear_multiply(TEXT_BG_MULTIPLY),
        };
        let x = self.pos_to_x(cursor.pos) + offset;
        painter.vline(x, paint_rect.y_range(), (LINE_WIDTH, bg_color));
        let paint_text = |text: String, offset_y: f32| {
            painter.text(
                pos2(x, paint_rect.top() + offset_y),
                Align2::LEFT_TOP,
                text,
                Default::default(),
                Color32::BLACK,
            )
        };
        let time = self.pos_to_time(&info.timescale, cursor.pos);
        let time_rect = paint_text(time.to_string(), 0.0);
        painter.rect_filled(time_rect, 0.0, bg_color.linear_multiply(TEXT_BG_MULTIPLY));
        paint_text(time, 0.0);
        if !cursor.name.is_empty() {
            let name_rect = paint_text(cursor.name.to_string(), time_rect.height());
            painter.rect_filled(name_rect, 0.0, bg_color.linear_multiply(TEXT_BG_MULTIPLY));
            paint_text(cursor.name.to_string(), time_rect.height());
        }
    }
}
