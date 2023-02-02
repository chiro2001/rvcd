use crate::frame_history::FrameHistory;
use crate::message::{RvcdChannel, RvcdMsg};
use crate::run_mode::RunMode;
use crate::service::Service;
use crate::tree_view::{TreeAction, TreeView};
use crate::utils::execute;
use crate::view::signal::SignalView;
use crate::view::WaveView;
use crate::wave::{Wave, WaveSignalInfo, WaveTreeNode};
use eframe::emath::Align;
use egui::{DroppedFile, Layout, ScrollArea, Sense, Ui};
use egui_toast::{ToastOptions, Toasts};
use rfd::FileHandle;
#[allow(unused_imports)]
use std::path::PathBuf;
use std::sync::mpsc;
use tracing::info;

#[derive(serde::Deserialize, serde::Serialize, Default, PartialEq, Debug)]
pub enum State {
    #[default]
    Idle,
    Loading,
    Working,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct Rvcd {
    /// File loading state
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub state: State,
    #[cfg(target_arch = "wasm32")]
    #[serde(skip)]
    pub state: State,
    /// ui <- -> service
    #[serde(skip)]
    pub channel: Option<RvcdChannel>,
    /// Loaded file path.
    ///
    /// **Only available on native**
    pub filepath: String,
    #[serde(skip)]
    pub load_progress: f32,
    /// Displaying signals in the tree leaves
    #[serde(skip)]
    pub signal_leaves: Vec<WaveSignalInfo>,
    #[serde(skip)]
    pub wave: Option<Wave>,
    #[cfg(not(target_arch = "wasm32"))]
    pub view: WaveView,
    #[cfg(target_arch = "wasm32")]
    #[serde(skip)]
    pub view: WaveView,

    #[serde(skip)]
    pub toasts: Toasts,
    #[serde(skip)]
    pub repaint_after_seconds: f32,
    #[serde(skip)]
    pub run_mode: RunMode,
    #[serde(skip)]
    pub frame_history: FrameHistory,
    pub debug_panel: bool,

    #[cfg(target_arch = "wasm32")]
    #[serde(skip)]
    pub file: Option<FileHandle>
}

impl Default for Rvcd {
    fn default() -> Self {
        Self {
            state: State::default(),
            channel: None,
            filepath: "".to_string(),
            load_progress: 0.0,
            signal_leaves: vec![],
            wave: None,
            view: Default::default(),
            toasts: Toasts::new()
                .direction(egui::Direction::BottomUp)
                .align_to_end(true),
            repaint_after_seconds: 1.0,
            run_mode: Default::default(),
            frame_history: Default::default(),
            debug_panel: false,
            #[cfg(target_arch = "wasm32")]
            file: None,
        }
    }
}

impl Rvcd {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (channel_req_tx, channel_req_rx) = mpsc::channel();
        let (channel_resp_tx, channel_resp_rx) = mpsc::channel();

        // launch service
        Service::start(RvcdChannel {
            tx: channel_resp_tx.clone(),
            rx: channel_req_rx,
        });

        let mut def = if let Some(storage) = cc.storage {
            let def: Rvcd = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            // auto open file
            // let filepath = "data/cpu_ila_commit.vcd";
            #[cfg(not(target_arch = "wasm32"))]
            {
                let filepath = &def.filepath;
                info!("last file: {}", filepath);
                if !filepath.is_empty() {
                    channel_req_tx
                        .send(RvcdMsg::FileOpen(rfd::FileHandle::from(
                            std::path::PathBuf::from(filepath),
                        )))
                        .unwrap();
                }
            }
            info!("last loaded {} signals", def.view.signals.len());
            def
        } else {
            Default::default()
        };
        def.view.set_tx(channel_resp_tx);
        Self {
            channel: Some(RvcdChannel {
                tx: channel_req_tx,
                rx: channel_resp_rx,
            }),
            ..def
        }
    }
    /// Add signal to view.
    ///
    /// * `repetitive`: is allow repetitive
    fn signal_clicked(&mut self, id: u64, repetitive: bool) {
        if repetitive || !self.view.signals.iter().any(|x| x.s.id == id) {
            if let Some(wave) = &self.wave {
                self.view.signals.push(SignalView::from_id(id, &wave.info));
            }
        }
    }
    pub fn sidebar(&mut self, ui: &mut Ui) {
        egui::TopBottomPanel::bottom("signal_leaf")
            // .min_height(100.0)
            .max_height(400.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.with_layout(
                        Layout::top_down(Align::LEFT).with_cross_justify(true),
                        |ui| {
                            let mut clicked_id = 0;
                            let mut clicked = false;
                            for s in self.signal_leaves.iter() {
                                let response =
                                    ui.add(egui::Label::new(s.to_string()).sense(Sense::click()));
                                if response.double_clicked() {
                                    clicked_id = s.id;
                                    clicked = true;
                                }
                            }
                            if clicked {
                                self.signal_clicked(clicked_id, true);
                            }
                        },
                    );
                });
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                ui.with_layout(
                    Layout::left_to_right(Align::LEFT).with_cross_justify(false),
                    |ui| {
                        // ScrollArea::vertical().show(ui, |ui| {
                        if let Some(wave) = &self.wave {
                            match TreeView::default().ui(ui, wave.info.tree.root()) {
                                TreeAction::None => {}
                                TreeAction::AddSignal(node) => match node {
                                    WaveTreeNode::WaveVar(d) => {
                                        self.signal_clicked(d.id, true);
                                    }
                                    _ => {}
                                },
                                TreeAction::SelectScope(nodes) => {
                                    self.signal_leaves = nodes
                                        .into_iter()
                                        .map(|node| match node {
                                            WaveTreeNode::WaveVar(v) => Some(v),
                                            _ => None,
                                        })
                                        .filter(|x| x.is_some())
                                        .map(|x| x.unwrap())
                                        .collect();
                                }
                                TreeAction::AddSignals(nodes) => {
                                    for node in nodes {
                                        match node {
                                            WaveTreeNode::WaveVar(d) => {
                                                self.signal_clicked(d.id, false);
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        } else {
                            ui.centered_and_justified(|ui| ui.label("No file loaded"));
                        }
                        // });
                    },
                );
            });
        });
    }
    pub fn wave_panel(&mut self, ui: &mut Ui) {
        if let Some(wave) = &self.wave {
            self.view.panel(ui, &wave);
        } else {
            ui.centered_and_justified(|ui| {
                ui.heading("No file loaded. Drag file here or open file in menu.");
            });
        }
    }
    pub fn message_handler(&mut self, msg: RvcdMsg) {
        match msg {
            RvcdMsg::LoadingProgress(_) => {}
            _ => {
                info!(
                    "ui handle msg: {:?}; signals: {}",
                    msg,
                    self.view.signals.len()
                );
            }
        };
        match msg {
            RvcdMsg::UpdateWave(wave) => {
                info!("ui recv wave: {}", wave);
                self.wave = Some(wave);
                self.signal_leaves.clear();
                if let Some(wave) = &self.wave {
                    self.view.signals_clean_unavailable(&wave.info);
                }
                // FIXME: update range
                self.state = State::Working;
            }
            RvcdMsg::FileOpen(_file) => {}
            RvcdMsg::Reload => {
                self.reload();
            }
            RvcdMsg::Notification(toast) => {
                self.toasts.add(toast);
            }
            RvcdMsg::FileOpenFailed => {
                let text = "Open file failed!";
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.toasts.error(text, std::time::Duration::from_secs(5));
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.toasts.error(text, ToastOptions::default());
                }
                self.reset();
            }
            RvcdMsg::FileOpenData(data) => {
                // re-direct this to service side
                if let Some(channel) = &self.channel {
                    channel.tx.send(RvcdMsg::FileOpenData(data)).unwrap();
                }
            }
            RvcdMsg::FileDrag(file) => {
                // re-direct this to service side
                if let Some(channel) = &self.channel {
                    channel.tx.send(RvcdMsg::FileOpen(file)).unwrap();
                }
            }
            RvcdMsg::LoadingProgress(progress) => {
                self.load_progress = progress;
            }
            RvcdMsg::FileLoadStart(_filepath) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if _filepath != self.filepath {
                        info!("open new file, clear all signals");
                        self.view.signals.clear();
                    } else {
                        info!("open old file, remove unavailable signals");
                        if let Some(wave) = &self.wave {
                            self.view.signals_clean_unavailable(&wave.info);
                        }
                    }
                    self.filepath = _filepath;
                }
                self.signal_leaves.clear();
                if self.state == State::Idle {
                    self.load_progress = 0.0;
                    self.state = State::Loading;
                }
            }
        };
    }
    pub fn reload(&mut self) {
        info!("reloading file");
        if let Some(channel) = &self.channel {
            let sender = channel.tx.clone();
            #[cfg(not(target_arch = "wasm32"))]
            {
                sender
                    .send(RvcdMsg::FileOpen(FileHandle::from(PathBuf::from(
                        self.filepath.to_string(),
                    ))))
                    .ok();
            }
            #[cfg(target_arch = "wasm32")]
            {
                let file = self.file.take();
                if let Some(file) = file {
                    sender.send(RvcdMsg::FileOpen(file)).ok();
                }
            }
        }
    }
    pub fn reset(&mut self) {
        self.wave = None;
        self.filepath.clear();
        self.state = State::Idle;
        self.view = self.view.reset();
    }
    pub fn debug_panel(&mut self, ui: &mut Ui) {
        let run_mode = &mut self.run_mode;
        ui.label("Mode:");
        ui.radio_value(run_mode, RunMode::Reactive, "Reactive")
            .on_hover_text("Repaint when there are animations or input (e.g. mouse movement)");
        ui.radio_value(run_mode, RunMode::Continuous, "Continuous")
            .on_hover_text("Repaint everything each frame");
        if self.run_mode == RunMode::Continuous {
            ui.label(format!("FPS: {:.1}", self.frame_history.fps()));
        } else {
            self.frame_history.ui(ui);
        }
        let mut debug_on_hover = ui.ctx().debug_on_hover();
        ui.checkbox(&mut debug_on_hover, "🐛 Debug mode");
        ui.ctx().set_debug_on_hover(debug_on_hover);
        if ui.button("Reset rvcd").clicked() {
            self.reset();
        }
        ui.horizontal(|ui| {
            if ui
                .button("Reset egui")
                .on_hover_text("Forget scroll, positions, sizes etc")
                .clicked()
            {
                *ui.ctx().memory() = Default::default();
            }

            if ui.button("Reset everything").clicked() {
                self.state = Default::default();
                *ui.ctx().memory() = Default::default();
            }
        });
        egui::warn_if_debug_build(ui);
    }
    pub fn menubar(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        egui::widgets::global_dark_light_mode_switch(ui);
        ui.menu_button("File", |ui| {
            // #[cfg(not(target_arch = "wasm32"))]
            if ui.button("Open").clicked() {
                if let Some(channel) = &self.channel {
                    let task = rfd::AsyncFileDialog::new()
                        .add_filter("VCD File", &["vcd"])
                        .pick_file();
                    let sender = channel.tx.clone();
                    execute(async move {
                        let file = task.await;
                        if let Some(file) = file {
                            // let path = PathBuf::from(file);
                            // let path = file.path().to_str().unwrap().to_string();
                            sender.send(RvcdMsg::FileOpen(file)).ok();
                        }
                    });
                }
                ui.close_menu();
            }
            ui.add_enabled_ui(self.state == State::Working, |ui| {
                if ui.button("Close").clicked() {
                    ui.close_menu();
                    self.reset();
                }
            });
            #[cfg(not(target_arch = "wasm32"))]
            if ui.button("Quit").clicked() {
                _frame.close();
            }
        });
        self.view.menu(ui);
        ui.checkbox(&mut self.debug_panel, "Debug Panel");
        if ui.button("Test Toast").clicked() {
            self.toasts.info("Test Toast", ToastOptions::default());
        }
        ui.label(format!("State: {:?}", self.state));
    }
    pub fn handle_dropping_file(&mut self, ctx: &egui::Context) {
        // Collect dropped files:
        if !ctx.input().raw.dropped_files.is_empty() {
            let dropped_files: Vec<DroppedFile> = ctx.input().raw.dropped_files.clone();
            info!("drag {} files!", dropped_files.len());
            dropped_files.first().map(|dropped_file| {
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(path) = &dropped_file.path {
                    if path.is_file() {
                        let file = FileHandle::from(path.clone());
                        self.message_handler(RvcdMsg::FileDrag(file));
                    } else {
                        self.message_handler(RvcdMsg::FileOpenFailed);
                    }
                } else {
                    if let Some(data) = &dropped_file.bytes {
                        self.message_handler(RvcdMsg::FileOpenData(data.clone()));
                    }
                }
                #[cfg(target_arch = "wasm32")]
                if let Some(data) = &dropped_file.bytes {
                    self.message_handler(RvcdMsg::FileOpenData(data.clone()));
                }
            });
        }
    }
}
