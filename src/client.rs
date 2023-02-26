#![cfg(not(target_arch = "wasm32"))]

use crate::manager::MANAGER_PORT;
use crate::rpc::rvcd_client_server::{RvcdClient, RvcdClientServer};
use crate::rpc::rvcd_rpc_client::RvcdRpcClient;
use crate::rpc::RvcdManagedInfo;
use crate::utils::sleep_ms;
use std::sync::{Arc, Mutex};
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tracing::{info, trace, warn};

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
    pub async fn run(&self) {
        let max_port = MANAGER_PORT + 1024;
        while self.data.lock().unwrap().port < max_port {
            let port = self.data.lock().unwrap().port;
            warn!("child binding at port: {}", port);
            let addr = format!("0.0.0.0:{}", port).parse().unwrap();
            let rpc_server = Server::builder()
                .add_service(RvcdClientServer::new(Self {
                    data: self.data.clone(),
                }))
                .serve(addr);
            let stop = Arc::new(Mutex::new(false));
            let ok;
            tokio::select! {
                r = rpc_server => {
                    info!("rpc_server done with {:?}", r);
                    ok = Some(r.is_ok());
                },
                r = Self::streaming_info(self.data.clone(), stop.clone()) => {
                    info!("streaming_info done with {:?}", r);
                    ok = Some(false);
                }
            };
            *stop.lock().unwrap() = true;
            if !ok.unwrap() {
                self.data.lock().as_mut().unwrap().port += 1;
            } else {
                break;
            }
            sleep_ms(100).await;
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
        // let (tx, rx) = mpsc::channel();
        // tokio::spawn(async move {
        let mut client = RvcdRpcClient::connect(format!("http://127.0.0.1:{}", MANAGER_PORT))
            .await
            .unwrap();
        loop {
            let r = if let Ok(data) = data.lock() {
                trace!("client sending info: {:?}", data);
                let paths = data.paths.clone();
                let port = data.port.clone();
                // drop(data);
                Some(RvcdManagedInfo {
                    client_port: port as u32,
                    paths,
                })
            } else {
                None
            };
            if let Some(r) = r {
                if client.client_info(r).await.is_err() {
                    break;
                }
            } else {
                break;
            }
            if *stop.lock().unwrap() {
                break;
            }
            sleep_ms(500).await;
        }
        // });
    }
}
