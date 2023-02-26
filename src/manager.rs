#![cfg(not(target_arch = "wasm32"))]

use crate::rpc::rvcd_rpc_server::RvcdRpc;
use crate::rpc::{RvcdManagedInfo, RvcdSignalPath};
use std::collections::HashMap;
use std::sync::Mutex;
use tonic::{Request, Response, Status};
use tracing::{debug, info, trace};

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

    async fn client_info(&self, request: Request<RvcdManagedInfo>) -> Result<Response<()>, Status> {
        trace!("client_info");
        let r = request.into_inner();
        trace!("manager recv client: {:?}", r);
        let mut m = self.managed_files.lock().unwrap();
        m.insert(r.client_port, r.paths);
        Ok(Response::new(()))
    }
}
