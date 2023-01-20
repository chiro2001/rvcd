use crate::wave::{WaveDataItem, WaveInfo};
use std::sync::mpsc;
use rfd::FileHandle;

#[derive(Debug)]
pub enum RVCDMsg {
    FileOpen(FileHandle),
    UpdateInfo(WaveInfo),
    UpdateData(Vec<WaveDataItem>),
}

unsafe impl Send for RVCDMsg {}

pub struct RVCDChannel {
    pub(crate) tx: mpsc::Sender<RVCDMsg>,
    pub(crate) rx: mpsc::Receiver<RVCDMsg>,
}
