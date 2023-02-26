use crate::rpc::rvcd_client_server::RvcdClient;
use crate::rpc::RvcdManagedInfo;
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct RvcdManagedClient {
    pub port: u16,
    pub paths: Vec<String>,
}

#[tonic::async_trait]
impl RvcdClient for RvcdManagedClient {
    async fn info(&self, _request: Request<()>) -> Result<Response<RvcdManagedInfo>, Status> {
        Ok(Response::new(RvcdManagedInfo {
            client_port: self.port as u32,
            paths: self.paths.clone(),
        }))
    }
}
