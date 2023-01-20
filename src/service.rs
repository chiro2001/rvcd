use crate::message::{RVCDChannel, RVCDMsg};
use crate::utils::execute;
use crate::wave::vcd::Vcd;
use crate::wave::{Wave, WaveLoader};
use anyhow::Result;
use log::{debug, info};
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

pub struct Service {
    pub wave: Arc<Mutex<Option<Wave>>>,
    pub channel: RVCDChannel,
}

unsafe impl Send for Service {}

impl Service {
    async fn handle_message(&mut self, msg: RVCDMsg) -> Result<()> {
        debug!("handle message: {:?}", msg);
        match msg {
            RVCDMsg::FileOpen(path) => {
                info!("loading file: {:?}", path);
                let mut file = File::open(path.as_os_str().to_str().unwrap()).unwrap();
                if let Ok(w) = Vcd::load(&mut file) {
                    if let Ok(mut wave) = self.wave.lock() {
                        *wave = Some(w);
                        self.channel
                            .tx
                            .send(RVCDMsg::UpdateInfo(wave.as_ref().unwrap().info.copy()))
                            .unwrap();
                        // send path back
                        self.channel.tx.send(RVCDMsg::FileOpen(path)).unwrap();
                    }
                    // *wave.lock().unwrap() = Some(w);
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn new(channel: RVCDChannel) -> Self {
        Self {
            wave: Arc::new(Mutex::new(None)),
            channel,
        }
    }

    pub async fn run(mut self) {
        info!("service starts");
        loop {
            sleep(Duration::from_millis(10));
            self.channel
                .rx
                .try_recv()
                .map(|msg| self.handle_message(msg))
                .ok();
        }
    }

    pub fn start(channel: RVCDChannel) {
        execute(async move {
            Service::new(channel).run().await;
        });
    }
}
