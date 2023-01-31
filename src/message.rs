use crate::wave::{WaveDataItem, WaveInfo};
use std::sync::mpsc;
use rfd::FileHandle;

#[derive(Debug)]
pub enum RvcdMsg {
    FileOpen(FileHandle),
    UpdateInfo(WaveInfo),
    UpdateData(Vec<WaveDataItem>),
}

unsafe impl Send for RvcdMsg {}

pub struct RvcdChannel {
    pub(crate) tx: mpsc::Sender<RvcdMsg>,
    pub(crate) rx: mpsc::Receiver<RvcdMsg>,
}
