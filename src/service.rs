use crate::message::{RvcdChannel, RvcdMsg};
use crate::utils::{execute, sleep_ms};
use crate::wave::vcd_parser::Vcd;
use crate::wave::WaveLoader;
use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::sync::{mpsc, Arc, Mutex};
use tracing::{error, info};

pub struct Service {
    pub channel: RvcdChannel,
    pub self_loop: RvcdChannel,
    pub cancel: Arc<Mutex<bool>>,
    pub loading: Arc<Mutex<bool>>,
}

unsafe impl Send for Service {}

impl Service {
    fn parse_data_send(&self, reader: &mut dyn Read) -> bool {
        if let Ok(wave) = Vcd::load(reader) {
            info!("service load wave: {}", wave);
            self.channel.tx.send(RvcdMsg::UpdateWave(wave)).unwrap();
            true
        } else {
            false
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn load_data_loop(
        path: String,
        tx: mpsc::Sender<RvcdMsg>,
        loop_tx: mpsc::Sender<RvcdMsg>,
        cancel: Arc<Mutex<bool>>,
    ) {
        let data = Self::load_data(path, tx, cancel);
        loop_tx.send(RvcdMsg::ServiceDataReady(data)).unwrap();
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn load_data(path: String, tx: mpsc::Sender<RvcdMsg>, cancel: Arc<Mutex<bool>>) -> Vec<u8> {
        let file = File::open(path);
        match file {
            Ok(file) => {
                let total_sz = file.metadata().unwrap().len();
                let mut reader = BufReader::new(file);
                if total_sz != 0 {
                    const BUF_SIZE: usize = 1024 * 256;
                    // const BUF_SIZE: usize = 8;
                    let mut data = vec![0u8; total_sz as usize];
                    let mut buf = [0u8; BUF_SIZE];
                    let mut count = 0;
                    let mut canceled = false;
                    info!("start reading file");
                    let time_start = std::time::Instant::now();
                    while let Ok(sz) = reader.read(&mut buf) {
                        if sz == 0 {
                            break;
                        }
                        data[count..(count + sz)].copy_from_slice(&buf[0..sz]);
                        count += sz;
                        let progress = count as f32 / total_sz as f32;
                        tx.send(RvcdMsg::LoadingProgress(progress, count)).unwrap();
                        match cancel.lock() {
                            Ok(mut r) => {
                                if *r {
                                    info!("cancel flag detected, false");
                                    canceled = true;
                                    *r = false;
                                    break;
                                }
                            }
                            Err(_e) => {
                                canceled = true;
                                error!("{}", _e);
                            }
                        }
                        // sleep_ms(1000).await;
                        // std::thread::sleep(std::time::Duration::from_millis(1000));
                    }
                    let time_stop = std::time::Instant::now();
                    let duration = time_stop - time_start;
                    info!("stop reading file, used {} seconds", duration.as_secs());
                    if !canceled {
                        data
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            }
            Err(_) => {
                vec![]
            }
        }
    }
    async fn handle_message(&mut self, msg: RvcdMsg) -> Result<()> {
        info!("service handle msg: {:?}", msg);
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
                    // send path back
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        self.channel
                            .tx
                            .send(RvcdMsg::FileLoadStart(
                                file.path().to_str().unwrap().to_string(),
                            ))
                            .unwrap();
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        self.channel
                            .tx
                            .send(RvcdMsg::FileLoadStart("".to_string()))
                            .unwrap();
                    }
                    #[cfg(target_arch = "wasm32")]
                    let data = Some(file.read().await);
                    #[cfg(not(target_arch = "wasm32"))]
                    let data: Option<Vec<u8>> = {
                        let path = file.path().to_str().unwrap().to_string();
                        let tx = self.channel.tx.clone();
                        let loop_tx = self.self_loop.tx.clone();
                        let cancel = self.cancel.clone();
                        let _th = std::thread::spawn(move || {
                            Self::load_data_loop(path, tx, loop_tx, cancel)
                        });
                        *self.loading.lock().unwrap() = true;
                        None
                    };
                    if let Some(data) = data {
                        self.self_loop
                            .tx
                            .send(RvcdMsg::ServiceDataReady(data))
                            .unwrap();
                    }
                } else {
                    #[cfg(not(target_arch = "wasm32"))]
                    if !file.path().to_str().unwrap().is_empty() {
                        self.channel.tx.send(RvcdMsg::FileOpenFailed).unwrap();
                    }
                }
            }
            RvcdMsg::FileOpenData(data) => {
                // TODO: reduce this data clone
                let mut reader: Cursor<Vec<_>> = Cursor::new(data.to_vec());
                if !self.parse_data_send(&mut reader) {
                    self.channel.tx.send(RvcdMsg::FileOpenFailed).unwrap();
                }
            }
            RvcdMsg::FileLoadCancel => {
                if let Ok(mut r) = self.cancel.lock() {
                    if !*r && *self.loading.lock().unwrap() {
                        *r = true;
                        info!("set cancel flag true");
                    }
                }
            }
            RvcdMsg::ServiceDataReady(data) => {
                *self.loading.lock().unwrap() = false;
                info!("start parsing data");
                // if let Ok(w) = Vcd::load(&mut file) {
                let mut reader = Cursor::new(data);
                if !self.parse_data_send(&mut reader) {
                    self.channel.tx.send(RvcdMsg::FileOpenFailed).unwrap();
                }
                info!("stop parsing data");
            }
            _ => {}
        }
        Ok(())
    }

    pub fn new(channel: RvcdChannel) -> Self {
        let (channel_loop_tx, channel_loop_rx) = mpsc::channel();
        let self_loop = RvcdChannel {
            tx: channel_loop_tx,
            rx: channel_loop_rx,
        };
        Self {
            channel,
            self_loop,
            cancel: Arc::new(Mutex::new(false)),
            loading: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn run(&mut self) {
        loop {
            sleep_ms(10).await;
            {
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
            {
                let r = self
                    .self_loop
                    .rx
                    .try_recv()
                    .map(|msg| self.handle_message(msg));
                if let Ok(r) = r {
                    match r.await {
                        Ok(_) => {}
                        Err(e) => {
                            error!("service loop run error: {}", e)
                        }
                    };
                }
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
