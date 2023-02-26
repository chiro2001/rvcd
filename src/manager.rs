#![cfg(not(target_arch = "wasm32"))]

use crate::rpc::rvcd_rpc_server::RvcdRpc;
use crate::rpc::{RvcdManagedInfo, RvcdSignalPath};
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Mutex;
use tonic::{Request, Response, Status, Streaming};
use tracing::debug;

pub const MANAGER_PORT: u16 = 5411;

#[derive(Debug, Default)]
pub struct RvcdManager {
    pub managed_files: Mutex<HashMap<u32, Vec<String>>>,
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

    async fn client_info(
        &self,
        request: Request<Streaming<RvcdManagedInfo>>,
    ) -> Result<Response<()>, Status> {
        let mut stream = request.into_inner();
        while let Some(result) = stream.next().await {
            if let Ok(r) = result {
                let mut m = self.managed_files.lock().unwrap();
                m.insert(r.client_port, r.paths);
            }
        }
        Ok(Response::new(()))
    }
}
