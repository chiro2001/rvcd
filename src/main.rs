#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[allow(unused_imports)]
use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;
use rvcd::app::RvcdApp;
#[cfg(not(target_arch = "wasm32"))]
use rvcd::manager::{RvcdRpcMessage, MANAGER_PORT};
#[cfg(not(target_arch = "wasm32"))]
use rvcd::utils::sleep_ms;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;
use tracing::info;

#[cfg(all(target_arch = "x86_64", feature = "jemalloc"))]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

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
    #[arg(long, default_value_t = false)]
    hidden: bool,
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
// #[tokio::main]
fn main() -> Result<()> {
    use rvcd::{
        app,
        manager::{RvcdExitMessage, RvcdManager},
        rpc::RvcdInputEvent,
    };
    use std::sync::{Arc, Mutex};
    use tokio::runtime::Builder;
    use tonic::transport::Server;
    use tracing::error;

    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        let args = RvcdArgs::parse();

        // Log to stdout (if you run with `RUST_LOG=debug`).
        tracing_subscriber::fmt::init();

        app::init();

        // let native_options = eframe::NativeOptions::default();
        let native_options = eframe::NativeOptions {
            // drag_and_drop_support: true,
            // depth_buffer: 3,
            // initial_window_size: Some([1280.0, 1024.0].into()),
            // #[cfg(feature = "wgpu")]
            // renderer: eframe::Renderer::Wgpu,
            ..Default::default()
        };
        let (rpc_tx, rpc_rx) = mpsc::channel();
        let rpc_tx2 = rpc_tx.clone();
        let rpc_tx3 = rpc_tx.clone();
        let rpc_tx4 = rpc_tx.clone();
        let rpc_tx5 = rpc_tx.clone();
        let (manager_tx, manager_rx) = mpsc::channel();
        let (exit_tx, exit_rx) = mpsc::channel();
        let exit_tx2 = exit_tx.clone();
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
                        exit_tx2,
                        if src.is_empty() { None } else { Some(src) },
                    ))
                }),
            )
            .expect("gui panic!");
        };
        let rpc = async move {
            let manager_rx = Arc::new(Mutex::new(manager_rx));
            loop {
                let rpc_tx = rpc_tx.clone();
                let exit_tx = exit_tx.clone();
                let addr = format!("0.0.0.0:{}", args.port).parse().unwrap();
                info!("[Manager] rpc server at {}", addr);
                let reflection_service = tonic_reflection::server::Builder::configure()
                    .register_encoded_file_descriptor_set(rvcd::rpc::RVCD_FILE_DESCRIPTOR_SET)
                    .build()
                    .unwrap();
                match Server::builder()
                    .add_service(reflection_service)
                    .add_service(rvcd::rpc::rvcd_rpc_server::RvcdRpcServer::new(
                        RvcdManager::new(rpc_tx, manager_rx.clone(), exit_tx),
                    ))
                    .serve(addr)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => error!("cannot run rpc: {:?}", e),
                }
                match exit_rx.try_recv() {
                    Ok(msg) => match msg {
                        RvcdExitMessage::Exit => break,
                    },
                    _ => {}
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
        tokio::spawn(rpc);
        tokio::spawn(RvcdApp::frame_buffer_tcp_server(
            rvcd::manager::DISP_PORT,
            rpc_tx4,
        ));
        if args.hidden {
            let mut event = RvcdInputEvent::default();
            event.set_type(rvcd::rpc::EventType::Visible);
            event.data = 0;
            rpc_tx5.send(RvcdRpcMessage::InputEvent(event)).unwrap();
        }
        #[cfg(target_os = "linux")]
        tokio::spawn(RvcdApp::frame_buffer_unix_server(
            rvcd::manager::UNIX_FB_PATH,
            rpc_tx5,
        ));
        gui.await;
    });
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
        let runner = eframe::WebRunner::new();
        runner.start(
            "the_canvas_id", // hardcode it
            web_options,
            Box::new(|cc| Box::new(RvcdApp::new(cc))),
        )
        .await
        .expect("failed to start eframe");

    });
}
