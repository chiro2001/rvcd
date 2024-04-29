#[cfg(not(target_arch = "wasm32"))]
use crate::code::editor::CodeEditor;
use crate::code::CodeEditorType;
use crate::files::preview_files_being_dropped;
use crate::frame_history::FrameHistory;
#[cfg(not(target_arch = "wasm32"))]
use crate::manager::RvcdRpcMessage;
use crate::manager::{RvcdExitMessage, RvcdManagerMessage};
use crate::rpc::{EventType, RvcdInputEvent};
use crate::run_mode::RunMode;
use crate::rvcd::State;
use crate::utils::sleep_ms;
use crate::verilog::VerilogGotoSource;
use crate::Rvcd;
use eframe::emath::Align;
use eframe::glow::Context;
use eframe::Frame;
use egui::{
    CentralPanel, ColorImage, DroppedFile, FontData, FontDefinitions, FontFamily, Id, Layout, Ui,
    Window,
};
use prost::Message;
use rust_i18n::locale;
use std::mem::MaybeUninit;
use std::sync::{mpsc, Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::debug;
// use tokio::sync::Mutex as FrameMutex;
use std::sync::Mutex as FrameMutex;
#[allow(unused_imports)]
use tracing::{error, info, warn};

#[derive(Debug)]
pub enum RvcdAppMessage {
    CreateCodeEditor(VerilogGotoSource),
}

pub const REPAINT_AFTER_SECONDS: f32 = 1.0;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct RvcdApp {
    pub apps: Vec<Rvcd>,
    pub app_now_id: Option<usize>,
    // #[serde(skip)]
    pub open_apps: Vec<(usize, bool)>,
    #[serde(skip)]
    pub run_mode: RunMode,
    #[serde(skip)]
    pub frame_history: FrameHistory,
    pub debug_panel: bool,
    pub sst_enabled: bool,
    pub locale: String,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub rpc_rx: Option<mpsc::Receiver<RvcdRpcMessage>>,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub manager_tx: Option<mpsc::Sender<RvcdManagerMessage>>,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub exit_tx: Option<mpsc::Sender<RvcdExitMessage>>,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub rpc_self_tx: Option<mpsc::Sender<RvcdRpcMessage>>,
    #[serde(skip)]
    pub loop_tx: Option<mpsc::Sender<RvcdAppMessage>>,
    #[serde(skip)]
    pub app_rx: Option<mpsc::Receiver<RvcdAppMessage>>,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub editors: Vec<CodeEditor>,
    pub code_editor: CodeEditorType,
    #[cfg(not(target_arch = "wasm32"))]
    pub default_source_dir: String,
    #[serde(skip)]
    pub global_frame: &'static mut FrameMutex<Option<Arc<ColorImage>>>,
    #[serde(skip)]
    pub extra_events: Option<Vec<egui::Event>>,
}

pub fn init() {
    info!("init rvcd app");
    unsafe {
        FRAME = MaybeUninit::new(FrameMutex::new(None));
    }
}

impl Default for RvcdApp {
    fn default() -> Self {
        Self {
            apps: vec![],
            app_now_id: None,
            open_apps: vec![],
            run_mode: Default::default(),
            frame_history: Default::default(),
            debug_panel: false,
            sst_enabled: true,
            locale: "".to_string(),
            #[cfg(not(target_arch = "wasm32"))]
            rpc_rx: None,
            #[cfg(not(target_arch = "wasm32"))]
            manager_tx: None,
            #[cfg(not(target_arch = "wasm32"))]
            exit_tx: None,
            #[cfg(not(target_arch = "wasm32"))]
            rpc_self_tx: None,
            loop_tx: None,
            app_rx: None,
            #[cfg(not(target_arch = "wasm32"))]
            editors: vec![],
            code_editor: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            default_source_dir: "".to_string(),
            #[allow(static_mut_refs)]
            global_frame: unsafe { FRAME.assume_init_mut() },
            extra_events: None,
        }
    }
}

impl RvcdApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        #[cfg(not(target_arch = "wasm32"))] rpc_rx: mpsc::Receiver<RvcdRpcMessage>,
        #[cfg(not(target_arch = "wasm32"))] rpc_tx: mpsc::Sender<RvcdRpcMessage>,
        #[cfg(not(target_arch = "wasm32"))] manager_tx: mpsc::Sender<RvcdManagerMessage>,
        #[cfg(not(target_arch = "wasm32"))] exit_tx: mpsc::Sender<RvcdExitMessage>,
        #[cfg(not(target_arch = "wasm32"))] default_source_dir: Option<String>,
    ) -> Self {
        // load chinese font
        let mut fonts = FontDefinitions::default();
        let font_name = "ali";
        fonts.font_data.insert(
            font_name.to_owned(),
            FontData::from_static(include_bytes!("../assets/Ali_Puhui_Medium.ttf")),
        );
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, font_name.to_owned());
        cc.egui_ctx.set_fonts(fonts);
        let mut def: RvcdApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(s) = default_source_dir {
            def.default_source_dir = s;
        }
        if def.locale.is_empty() {
            // detect locate
            // TODO: detect on windows
            // if let Ok(lang) = std::env::var("LANG") {
            //     let locale = lang
            //         .as_str()
            //         .replace("_", "-")
            //         .as_str()
            //         .replace(".UTF-8", "");
            //     info!("auto detected locale: {}", locale);
            //     if available_locales().iter().any(|x| x.to_string() == locale) {
            //         rust_i18n::set_locale(locale.as_str());
            //         def.locale = locale;
            //     }
            // } else {
            rust_i18n::set_locale("zh-CN");
            // }
        } else {
            rust_i18n::set_locale(def.locale.as_str());
        }
        if def.apps.is_empty() {
            def.apps = vec![Rvcd::new(0)];
        }
        def.open_apps = def.apps.iter().map(|x| (x.id, true)).collect();
        def.apps = def
            .apps
            .into_iter()
            .map(|mut a| {
                a.init();
                a
            })
            .collect();
        #[cfg(not(target_arch = "wasm32"))]
        {
            def.rpc_rx = Some(rpc_rx);
            def.rpc_self_tx = Some(rpc_tx);
            def.manager_tx = Some(manager_tx);
            def.exit_tx = Some(exit_tx);
        }
        let (tx, rx) = mpsc::channel();
        def.apps
            .iter_mut()
            .for_each(|app| app.set_upper_tx(tx.clone()));
        def.loop_tx = Some(tx);
        def.app_rx = Some(rx);
        def
    }
    pub fn debug_panel(&mut self, ui: &mut Ui) {
        let run_mode = &mut self.run_mode;
        ui.label(t!("debug.mode"));
        ui.radio_value(run_mode, RunMode::Reactive, t!("debug.reactive.text"))
            .on_hover_text(t!("debug.reactive.hover"));
        ui.radio_value(run_mode, RunMode::Continuous, t!("debug.continuous.text"))
            .on_hover_text(t!("debug.continuous.hover"));
        if self.run_mode == RunMode::Continuous {
            ui.label(t!(
                "debug.fps",
                fps = format!("{:.1}", self.frame_history.fps()).as_str()
            ));
        } else {
            self.frame_history.ui(ui);
        }
        #[cfg(debug_assertions)]
        {
            let mut debug_on_hover = ui.ctx().debug_on_hover();
            ui.checkbox(
                &mut debug_on_hover,
                format!("🐛 {}", t!("debug.debug_mode")),
            );
            ui.ctx().set_debug_on_hover(debug_on_hover);
        }
        ui.horizontal(|ui| {
            if let Some(id) = self.app_now_id {
                if ui.button(t!("debug.reset_this_rvcd")).clicked() {
                    if let Some(app) = self.apps.get_mut(id) {
                        app.reset();
                    }
                }
            }
            if ui.button(t!("debug.reset_app")).clicked() {
                for app in &mut self.apps {
                    app.on_exit();
                }
                self.apps.clear();
            }
        });
        ui.horizontal(|ui| {
            if ui
                .button(t!("debug.reset_egui.text"))
                .on_hover_text(t!("debug.reset_egui.hover"))
                .clicked()
            {
                ui.ctx().memory_mut(|mem| *mem = Default::default());
            }

            if ui.button(t!("debug.reset_everything")).clicked() {
                ui.ctx().memory_mut(|mem| *mem = Default::default());
            }
        });
        egui::warn_if_debug_build(ui);
    }
    fn new_id(&self) -> usize {
        self.apps
            .iter()
            .map(|x| x.id)
            .max()
            .map(|x| x + 1)
            .unwrap_or(0)
    }
    fn new_window(&mut self, maximize: bool) -> usize {
        let id = self.new_id();
        let mut n = Rvcd::new(id);
        n.init();
        if let Some(tx) = self.loop_tx.clone() {
            n.set_upper_tx(tx);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if !self.default_source_dir.is_empty() {
            n.source_dir = self.default_source_dir.clone();
        }
        self.apps.push(n);
        self.open_apps.push((id, true));
        if maximize {
            self.app_now_id = Some(id);
        }
        id
    }
    pub fn close_all(&mut self) {
        info!("close all apps");
        self.apps.iter_mut().for_each(|app| app.on_exit());
        self.app_now_id = None;
        self.open_apps.clear();
        self.apps.clear();
    }
    pub fn message_handler(&mut self, msg: RvcdAppMessage) {
        info!("app message handle: {:?}", msg);
        match msg {
            #[cfg(not(target_arch = "wasm32"))]
            RvcdAppMessage::CreateCodeEditor(p) => match self.code_editor {
                CodeEditorType::Internal => {
                    if let Some(editor) =
                        self.editors.iter_mut().find(|x| x.file == p.file.as_str())
                    {
                        editor.goto = Some(p.location);
                    } else {
                        self.editors
                            .push(CodeEditor::new(p.file.as_str(), Some(p.location)));
                    }
                }
                CodeEditorType::Scaleda => {
                    tokio::spawn(async move {
                        match crate::rpc::scaleda_rpc_client::ScaledaRpcClient::connect(
                            "http://127.0.0.1:4151",
                        )
                        .await
                        {
                            Ok(mut client) => {
                                if let Err(e) = client
                                    .goto_source(crate::rpc::ScaledaGotoSource {
                                        file: p.file,
                                        path: p.path,
                                        line: p.location.line as u32,
                                        column: p.location.column as u32,
                                    })
                                    .await
                                {
                                    warn!("cannot call scaleda rpc: {:?}", e);
                                }
                                client
                                    .ping(crate::rpc::ScaledaEmpty::default())
                                    .await
                                    .unwrap();
                            }
                            Err(e) => {
                                warn!("cannot reach scaleda: {}", e);
                            }
                        }
                    });
                }
                CodeEditorType::VsCode => {
                    let loc = p.location.clone();
                    let args = if loc.line > 0 && loc.column > 0 {
                        // on linux, format not right
                        let s = format!("{}:{}:{}", p.file, loc.line, loc.column);
                        vec!["-g".to_string(), s]
                    } else {
                        vec![p.file]
                    };
                    let executable = if cfg!(target_os = "windows") {
                        "code.exe"
                    } else {
                        "code"
                    };
                    info!("call command: {} {:?}", executable, args);
                    match std::process::Command::new(executable).args(args).output() {
                        Ok(output) => {
                            info!("code ret: {:?}", output.status);
                            info!("code out: {:?}", String::from_utf8(output.stdout));
                            info!("code out: {:?}", String::from_utf8(output.stderr));
                        }
                        Err(e) => {
                            error!("cannot execute vscode: {}", e);
                        }
                    }
                }
            },
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }
    pub async fn frame_buffer_tcp_server(port: u16, tx: mpsc::Sender<RvcdRpcMessage>) {
        loop {
            let tx = tx.clone();
            if let Err(e) = Self::frame_buffer_tcp_server_internal(port, tx).await {
                warn!("frame buffer tcp server error: {:?}", e);
                sleep_ms(1000).await;
            } else {
                break;
            }
        }
    }
    pub async fn frame_buffer_tcp_server_internal(
        port: u16,
        tx: mpsc::Sender<RvcdRpcMessage>,
    ) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        info!("FB Server started on port {}", port);
        let tx = tx.clone();
        while let Ok((socket, _)) = listener.accept().await {
            info!(
                "New FB tcp client connected: {:?}",
                socket.peer_addr().unwrap()
            );
            let tx = tx.clone();
            let tx2 = tx.clone();
            let (reader, writer) = socket.into_split();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(writer, tx).await {
                    error!("Error handling connection: {:?}", e);
                }
            });
            tokio::spawn(async move {
                if let Err(e) = handle_stream_input(reader, tx2).await {
                    error!("Error handling connection: {:?}", e);
                }
            });
        }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    pub async fn frame_buffer_unix_server(path: &str, tx: mpsc::Sender<RvcdRpcMessage>) {
        // try to remove old socket
        let _ = std::fs::remove_file(path);
        loop {
            let tx = tx.clone();
            if let Err(e) = Self::frame_buffer_unix_server_internal(path, tx).await {
                warn!("frame buffer unix server error: {:?}", e);
                sleep_ms(1000).await;
            } else {
                break;
            }
        }
    }
    #[cfg(target_os = "linux")]
    pub async fn frame_buffer_unix_server_internal(
        path: &str,
        tx: mpsc::Sender<RvcdRpcMessage>,
    ) -> anyhow::Result<()> {
        let listener = tokio::net::UnixListener::bind(path)?;
        info!("FB Server started on path {}", path);

        let tx = tx.clone();
        while let Ok((socket, _)) = listener.accept().await {
            info!(
                "New FB unix client connected: {:?}",
                socket.peer_addr().unwrap()
            );
            let tx = tx.clone();
            let tx2 = tx.clone();
            let (reader, writer) = socket.into_split();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(writer, tx).await {
                    error!("Error handling connection: {:?}", e);
                }
            });
            tokio::spawn(async move {
                if let Err(e) = handle_stream_input(reader, tx2).await {
                    error!("Error handling connection: {:?}", e);
                }
            });
        }
        Ok(())
    }
}

async fn handle_stream_input<S: AsyncReadExt + std::marker::Unpin>(
    mut reader: S,
    tx: mpsc::Sender<RvcdRpcMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 32];
    loop {
        let len = reader.read_u32().await? as usize;
        buf.iter_mut().for_each(|x| *x = 0);
        let mut n = 0;
        while n < len {
            let r = reader.read(&mut buf[n..]).await;
            match r {
                Ok(0) => {
                    return Ok(());
                }
                Ok(size) => {
                    n += size;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
        let string = buf[..len]
            .iter()
            .map(|x| format!("{:02X}", x))
            .collect::<Vec<String>>()
            .join(" ");
        debug!("read data: {}", string);
        match RvcdInputEvent::decode(&buf[..len]) {
            Ok(event) => {
                if event.r#type() != EventType::None {
                    if let Err(e) = tx.send(RvcdRpcMessage::InputEvent(event)) {
                        error!("cannot send input event: {:?}", e);
                    }
                } else {
                    warn!("invalid input event: {:?}", event)
                }
            }
            Err(e) => {
                error!("cannot decode input event! {:?}", e);
            }
        }
        sleep_ms(1).await;
    }
}

async fn handle_connection<S: AsyncWriteExt + std::marker::Unpin>(
    mut writer: S,
    tx: mpsc::Sender<RvcdRpcMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // request frame
        tx.send(RvcdRpcMessage::RequestFrame)?;
        let frame_data = unsafe { FRAME.assume_init_mut() }.lock()?.clone();
        if let Some(f) = frame_data {
            let (width, height) = (f.size[0] as u16, f.size[1] as u16);
            let mut data = vec![0u8; f.pixels.len() * 2];
            // copy data, can be optimized...
            data.iter_mut()
                // .zip(f.pixels.iter().flat_map(|p| p.to_array()))
                .zip(f.pixels.iter().flat_map(|p| {
                    let (r, g, b, _a) = p.to_tuple();
                    // [a, r, g, b]
                    // convert to rgb565
                    let red5 = (r as u16 >> 3) & 0x1F;
                    let green6 = (g as u16 >> 2) & 0x3F;
                    let blue5 = (b as u16 >> 3) & 0x1F;
                    let rgb565: u16 = (red5 << 11) | (green6 << 5) | blue5;
                    rgb565.to_be_bytes()
                }))
                .for_each(|(d, s)| *d = s);
            writer.write_u16(width).await?;
            writer.write_u16(height).await?;
            writer.write_all(&data).await?;
            // socket.flush().await?;
        } else {
            debug!("no frame data");
        }
        sleep_ms(1).await;
    }
    // Ok(())
}

impl eframe::App for RvcdApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
        self.frame_history
            .on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);
        match self.run_mode {
            RunMode::Continuous => {
                // Tell the backend to repaint as soon as possible
                ctx.request_repaint();
            }
            RunMode::Reactive => {
                // let the computer rest for a bit
                ctx.request_repaint_after(std::time::Duration::from_secs_f32(
                    REPAINT_AFTER_SECONDS,
                ));
            }
        }
        egui::TopBottomPanel::top("global_menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if let Some(id) = self.app_now_id {
                    if let Some(app) = self.apps.iter_mut().find(|app| app.id == id) {
                        app.menubar(ui, true);
                    }
                }
                ui.menu_button("Language", |ui| {
                    let locales = rust_i18n::available_locales!();
                    let locale_now = locale();
                    let mut set_locale = |source: bool, locale: &str| {
                        let mut source = source;
                        if ui.checkbox(&mut source, locale.to_string()).clicked() {
                            rust_i18n::set_locale(locale);
                            self.locale = locale.to_string();
                            ui.close_menu();
                        }
                    };
                    for locale in locales {
                        set_locale(locale.to_string() == locale_now.to_string(), locale);
                    }
                });
                ui.menu_button("Editor", |ui| {
                    use CodeEditorType::*;
                    let editor_types = vec![Scaleda, Internal, VsCode];
                    for e in editor_types {
                        let mut eq = e == self.code_editor;
                        if ui.checkbox(&mut eq, e.to_string()).clicked() {
                            self.code_editor = e;
                            ui.close_menu();
                        }
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button(t!("menu.quit")).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    ui.add_enabled_ui(self.apps.len() > 0, |ui| {
                        if ui.button(t!("menu.close_all")).clicked() {
                            // if self.apps.len() > 1 {
                            self.close_all();
                            // } else {
                            //     if let Some(app) = self.apps.first_mut() {
                            //         app.reset();
                            //     }
                            // }
                        }
                    });
                    if ui.button(t!("menu.new_window")).clicked() {
                        self.new_window(false);
                    }
                    if self.app_now_id.is_some() {
                        if ui.button(t!("menu.minimize")).clicked() {
                            self.app_now_id = None;
                        }
                    } else {
                        ui.add_enabled_ui(self.apps.len() == 1, |ui| {
                            if ui.button(t!("menu.maximize")).clicked() {
                                if let Some(app) = self.apps.get(0) {
                                    self.app_now_id = Some(app.id);
                                }
                            }
                        });
                    }
                    ui.checkbox(&mut self.debug_panel, t!("menu.debug_panel"));
                    ui.checkbox(&mut self.sst_enabled, t!("menu.sst"));
                    // ui.label(format!("apps: {:?}", self.apps));
                });
            });
        });
        if self.debug_panel {
            egui::SidePanel::left("debug_panel").show(ctx, |ui| {
                self.debug_panel(ui);
            });
        }
        let app_now_id = self.app_now_id;
        let mut show_app_in_window = |app: &mut Rvcd, ctx: &egui::Context| {
            let open_app = self.open_apps.iter_mut().find(|x| x.0 == app.id);
            if let Some((id, open)) = open_app {
                Window::new(app.title())
                    .min_height(200.0)
                    .default_width(ctx.used_size().x / 2.0)
                    .vscroll(false)
                    .open(open)
                    .id(Id::new(*id))
                    .title_bar(true)
                    .show(ctx, |ui| {
                        app.update(ui, self.sst_enabled, false, || self.app_now_id = Some(*id));
                    });
            }
        };
        let mut will_minimum_this = false;
        if let Some(id) = app_now_id {
            if let Some(app) = self.apps.get_mut(id) {
                CentralPanel::default()
                    .frame(
                        egui::Frame::default()
                            .inner_margin(0.0)
                            .outer_margin(0.0)
                            .rounding(0.0),
                    )
                    .show(ctx, |ui| {
                        // when testing frame sending, use spinner to auto update
                        // ui.spinner();
                        // ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        // ui.spacing_mut().window_margin = egui::Margin::same(0.0);
                        ui.visuals_mut().window_rounding = egui::Rounding::same(0.0);
                        app.update(ui, self.sst_enabled, true, || {
                            will_minimum_this = true;
                        });
                    });
                for app in &mut self.apps {
                    if id != app.id {
                        show_app_in_window(app, ctx);
                    }
                }
            }
        } else {
            for app in &mut self.apps {
                show_app_in_window(app, ctx);
            }
        }
        if will_minimum_this {
            self.app_now_id = None;
        }
        // remove closed windows
        let to_removes = self
            .open_apps
            .iter()
            .filter(|x| !x.1)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        if let Some(id) = self.app_now_id {
            if to_removes.iter().any(|i| *i == id) {
                self.app_now_id = None;
            }
        }
        self.apps.iter_mut().for_each(|app| {
            if to_removes.iter().any(|id| *id == app.id) {
                app.on_exit();
            }
        });
        let to_remove_indexes = self
            .apps
            .iter()
            .enumerate()
            .filter(|x| to_removes.iter().any(|id| x.1.id == *id))
            .map(|x| x.0)
            .collect::<Vec<_>>();
        for i in to_remove_indexes {
            if i < self.apps.len() {
                let removed = self.apps.remove(i);
                // let removed = self.apps.get(i).unwrap();
                self.open_apps.retain(|x| x.0 != removed.id);
                info!(
                    "remove rvcd: id={}, open_apps: {:?}",
                    removed.id, self.open_apps
                );
            }
        }
        preview_files_being_dropped(ctx);
        // FIXME: wasm target cannot handle multi files
        for file in &ctx.input(|i| i.raw.dropped_files.clone()) {
            let file: &DroppedFile = file;
            let is_file = file.path.clone().map(|x| x.is_file()).unwrap_or(false);
            let has_maximum_window = self.app_now_id.is_some();
            match self.app_now_id.and_then(|id| {
                self.apps
                    .iter_mut()
                    .find(|a| a.id == id && a.state == State::Idle)
            }) {
                Some(app) => app.handle_dropping_file(file),
                None => {
                    if is_file {
                        let id = self.new_window(!has_maximum_window);
                        if let Some(app) = self.apps.iter_mut().find(|a| a.id == id) {
                            app.handle_dropping_file(file);
                        }
                    } else {
                        if let Some(app) = self.apps.first_mut() {
                            app.handle_dropping_file(file);
                        } else {
                            let id = self.new_window(!has_maximum_window);
                            if let Some(app) = self.apps.iter_mut().find(|a| a.id == id) {
                                app.handle_dropping_file(file);
                            }
                        }
                    }
                }
            };
        }
        // if empty, create new main window
        if self.apps.is_empty() {
            self.new_window(true);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut messages = vec![];
            if let Some(rx) = &self.rpc_rx {
                while let Ok(r) = rx.try_recv() {
                    messages.push(r);
                }
            }
            let mut handled_app = vec![];
            let mut frame_requested = false;
            for message in &messages {
                match message {
                    RvcdRpcMessage::RequestFrame | RvcdRpcMessage::InputEvent(_) => {}
                    _ => info!("rvcd app handle rpc message: {:?}", message),
                }
                match message {
                    RvcdRpcMessage::GotoPath(g) => {
                        if g.file.is_empty() {
                            // goto signal in file that have opened
                            for app in &mut self.apps {
                                app.handle_rpc_message(message.clone());
                            }
                        } else {
                            // create new if no app opened file
                            let mut retry = false;
                            let mut handle = |app: &mut Rvcd| {
                                if app.handle_rpc_message(message.clone()) {
                                    retry = true;
                                }
                            };
                            if let Some(app) = self.apps.iter_mut().find(|x| x.filepath == g.file) {
                                handle(app);
                            } else {
                                if let Some(app) =
                                    self.apps.iter_mut().find(|x| x.state == State::Idle)
                                {
                                    info!("goto path: use idle app");
                                    retry = app.handle_rpc_message(RvcdRpcMessage::OpenWaveFile(
                                        g.file.clone(),
                                    ));
                                } else {
                                    info!("goto path: create app");
                                    let id = self.new_window(self.app_now_id.is_none());
                                    if let Some(app) = self.apps.iter_mut().find(|x| x.id == id) {
                                        app.handle_rpc_message(RvcdRpcMessage::OpenWaveFile(
                                            g.file.clone(),
                                        ));
                                    }
                                    retry = true;
                                }
                            }
                            if retry {
                                info!("do retry for: {:?}", g);
                                if let Some(tx) = &self.rpc_self_tx {
                                    tx.send(message.clone()).unwrap();
                                }
                            }
                        }
                    }
                    RvcdRpcMessage::OpenWaveFile(path) => {
                        let mut ok = false;
                        let send = |app: &Rvcd| {
                            if let Some(channel) = &app.channel {
                                channel
                                    .tx
                                    .send(crate::message::RvcdMsg::FileOpen(rfd::FileHandle::from(
                                        std::path::PathBuf::from(path.as_str()),
                                    )))
                                    .unwrap();
                                info!("send app<{}> file {}", app.id, path);
                            }
                        };
                        if let Some(id) = self.app_now_id {
                            if !handled_app.contains(&id) {
                                if let Some(app) = self
                                    .apps
                                    .iter_mut()
                                    .find(|app| app.id == id && app.state == State::Idle)
                                {
                                    info!("use maximized app to handle {}", path);
                                    send(app);
                                    handled_app.push(app.id);
                                    ok = true;
                                }
                            }
                        }
                        if !ok {
                            if !self.apps.iter().any(|x| {
                                x.filepath.as_str() == path.as_str() && x.state == State::Idle
                            }) {
                                info!("create new window to handle {}", path);
                                let has_maximum_window = self.app_now_id.is_some();
                                let id = self.new_window(!has_maximum_window);
                                if let Some(app) = self.apps.iter().find(|a| a.id == id) {
                                    send(app);
                                    handled_app.push(app.id);
                                }
                            }
                        }
                    }
                    RvcdRpcMessage::OpenSourceFile(path) => {
                        for app in &mut self.apps {
                            app.handle_rpc_message(RvcdRpcMessage::OpenSourceFile(
                                path.to_string(),
                            ));
                        }
                    }
                    RvcdRpcMessage::OpenSourceDir(path) => {
                        for app in &mut self.apps {
                            app.handle_rpc_message(RvcdRpcMessage::OpenSourceDir(path.to_string()));
                        }
                    }
                    RvcdRpcMessage::RequestFrame => {
                        if !frame_requested {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
                            frame_requested = true;
                        }
                    }
                    RvcdRpcMessage::InputEvent(event) => {
                        debug!("recv input event: {:?}", event);
                        match event.r#type() {
                            crate::rpc::EventType::None => {}
                            crate::rpc::EventType::Resize => {
                                let width = event.x as f32 / ctx.pixels_per_point();
                                let height = event.y as f32 / ctx.pixels_per_point();
                                debug!("resize: {}x{}", width, height);
                                if width < 20.0 || height < 20.0 {
                                    warn!("size too small, ignore");
                                } else {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                        egui::vec2(width, height),
                                    ));
                                }
                            }
                            crate::rpc::EventType::PointerMovement => {
                                let events = self.extra_events.get_or_insert(vec![]);
                                let pos = egui::pos2(
                                    event.x as f32 / ctx.pixels_per_point(),
                                    event.y as f32 / ctx.pixels_per_point(),
                                );
                                events.push(egui::Event::PointerMoved(pos));
                            }
                            crate::rpc::EventType::Wheel => {
                                let events = self.extra_events.get_or_insert(vec![]);
                                events.push(egui::Event::Scroll(egui::vec2(
                                    event.x as f32,
                                    event.y as f32,
                                )));
                            }
                            crate::rpc::EventType::Click => {
                                let events = self.extra_events.get_or_insert(vec![]);
                                let pos = egui::pos2(
                                    event.x as f32 / ctx.pixels_per_point(),
                                    event.y as f32 / ctx.pixels_per_point(),
                                );
                                events.push(egui::Event::PointerButton {
                                    pos,
                                    button: match event.button {
                                        1 => egui::PointerButton::Primary,
                                        3 => egui::PointerButton::Secondary,
                                        2 => egui::PointerButton::Middle,
                                        _ => egui::PointerButton::Primary,
                                    },
                                    pressed: event.data != 0,
                                    modifiers: Default::default(),
                                });
                            }
                            EventType::Zoom => {
                                let events = self.extra_events.get_or_insert(vec![]);
                                let zoom = event.data as f32 / 1000.0;
                                events.push(egui::Event::Zoom(zoom));
                            }
                            EventType::Visible => {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(
                                    event.data != 0,
                                ));
                            }
                        }
                    }
                }
            }
        }
        let mut messages = vec![];
        if let Some(rx) = &self.app_rx {
            while let Ok(r) = rx.try_recv() {
                messages.push(r);
            }
        }
        for message in messages {
            self.message_handler(message);
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.editors.retain(|x| x.open);
        #[cfg(not(target_arch = "wasm32"))]
        for editor in &mut self.editors {
            editor.ui(ctx);
        }

        // handle screen shot
        ctx.input(|i| {
            for event in i.events.iter() {
                match event {
                    egui::Event::Screenshot {
                        viewport_id: _,
                        image,
                    } => {
                        self.global_frame.lock().unwrap().replace(image.clone());
                    }
                    _ => {}
                }
            }
        });
    }
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn on_exit(&mut self, _gl: Option<&Context>) {
        // self.close_all();
        if let Some(tx) = &self.exit_tx {
            tx.send(RvcdExitMessage::Exit).unwrap();
        }
        // close all but save data
        for app in &mut self.apps {
            app.on_exit();
        }
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let events = self.extra_events.take();
        if let Some(mut events) = events {
            raw_input.events.append(&mut events)
        }
    }
}

pub static mut FRAME: MaybeUninit<FrameMutex<Option<Arc<ColorImage>>>> = MaybeUninit::uninit();
