use std::sync::mpsc;

pub enum RVCDMsg {
    FileOpen(std::path::PathBuf)
}

pub struct RVCDChannel {
    pub(crate) sender: mpsc::Sender<RVCDMsg>,
    pub(crate) receiver: mpsc::Receiver<RVCDMsg>,
}