use crate::server::server::rvcd_rpc_server::RvcdRpc;
use tonic::{Request, Response, Status};
use tracing::debug;

pub mod server {
    tonic::include_proto!("rvcd");
}

#[derive(Debug, Default)]
pub struct RvcdRemote {}

#[tonic::async_trait]
impl RvcdRpc for RvcdRemote {
    async fn open_file(&self, request: Request<String>) -> Result<Response<()>, Status> {
        debug!("got a request open_file: {:?}", request);
        Ok(Response::new(()))
    }
}
