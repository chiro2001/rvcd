#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct WaveView {
    pub(crate) signals: Vec<u64>,
}

impl Default for WaveView {
    fn default() -> Self {
        Self { signals: vec![] }
    }
}
