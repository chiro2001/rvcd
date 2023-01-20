use crate::message::{RVCDChannel, RVCDMsg};
use log::info;
use std::sync::mpsc::TryRecvError;
use std::sync::{mpsc, Arc, Mutex, TryLockResult};
use std::thread::sleep;
use std::time::Duration;

pub async fn service(channel: RVCDChannel) {
    let rx = Arc::new(Mutex::new(channel.rx));
    let tx = Arc::new(Mutex::new(channel.tx));
    loop {
        sleep(Duration::from_millis(10));
        rx.try_lock()
            .map(|rx| rx.try_recv())
            .map(|r| {
                r.map(|msg| match msg {
                    RVCDMsg::FileOpen(path) => {
                        info!("loading file: {:?}", path);
                    }
                })
                .ok()
            })
            .ok();
    }
}
