#![cfg(not(target_arch = "wasm32"))]

use crate::manager::MANAGER_PORT;
use crate::rpc::rvcd_client_server::{RvcdClient, RvcdClientServer};
use crate::rpc::rvcd_rpc_client::RvcdRpcClient;
use crate::rpc::RvcdManagedInfo;
use crate::utils::sleep_ms;
use std::ops::Deref;
use std::sync::{mpsc, Arc, Mutex};
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tracing::{info, warn};

#[derive(Debug)]
pub struct RvcdManagedClientData {
    pub port: u16,
    pub paths: Vec<String>,
}

impl Default for RvcdManagedClientData {
    fn default() -> Self {
        Self {
            port: MANAGER_PORT + 1,
            paths: vec![],
        }
    }
}

#[derive(Debug, Default)]
pub struct RvcdManagedClient {
    pub data: Arc<Mutex<RvcdManagedClientData>>,
}

#[tonic::async_trait]
impl RvcdClient for RvcdManagedClient {
    async fn info(&self, _request: Request<()>) -> Result<Response<RvcdManagedInfo>, Status> {
        Ok(Response::new(RvcdManagedInfo {
            client_port: self.data.lock().unwrap().port as u32,
            paths: self.data.lock().unwrap().paths.clone(),
        }))
    }
}

impl RvcdManagedClient {
    pub async fn run(&mut self) {
        let max_port = MANAGER_PORT + 1024;
        while self.data.lock().unwrap().port < max_port {
            let addr = format!("0.0.0.0:{}", self.data.lock().unwrap().port)
                .parse()
                .unwrap();
            let rpc_server = Server::builder()
                .add_service(RvcdClientServer::new(Self {
                    data: self.data.clone(),
                }))
                .serve(addr);
            let stop = Arc::new(Mutex::new(false));
            tokio::select! {
                _ = rpc_server => {},
                _ = Self::streaming_info(self.data.clone(), stop.clone()) => {}
            };
            *stop.lock().unwrap() = true;
            sleep_ms(10).await;
            self.data.lock().as_mut().unwrap().port += 1;
        }
        if self.data.lock().unwrap().port >= max_port {
            warn!("managed client runs out of ports!");
        }
    }
    pub fn set_paths(&self, paths: &[String]) {
        if let Ok(d) = self.data.lock().as_mut() {
            d.paths.clear();
            d.paths.extend_from_slice(paths);
        }
    }
    pub async fn streaming_info(data: Arc<Mutex<RvcdManagedClientData>>, stop: Arc<Mutex<bool>>) {
        let mut client = RvcdRpcClient::connect(format!("http://127.0.0.1:{}", MANAGER_PORT))
            .await
            .unwrap();

        let (tx, rx) = mpsc::channel();
        tokio::spawn(async move {
            loop {
                if let Ok(data) = data.lock() {
                    let paths = data.paths.clone();
                    if tx
                        .send(RvcdManagedInfo {
                            client_port: data.port as u32,
                            paths,
                        })
                        .is_err()
                    {
                        break;
                    }
                } else {
                    break;
                }
                if *stop.lock().unwrap() {
                    break;
                }
            }
        });
        client.client_info(futures::stream::iter(rx)).await.unwrap();
    }
}
