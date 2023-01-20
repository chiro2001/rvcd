use crate::message::{RVCDChannel, RVCDMsg};
use crate::wave::vcd::Vcd;
use crate::wave::{Wave, WaveLoader};
use log::info;
use std::fs::File;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

pub async fn service(channel: RVCDChannel) {
    let rx = Arc::new(Mutex::new(channel.rx));
    let tx = Arc::new(Mutex::new(channel.tx));

    let mut wave = Arc::new(Mutex::new(None));

    loop {
        sleep(Duration::from_millis(10));
        rx.try_lock()
            .map(|rx| rx.try_recv())
            .map(|r| {
                r.map(|msg| match msg {
                    RVCDMsg::FileOpen(path) => {
                        info!("loading file: {:?}", path);
                        let mut file = File::open(path.as_os_str().to_str().unwrap()).unwrap();
                        if let Ok(w) = Vcd::load(&mut file) {
                            if let Ok(mut wave) = wave.lock() {
                                *wave = Some(w);
                                // tx.lock().unwrap().send(RVCDMsg::UpdateInfo(Arc::new(Mutex::new(wave.as_ref().unwrap().info))));
                                tx.lock()
                                    .unwrap()
                                    .send(RVCDMsg::UpdateInfo(wave.as_ref().unwrap().info.copy()))
                                    .unwrap();
                            }
                            // *wave.lock().unwrap() = Some(w);
                            // tx.lock().unwrap().send(RVCDMsg::UpdateInfo(wave.lock().unwrap().unwrap().info)).unwrap();
                        }
                    }
                    _ => {}
                })
                .ok()
            })
            .ok();
    }
}
