use crate::wave::{WaveDataItem, WaveInfo};
use std::sync::mpsc;
use rfd::FileHandle;

#[derive(Debug)]
pub enum RvcdMsg {
    FileOpen(FileHandle),
    Reload,
    UpdateInfo(WaveInfo),
    UpdateData(Vec<WaveDataItem>),
}

unsafe impl Send for RvcdMsg {}

#[derive(Debug)]
pub struct RvcdChannel {
    pub tx: mpsc::Sender<RvcdMsg>,
    pub rx: mpsc::Receiver<RvcdMsg>,
}
