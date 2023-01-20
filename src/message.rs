use crate::wave::WaveInfo;
use std::sync::mpsc;

#[derive(Clone, Debug)]
pub enum RVCDMsg {
    FileOpen(std::path::PathBuf),
    UpdateInfo(WaveInfo),
}

unsafe impl Send for RVCDMsg {}

pub struct RVCDChannel {
    pub(crate) tx: mpsc::Sender<RVCDMsg>,
    pub(crate) rx: mpsc::Receiver<RVCDMsg>,
}
