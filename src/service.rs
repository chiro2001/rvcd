use crate::message::{RvcdChannel, RvcdMsg};
use crate::utils::execute;
use crate::wave::vcd::Vcd;
use crate::wave::{Wave, WaveLoader};
use anyhow::Result;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info};

pub struct Service {
    pub wave: Arc<Mutex<Option<Wave>>>,
    pub channel: RvcdChannel,
}

unsafe impl Send for Service {}

impl Service {
    async fn handle_message(&mut self, msg: RvcdMsg) -> Result<()> {
        debug!("handle message: {:?}", msg);
        match msg {
            RvcdMsg::FileOpen(file) => {
                info!("loading file: {:?}", file);
                // let mut file = File::open(path.as_os_str().to_str().unwrap()).unwrap();
                // let mut file = File::open(path.to_string()).unwrap();
                #[cfg(not(target_arch = "wasm32"))]
                let exists = file.path().exists();
                #[cfg(target_arch = "wasm32")]
                let exists = true;

                if exists {
                    // TODO: partly read
                    let data = file.read().await;
                    // if let Ok(w) = Vcd::load(&mut file) {
                    let mut reader = Cursor::new(data);
                    if let Ok(w) = Vcd::load(&mut reader) {
                        info!("service load wave: {}", w);
                        if let Ok(mut wave) = self.wave.lock() {
                            *wave = Some(w);
                            self.channel
                                .tx
                                .send(RvcdMsg::UpdateInfo(wave.as_ref().unwrap().info.copy()))
                                .unwrap();
                            self.channel
                                .tx
                                .send(RvcdMsg::UpdateData(wave.as_ref().unwrap().data.to_vec()))
                                .unwrap();
                            // send path back
                            self.channel.tx.send(RvcdMsg::FileOpen(file)).unwrap();
                        }
                        // *wave.lock().unwrap() = Some(w);
                    }
                } else {
                    #[cfg(not(target_arch = "wasm32"))]
                    if !file.path().to_str().unwrap().is_empty() {
                        self.channel.tx.send(RvcdMsg::FileOpenFailed).unwrap();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn new(channel: RvcdChannel) -> Self {
        Self {
            wave: Arc::new(Mutex::new(None)),
            channel,
        }
    }

    pub async fn run(&mut self) {
        loop {
            #[cfg(not(target_arch = "wasm32"))]
            std::thread::sleep(std::time::Duration::from_millis(10));
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

    pub fn start(channel: RvcdChannel) {
        info!("starting service...");
        execute(async move {
            run_service(channel).await;
        });
        info!("service started");
    }
}

async fn run_service(channel: RvcdChannel) {
    let mut s = Service::new(channel);
    info!("service starts");
    s.run().await
}
