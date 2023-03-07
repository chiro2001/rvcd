#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[allow(unused_imports)]
use anyhow::Result;
use rvcd::app::RvcdApp;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;
use tracing::info;
#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;
#[cfg(not(target_arch = "wasm32"))]
use rvcd::manager::{RvcdRpcMessage, MANAGER_PORT};
use rvcd::manager::RvcdManagerMessage;
#[cfg(not(target_arch = "wasm32"))]
use rvcd::utils::sleep_ms;

/// Simple program to greet a person
#[cfg(not(target_arch = "wasm32"))]
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct RvcdArgs {
    /// Files to open
    file: Vec<String>,
    /// Input sources
    #[arg(short)]
    input: Vec<String>,
    /// Default source path
    #[arg(short, long, default_value = "")]
    src: String,
    /// Manager port
    #[arg(short, long, default_value_t = MANAGER_PORT)]
    port: u16,
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<()> {
    use rvcd::manager::RvcdManager;
    use tonic::transport::Server;

    let args = RvcdArgs::parse();

    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    // let native_options = eframe::NativeOptions::default();
    let native_options = eframe::NativeOptions {
        drag_and_drop_support: true,
        // initial_window_size: Some([1280.0, 1024.0].into()),
        // #[cfg(feature = "wgpu")]
        // renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    let (rpc_tx, rpc_rx) = mpsc::channel();
    let rpc_tx2 = rpc_tx.clone();
    let rpc_tx3 = rpc_tx.clone();
    let (manager_tx, manager_rx) = mpsc::channel();
    let src = args.src.clone();
    let gui = async move {
        eframe::run_native(
            "Rvcd",
            native_options,
            Box::new(|cc| {
                Box::new(RvcdApp::new(
                    cc,
                    rpc_rx,
                    rpc_tx2,
                    manager_tx,
                    if src.is_empty() { None } else { Some(src) },
                ))
            }),
        )
        .expect("gui panic!");
    };
    let rpc = async move {
        loop {
            let rpc_tx = rpc_tx.clone();
            let addr = format!("0.0.0.0:{}", args.port).parse().unwrap();
            info!("[Manager] rpc server at {}", addr);
            let reflection_service = tonic_reflection::server::Builder::configure()
                .register_encoded_file_descriptor_set(rvcd::rpc::RVCD_FILE_DESCRIPTOR_SET)
                .build()
                .unwrap();
            match Server::builder()
                .add_service(reflection_service)
                .add_service(rvcd::rpc::rvcd_rpc_server::RvcdRpcServer::new(
                    RvcdManager::new(rpc_tx),
                ))
                .serve(addr)
                .await
            {
                Ok(_) => {}
                Err(_) => {}
            }
            match manager_rx.try_recv() {
                Ok(msg) => {
                    match msg {
                        RvcdManagerMessage::Exit => break,
                    }
                },
                _ => {},
            }
            sleep_ms(1000).await;
        }
    };
    for file in args.file {
        rpc_tx3.send(RvcdRpcMessage::OpenWaveFile(file)).unwrap();
    }
    for source in args.input {
        rpc_tx3
            .send(RvcdRpcMessage::OpenSourceFile(source))
            .unwrap();
    }
    // pin_mut!(gui, rpc);
    // let _ = select(gui, rpc).await;
    tokio::spawn(rpc);
    gui.await;
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
            Box::new(|cc| Box::new(RvcdApp::new(cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}
