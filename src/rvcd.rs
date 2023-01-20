use crate::message::{RVCDChannel, RVCDMsg};
use crate::service::service;
use crate::utils::execute;
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
    pub(crate) signal_paths: Vec<Vec<String>>,
    #[serde(skip)]
    pub(crate) signals: Vec<u64>,
}

impl Default for RVCD {
    fn default() -> Self {
        Self {
            state: State::default(),
            channel: None,
            filepath: "".to_string(),
            signal_paths: vec![],
            signals: vec![],
        }
    }
}

impl RVCD {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (channel_req_tx, channel_req_rx) = mpsc::channel();
        let (channel_resp_tx, channel_resp_rx) = mpsc::channel();

        // launch service
        execute(service(RVCDChannel {
            tx: channel_resp_tx,
            rx: channel_req_rx,
        }));

        if let Some(storage) = cc.storage {
            Self {
                channel: Some(RVCDChannel {
                    tx: channel_req_tx,
                    rx: channel_resp_rx,
                }),
                ..eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
            }
        } else {
            Self {
                channel: Some(RVCDChannel {
                    tx: channel_req_tx,
                    rx: channel_resp_rx,
                }),
                ..Default::default()
            }
        }
    }
}
