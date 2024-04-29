#![cfg(not(target_arch = "wasm32"))]

use crate::rpc::rvcd_client_client::RvcdClientClient;
use crate::rpc::rvcd_rpc_server::RvcdRpc;
use crate::rpc::{
    RvcdEmpty, RvcdFrame, RvcdInputEvent, RvcdLoadSourceDir, RvcdLoadSources, RvcdManagedInfo,
    RvcdOpenFile, RvcdOpenFileWith, RvcdRemoveClient, RvcdSignalPath,
};
use egui::ColorImage;
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tonic::transport::Channel;
use tonic::{IntoRequest, Request, Response, Status};
use tracing::{info, trace, warn};

pub const MANAGER_PORT: u16 = 5411;
pub const DISP_PORT: u16 = 5444;
pub const UNIX_FB_PATH: &str = "/tmp/rvcd.socket";

#[derive(Clone, Debug)]
pub enum RvcdRpcMessage {
    GotoPath(RvcdSignalPath),
    OpenWaveFile(String),
    OpenSourceFile(String),
    OpenSourceDir(String),
    RequestFrame,
    InputEvent(RvcdInputEvent),
}
unsafe impl Send for RvcdRpcMessage {}

#[derive(Clone, Debug, Default)]
pub enum RvcdManagerMessage {
    // RecvFrame(RvcdFrame),
    #[default]
    None,
    RecvFrame(Arc<ColorImage>),
}
unsafe impl Send for RvcdManagerMessage {}

#[derive(Clone, Debug)]
pub enum RvcdExitMessage {
    Exit,
}
unsafe impl Send for RvcdExitMessage {}

#[derive(Debug)]
pub struct RvcdManager {
    pub managed_files: Mutex<HashMap<u32, (String, Vec<String>, Instant)>>,
    pub tx: Arc<Mutex<mpsc::Sender<RvcdRpcMessage>>>,
    pub rx: Arc<Mutex<mpsc::Receiver<RvcdManagerMessage>>>,
    pub exit_tx: Arc<Mutex<mpsc::Sender<RvcdExitMessage>>>,
}

impl RvcdManager {
    pub fn new(
        tx: mpsc::Sender<RvcdRpcMessage>,
        rx: Arc<Mutex<mpsc::Receiver<RvcdManagerMessage>>>,
        exit_tx: mpsc::Sender<RvcdExitMessage>,
    ) -> Self {
        Self {
            tx: Arc::new(Mutex::new(tx)),
            rx,
            exit_tx: Arc::new(Mutex::new(exit_tx)),
            managed_files: Default::default(),
        }
    }
}

#[tonic::async_trait]
impl RvcdRpc for RvcdManager {
    async fn open_file(
        &self,
        request: Request<RvcdOpenFile>,
    ) -> Result<Response<RvcdEmpty>, Status> {
        info!("got a request open_file: {:?}", request);
        let data = request.into_inner();
        let mut found = false;
        let managed_files = { self.managed_files.lock().unwrap().clone() };
        for (_k, v) in managed_files {
            if v.0.as_str() == data.path.as_str() {
                found = true;
                info!("duplicated file: [{}] {:?}", _k, v);
                break;
            }
        }
        if !found {
            // send to self, open new
            self.tx
                .lock()
                .unwrap()
                .send(RvcdRpcMessage::OpenWaveFile(data.path))
                .unwrap();
        }
        Ok(Response::new(RvcdEmpty::default()))
    }

    async fn goto_signal(
        &self,
        request: Request<RvcdSignalPath>,
    ) -> Result<Response<RvcdEmpty>, Status> {
        let data = request.into_inner();
        let mut found = false;
        let managed_files = { self.managed_files.lock().unwrap().clone() };
        for (k, v) in managed_files {
            if v.0.as_str() == data.file.as_str() || data.file.is_empty() {
                if let Ok(channel) = Channel::from_shared(format!("http://127.0.0.1:{}", k)) {
                    let channel = channel.connect().await;
                    if let Ok(channel) = channel {
                        let channel =
                            tower::timeout::Timeout::new(channel, Duration::from_millis(100));
                        let mut client = RvcdClientClient::new(channel);
                        if let Ok(_e) = client.goto_signal(data.clone()).await {
                            info!("ask {} goto signal {:?}", k, data);
                            found = true;
                            break;
                        }
                    }
                }
            }
        }
        if !found {
            // send to self, open new
            self.tx
                .lock()
                .unwrap()
                .send(RvcdRpcMessage::GotoPath(data.clone()))
                .unwrap();
        }
        Ok(Response::new(RvcdEmpty::default()))
    }

    async fn client_info(
        &self,
        request: Request<RvcdManagedInfo>,
    ) -> Result<Response<RvcdEmpty>, Status> {
        trace!("client_info");
        let r = request.into_inner();
        trace!("manager recv client: {:?}", r);
        let m = {
            let mut m = self.managed_files.lock().unwrap();
            m.insert(r.client_port, (r.wave_file, r.paths, Instant::now()));
            m.clone()
        };
        // notify outdated clients
        let mut to_remove_keys = vec![];
        for (k, v) in m.iter() {
            let now = Instant::now();
            if now.duration_since(v.2) > Duration::from_millis(2000) {
                to_remove_keys.push(*k);
                if let Ok(channel) = Channel::from_shared(format!("http://127.0.0.1:{}", k)) {
                    let channel = channel.connect().await;
                    if let Ok(channel) = channel {
                        let channel =
                            tower::timeout::Timeout::new(channel, Duration::from_millis(100));
                        let mut client = RvcdClientClient::new(channel);
                        let _e = client.ping(RvcdEmpty::default()).await;
                    }
                }
            }
        }
        for k in to_remove_keys {
            info!("removing port {}", k);
            self.managed_files.lock().unwrap().remove(&k);
        }
        Ok(Response::new(RvcdEmpty::default()))
    }

    async fn ping(&self, _request: Request<RvcdEmpty>) -> Result<Response<RvcdEmpty>, Status> {
        Ok(Response::new(RvcdEmpty::default()))
    }

    async fn load_source_dir(
        &self,
        request: Request<RvcdLoadSourceDir>,
    ) -> Result<Response<RvcdEmpty>, Status> {
        self.tx
            .lock()
            .unwrap()
            .send(RvcdRpcMessage::OpenSourceDir(request.into_inner().path))
            .unwrap();
        Ok(Response::new(RvcdEmpty::default()))
    }

    async fn load_source(
        &self,
        request: Request<RvcdLoadSources>,
    ) -> Result<Response<RvcdEmpty>, Status> {
        for file in request.into_inner().files {
            self.tx
                .lock()
                .unwrap()
                .send(RvcdRpcMessage::OpenSourceFile(file))
                .unwrap();
        }
        Ok(Response::new(RvcdEmpty::default()))
    }

    async fn open_file_with(
        &self,
        request: Request<RvcdOpenFileWith>,
    ) -> Result<Response<RvcdEmpty>, Status> {
        let data = request.into_inner();
        info!("open file with: {:?}", data);
        self.open_file(RvcdOpenFile { path: data.file }.into_request())
            .await?;
        self.load_source_dir(
            RvcdLoadSourceDir {
                path: data.source_dir,
            }
            .into_request(),
        )
        .await?;
        self.load_source(
            RvcdLoadSources {
                files: data.source_files,
            }
            .into_request(),
        )
        .await?;
        if let Some(goto) = data.goto {
            self.goto_signal(goto.into_request()).await?;
        }
        Ok(Response::new(RvcdEmpty::default()))
    }

    async fn remove_client(
        &self,
        request: Request<RvcdRemoveClient>,
    ) -> Result<Response<RvcdEmpty>, Status> {
        let data = request.into_inner();
        if self
            .managed_files
            .lock()
            .unwrap()
            .remove(&data.key)
            .is_none()
        {
            warn!("no such key: {}", data.key);
            Err(Status::aborted("No such key"))
        } else {
            info!("removed key: {}", data.key);
            Ok(Response::new(RvcdEmpty::default()))
        }
    }
    async fn request_frame(
        &self,
        _request: Request<RvcdEmpty>,
    ) -> Result<Response<RvcdFrame>, Status> {
        let _ = self.tx.lock().unwrap().send(RvcdRpcMessage::RequestFrame);
        let mut msg = Default::default();
        loop {
            match self.rx.lock().unwrap().recv() {
                Ok(m) => msg = m,
                _ => break,
            }
        }
        let frame = match msg {
            RvcdManagerMessage::RecvFrame(f) => {
                let (width, height) = (f.size[0] as u32, f.size[1] as u32);
                let mut data = vec![0u8; f.pixels.len() * 2];
                // copy data, can be optimized...
                data.iter_mut()
                    // .zip(f.pixels.iter().flat_map(|p| p.to_array()))
                    .zip(f.pixels.iter().flat_map(|p| {
                        let (r, g, b, _a) = p.to_tuple();
                        // [a, r, g, b]
                        // convert to rgb565
                        let red5 = (r as u16 >> 3) & 0x1F;
                        let green6 = (g as u16 >> 2) & 0x3F;
                        let blue5 = (b as u16 >> 3) & 0x1F;
                        let rgb565: u16 = (red5 << 11) | (green6 << 5) | blue5;
                        rgb565.to_be_bytes()
                    }))
                    .for_each(|(d, s)| *d = s);
                let rvcd_frame = RvcdFrame {
                    width,
                    height,
                    data,
                };
                info!(
                    "ready frame: {}x{} data len {}",
                    rvcd_frame.width,
                    rvcd_frame.height,
                    rvcd_frame.data.len()
                );
                Some(rvcd_frame)
            }
            _ => None,
        };
        Ok(Response::new(frame.map_or(Default::default(), |f| f)))
    }
    async fn input_event(
        &self,
        request: Request<RvcdInputEvent>,
    ) -> Result<Response<RvcdEmpty>, Status> {
        self.tx
            .lock()
            .unwrap()
            .send(RvcdRpcMessage::InputEvent(request.into_inner()))
            .unwrap();
        Ok(Response::new(RvcdEmpty::default()))
    }
}
