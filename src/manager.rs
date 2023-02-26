#![cfg(not(target_arch = "wasm32"))]

use crate::rpc::rvcd_client_client::RvcdClientClient;
use crate::rpc::rvcd_rpc_server::RvcdRpc;
use crate::rpc::{RvcdManagedInfo, RvcdSignalPath};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tonic::transport::Channel;
use tonic::{Request, Response, Status};
use tracing::{debug, info, trace};

pub const MANAGER_PORT: u16 = 5411;

#[derive(Debug, Default)]
pub struct RvcdManager {
    pub managed_files: Mutex<HashMap<u32, (Vec<String>, std::time::Instant)>>,
}

#[tonic::async_trait]
impl RvcdRpc for RvcdManager {
    async fn open_file(&self, request: Request<String>) -> Result<Response<()>, Status> {
        debug!("got a request open_file: {:?}", request);
        Ok(Response::new(()))
    }

    async fn goto_signal(&self, _request: Request<RvcdSignalPath>) -> Result<Response<()>, Status> {
        todo!()
    }

    async fn client_info(&self, request: Request<RvcdManagedInfo>) -> Result<Response<()>, Status> {
        trace!("client_info");
        let r = request.into_inner();
        trace!("manager recv client: {:?}", r);
        let m = {
            let mut m = self.managed_files.lock().unwrap();
            m.insert(r.client_port, (r.paths, std::time::Instant::now()));
            // notify outdated clients
            m.clone()
        };
        let mut to_remove_keys = vec![];
        for (k, v) in m.iter() {
            let now = Instant::now();
            if now.duration_since(v.1) > Duration::from_millis(2000) {
                to_remove_keys.push(*k);
                if let Ok(channel) = Channel::from_shared(format!("http://127.0.0.1:{}", k)) {
                    let channel = channel.connect().await;
                    if let Ok(channel) = channel {
                        let channel =
                            tower::timeout::Timeout::new(channel, Duration::from_millis(100));
                        let mut client = RvcdClientClient::new(channel);
                        let _e = client.ping(()).await;
                    }
                }
            }
        }
        Ok(Response::new(()))
    }
}
