#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[allow(unused_imports)]
use anyhow::Result;
use tracing::info;
use rvcd::Rvcd;

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<()> {
    use futures::future::{join, select};
    use futures::pin_mut;
    use tonic::transport::Server;
    use rvcd::server::server::rvcd_rpc_server::RvcdRpcServer;
    use rvcd::server::RvcdRemote;

    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let native_options = eframe::NativeOptions::default();
    let gui = async move {
        eframe::run_native(
            "Rvcd",
            native_options,
            Box::new(|cc| Box::new(Rvcd::new(cc))),
        );
    };
    let rpc = async move {
        let addr = "[::1]:50051".parse().unwrap();
        info!("starting rpc server at {}", addr);
        Server::builder()
            .add_service(RvcdRpcServer::new(RvcdRemote::default()))
            .serve(addr)
            .await
            .unwrap();
    };
    pin_mut!(gui, rpc);
    let _ = select(gui, rpc).await;
    Ok(())
}

// when compiling to web using trunk.
#[cfg(target_arch = "wasm32")]
fn main() {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();

    info!("starting rvcd");

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "the_canvas_id", // hardcode it
            web_options,
            Box::new(|cc| Box::new(Rvcd::new(cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}
