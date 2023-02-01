#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default)]
pub struct WaveCursor {
    pub id: usize,
    pub pos: u64,
    pub name: String,
    pub valid: bool,
}
impl WaveCursor {
    pub fn new(id: usize, pos: u64) -> Self {
        Self {
            id,
            pos,
            name: format!("Cursor{}", id),
            valid: true,
        }
    }
    pub fn from_string(name: &str) -> Self {
        Self {
            id: 0,
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
