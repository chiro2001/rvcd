use std::sync::mpsc;

pub enum RVCDMsg {
    FileOpen(std::path::PathBuf),
    UpdateInfo()
}

pub struct RVCDChannel {
    pub(crate) tx: mpsc::Sender<RVCDMsg>,
    pub(crate) rx: mpsc::Receiver<RVCDMsg>,
}