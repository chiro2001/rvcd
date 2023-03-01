#![cfg(not(target_arch = "wasm32"))]

use crate::rpc::rvcd_client_client::RvcdClientClient;
use crate::rpc::rvcd_rpc_server::RvcdRpc;
use crate::rpc::{RvcdEmpty, RvcdManagedInfo, RvcdOpenFile, RvcdSignalPath};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tonic::transport::Channel;
use tonic::{Request, Response, Status};
use tracing::{debug, info, trace};

pub const MANAGER_PORT: u16 = 5411;

#[derive(Clone)]
pub enum RvcdRpcMessage {
    GotoPath(RvcdSignalPath),
    OpenWaveFile(String),
    OpenSourceFile(String),
}
unsafe impl Send for RvcdRpcMessage {}

#[derive(Debug)]
pub struct RvcdManager {
    pub managed_files: Mutex<HashMap<u32, (String, Vec<String>, Instant)>>,
    pub tx: Arc<Mutex<mpsc::Sender<RvcdRpcMessage>>>,
}

impl RvcdManager {
    pub fn new(tx: mpsc::Sender<RvcdRpcMessage>) -> Self {
        Self {
            tx: Arc::new(Mutex::new(tx)),
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
        debug!("got a request open_file: {:?}", request);
        let data = request.into_inner();
        let mut found = false;
        let managed_files = { self.managed_files.lock().unwrap().clone() };
        for (_k, v) in managed_files {
            if v.0.as_str() == data.path.as_str() {
                found = true;
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
            if v.0.as_str() == data.file.as_str() {
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
}
