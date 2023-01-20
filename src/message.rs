use std::sync::{Arc, mpsc, Mutex};
use crate::wave::WaveInfo;

#[derive(Clone)]
pub enum RVCDMsg {
    FileOpen(std::path::PathBuf),
    // UpdateInfo(Arc<Mutex<WaveInfo>>)
    UpdateInfo(WaveInfo)
}

unsafe impl Send for RVCDMsg {

}

pub struct RVCDChannel {
    pub(crate) tx: mpsc::Sender<RVCDMsg>,
    pub(crate) rx: mpsc::Receiver<RVCDMsg>,
}