use crate::wave::WaveInfo;
use std::sync::mpsc;

#[derive(Clone)]
pub enum RVCDMsg {
    FileOpen(std::path::PathBuf),
    // UpdateInfo(Arc<Mutex<WaveInfo>>)
    UpdateInfo(WaveInfo),
}

unsafe impl Send for RVCDMsg {}

pub struct RVCDChannel {
    pub(crate) tx: mpsc::Sender<RVCDMsg>,
    pub(crate) rx: mpsc::Receiver<RVCDMsg>,
}
