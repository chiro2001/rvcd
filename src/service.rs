use crate::message::{RVCDChannel, RVCDMsg};
use crate::utils::execute;
use crate::wave::vcd::Vcd;
use crate::wave::{Wave, WaveLoader};
use anyhow::Result;
use tracing::{debug, error, info};
use std::io::Cursor;
use std::sync::{Arc, Mutex};
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
            RVCDMsg::FileOpen(mut file) => {
                info!("loading file: {:?}", file);
                // let mut file = File::open(path.as_os_str().to_str().unwrap()).unwrap();
                // let mut file = File::open(path.to_string()).unwrap();
                let data = file.read().await;
                // if let Ok(w) = Vcd::load(&mut file) {
                let mut reader = Cursor::new(data);
                if let Ok(w) = Vcd::load(&mut reader) {
                    if let Ok(mut wave) = self.wave.lock() {
                        *wave = Some(w);
                        self.channel
                            .tx
                            .send(RVCDMsg::UpdateInfo(wave.as_ref().unwrap().info.copy()))
                            .unwrap();
                        // send path back
                        self.channel.tx.send(RVCDMsg::FileOpen(file)).unwrap();
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

    pub async fn run(&mut self) {
        loop {
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            #[cfg(target_arch = "wasm32")]
            {
                // #[wasm_bindgen]
                pub fn sleep(ms: i32) -> js_sys::Promise {
                    js_sys::Promise::new(&mut |resolve, _| {
                        web_sys::window()
                            .unwrap()
                            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
                            .unwrap();
                    })
                }
                let promise = sleep(10);
                let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
            }
            let r = self
                .channel
                .rx
                .try_recv()
                .map(|msg| self.handle_message(msg));
            if let Ok(r) = r {
                match r.await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("service run error: {}", e)
                    }
                };
            }
        }
    }

    pub fn start(channel: RVCDChannel) {
        info!("starting service...");
        execute(async move {
            run_service(channel).await;
        });
        info!("service started");
    }
}

async fn run_service(channel: RVCDChannel) {
    let mut s = Service::new(channel);
    info!("service starts");
    s.run().await
}
