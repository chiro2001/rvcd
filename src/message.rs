use crate::wave::Wave;
use egui_toast::Toast;
use rfd::FileHandle;
use std::fmt::{Debug, Formatter};
use std::sync::{mpsc, Arc};

// #[derive(Debug)]
pub enum RvcdMsg {
    FileOpen(FileHandle),
    FileOpenFailed,
    Reload,
    UpdateWave(Arc<Wave>),
    Notification(Toast),
}

impl Debug for RvcdMsg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            RvcdMsg::Notification(_toast) => write!(f, "RvcdMsg: Toast[...]"),
            RvcdMsg::FileOpen(file) => write!(f, "RvcdMsg: FileOpen({:?})", file),
            RvcdMsg::FileOpenFailed => write!(f, "RvcdMsg: FileOpenFailed"),
            RvcdMsg::Reload => write!(f, "RvcdMsg: Reload"),
            RvcdMsg::UpdateWave(_) => write!(f, "RvcdMsg: UpdateWave"),
        }
    }
}

/// We must assert all data in [RvcdMsg] are safe to send
unsafe impl Send for RvcdMsg {}

/// [RvcdMsg] tx-rx pair
#[derive(Debug)]
pub struct RvcdChannel {
    pub tx: mpsc::Sender<RvcdMsg>,
    pub rx: mpsc::Receiver<RvcdMsg>,
}
