#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct WaveView {
    pub signals: Vec<u64>,
    pub range: [u64; 2],
}

impl Default for WaveView {
    fn default() -> Self {
        Self {
            signals: vec![],
            range: [0, 0],
        }
    }
}
