use crate::wave::{WaveDataItem, WaveInfo};
use egui_toast::Toast;
use rfd::FileHandle;
use std::fmt::{Debug, Formatter};
use std::sync::mpsc;

// #[derive(Debug)]
pub enum RvcdMsg {
    FileOpen(FileHandle),
    FileOpenFailed,
    Reload,
    UpdateInfo(WaveInfo),
    UpdateData(Vec<WaveDataItem>),
    Notification(Toast),
}

impl Debug for RvcdMsg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            RvcdMsg::Notification(_toast) => write!(f, "Toast[...]"),
            _ => write!(f, "{:?}", self),
        }
    }
}

unsafe impl Send for RvcdMsg {}

#[derive(Debug)]
pub struct RvcdChannel {
    pub tx: mpsc::Sender<RvcdMsg>,
    pub rx: mpsc::Receiver<RvcdMsg>,
}
