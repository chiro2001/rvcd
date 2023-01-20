use egui_extras::{Size, StripBuilder};
use vcd::IdCode;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct RVCD {
    filepath: String,
    // shown signals
    signal_paths: Vec<Vec<String>>,
    #[serde(skip)]
    signals: Vec<IdCode>,
}

impl Default for RVCD {
    fn default() -> Self {
        Self {
            filepath: "".to_string(),
            signal_paths: vec![],
            signals: vec![],
        }
    }
}

impl RVCD {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}