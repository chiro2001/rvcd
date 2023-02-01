use crate::radix::Radix;
use crate::wave::{WaveInfo, WaveSignalInfo};

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug, Clone, Default)]
pub enum SignalViewMode {
    #[default]
    Default,
    Number(Radix),
    Analog,
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

