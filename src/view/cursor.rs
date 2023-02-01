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
