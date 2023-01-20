use crate::message::{RVCDChannel, RVCDMsg};
use crate::service::Service;
use crate::tree_view::TreeView;
use crate::wave::WaveInfo;
use log::info;
use std::path::PathBuf;
use std::sync::mpsc;

#[derive(serde::Deserialize, serde::Serialize, Default)]
pub enum State {
    #[default]
    Idle,
    Loading,
    Working,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct RVCD {
    #[serde(skip)]
    pub(crate) state: State,
    /// ui <- -> service
    #[serde(skip)]
    pub(crate) channel: Option<RVCDChannel>,

    pub(crate) filepath: String,
    #[serde(skip)]
    pub(crate) signals: Vec<u64>,

    #[serde(skip)]
    pub(crate) signal_leaves: Vec<(u64, String)>,

    #[serde(skip)]
    pub(crate) tree: TreeView,
    #[serde(skip)]
    pub(crate) wave_info: Option<WaveInfo>,
}

impl Default for RVCD {
    fn default() -> Self {
        Self {
            state: State::default(),
            channel: None,
            filepath: "".to_string(),
            signals: vec![],
            signal_leaves: vec![],
            tree: Default::default(),
            wave_info: None,
        }
    }
}

impl RVCD {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (channel_req_tx, channel_req_rx) = mpsc::channel();
        let (channel_resp_tx, channel_resp_rx) = mpsc::channel();

        // launch service
        Service::start(RVCDChannel {
            tx: channel_resp_tx,
            rx: channel_req_rx,
        });

        let def = if let Some(storage) = cc.storage {
            let def: RVCD = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            // auto open file
            // let filepath = "data/cpu_ila_commit.vcd";
            let filepath = &def.filepath;
            info!("last file: {}", filepath);
            if !filepath.is_empty() {
                channel_req_tx
                    .send(RVCDMsg::FileOpen(PathBuf::from(filepath)))
                    .unwrap();
            }
            def
        } else {
            Default::default()
        };
        Self {
            channel: Some(RVCDChannel {
                tx: channel_req_tx,
                rx: channel_resp_rx,
            }),
            ..def
        }
    }
}
