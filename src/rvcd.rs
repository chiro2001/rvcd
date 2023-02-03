use crate::message::{RvcdChannel, RvcdMsg};
use crate::service::Service;
use crate::size::FileSizeUnit;
use crate::tree_view::{TreeAction, TreeView};
use crate::utils::execute;
use crate::view::signal::SignalView;
use crate::view::{WaveView, SIGNAL_LEAF_HEIGHT_DEFAULT};
use crate::wave::{Wave, WaveSignalInfo, WaveTreeNode};
use eframe::emath::Align;
use egui::{vec2, Direction, DroppedFile, Layout, ProgressBar, ScrollArea, Sense, Ui, Widget};
use egui_extras::{Column, TableBuilder};
use egui_toast::{ToastOptions, Toasts};
use num_traits::Float;
use rfd::FileHandle;
#[allow(unused_imports)]
use std::path::PathBuf;
use std::sync::mpsc;
use tracing::{info, warn};

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
    pub id: usize,
    title: String,
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
    pub load_progress: (f32, usize),
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

    #[cfg(target_arch = "wasm32")]
    #[serde(skip)]
    pub file: Option<FileHandle>,

    pub tree: TreeView,
}

impl Default for Rvcd {
    fn default() -> Self {
        Self {
            id: 0,
            title: "Rvcd".to_string(),
            state: State::default(),
            channel: None,
            filepath: "".to_string(),
            load_progress: (0.0, 0),
            signal_leaves: vec![],
            wave: None,
            view: Default::default(),
            toasts: Toasts::new()
                .direction(Direction::BottomUp)
                .align_to_end(true),
            #[cfg(target_arch = "wasm32")]
            file: None,
            tree: Default::default(),
        }
    }
}

impl Rvcd {
    /// Called once before the first frame.
    pub fn new(id: usize) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }
    pub fn init(mut self) -> Self {
        let (channel_req_tx, channel_req_rx) = mpsc::channel();
        let (channel_resp_tx, channel_resp_rx) = mpsc::channel();

        // launch service
        Service::start(RvcdChannel {
            tx: channel_resp_tx.clone(),
            rx: channel_req_rx,
        });
        // auto open file
        // let filepath = "data/cpu_ila_commit.vcd";
        #[cfg(not(target_arch = "wasm32"))]
        {
            let filepath = &self.filepath;
            info!("last file: {}", filepath);
            if !filepath.is_empty() {
                channel_req_tx
                    .send(RvcdMsg::FileOpen(rfd::FileHandle::from(
                        std::path::PathBuf::from(filepath),
                    )))
                    .unwrap();
            }
        }
        self.channel = Some(RvcdChannel {
            tx: channel_req_tx,
            rx: channel_resp_rx,
        });
        self.view.set_tx(channel_resp_tx);
        info!("last loaded {} signals", self.view.signals.len());
        self
    }
    pub fn title(&self) -> &str {
        match self.state {
            State::Working => self.title.as_str(),
            _ => "Rvcd",
        }
    }
    pub fn update<F>(
        &mut self,
        ui: &mut Ui,
        frame: &mut eframe::Frame,
        sst_enabled: bool,
        maximum: bool,
        do_min_max: F,
    ) where
        F: FnOnce(),
    {
        egui::TopBottomPanel::top("top_panel").show_inside(ui, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                self.menubar(ui, frame);
                if maximum {
                    if ui.button("Minimum").clicked() {
                        do_min_max();
                    }
                } else {
                    if ui.button("Maximum").clicked() {
                        do_min_max();
                    }
                }
            });
        });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.add_enabled_ui(self.state == State::Working, |ui| {
                if sst_enabled {
                    egui::SidePanel::left("side_panel")
                        .resizable(true)
                        .show_inside(ui, |ui| {
                            self.sidebar(ui);
                        });
                }
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        self.wave_panel(ui);
                    });
                });
            });
        });

        if let Some(channel) = &self.channel {
            let mut messages = vec![];
            while let Ok(rx) = channel.rx.try_recv() {
                messages.push(rx);
            }
            for rx in messages {
                self.message_handler(rx);
            }
        }

        let ctx = ui.ctx();
        if self.state == State::Loading {
            egui::Window::new("Loading")
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Loading Progress: {:.1}% / {}",
                        self.load_progress.0 * 100.0,
                        FileSizeUnit::from_bytes(self.load_progress.1)
                    ));
                    ProgressBar::new(self.load_progress.0).ui(ui);
                    ui.centered_and_justified(|ui| {
                        if ui.button("Cancel").clicked() {
                            if let Some(channel) = &self.channel {
                                info!("sent FileLoadCancel");
                                channel.tx.send(RvcdMsg::FileLoadCancel).unwrap();
                            }
                        }
                    });
                });
            ctx.request_repaint();
        }
        self.toasts
            .show_with_anchor(ctx, ctx.available_rect().max - vec2(20.0, 10.0));

        // TODO: fix files drop
        // preview_files_being_dropped(ctx);
        // self.handle_dropping_file(ctx);
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
            .min_height(200.0)
            .max_height(400.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                // let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
                let text_height = SIGNAL_LEAF_HEIGHT_DEFAULT;
                TableBuilder::new(ui)
                    .resizable(false)
                    .striped(true)
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .column(Column::remainder())
                    .min_scrolled_height(0.0)
                    .max_scroll_height(f32::infinity())
                    .header(SIGNAL_LEAF_HEIGHT_DEFAULT, |mut header| {
                        header.col(|ui| {
                            ui.strong("Signals");
                        });
                    })
                    .body(|body| {
                        body.rows(
                            text_height,
                            self.signal_leaves.len(),
                            |row_index, mut row| {
                                row.col(|ui| {
                                    if let Some(signal) = self.signal_leaves.get(row_index) {
                                        let response = ui.add(
                                            egui::Label::new(signal.to_string())
                                                .sense(Sense::click_and_drag()),
                                        );
                                        if response.double_clicked() {
                                            self.signal_clicked(signal.id, true);
                                        }
                                    }
                                });
                            },
                        );
                    });
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ScrollArea::both().show(ui, |ui| {
                // ui.centered_and_justified(|ui| {
                if let Some(wave) = &self.wave {
                    match self.tree.ui(ui, wave.info.tree.root()) {
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
            RvcdMsg::LoadingProgress(..) => {}
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
                // set state to loading
                self.state = State::Loading;
            }
            RvcdMsg::LoadingProgress(progress, sz) => {
                self.load_progress = (progress, sz);
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
                    self.load_progress = (0.0, 0);
                    self.state = State::Loading;
                }
            }
            RvcdMsg::FileLoadCancel => {}
            RvcdMsg::ServiceDataReady(_) => {}
            RvcdMsg::StopService => {}
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
        ui.menu_button("SST", |ui| {
            // if ui.checkbox(&mut self.sst_enabled, "Enable SST").clicked() {
            //     ui.close_menu();
            // };
            // if self.sst_enabled {
            self.tree.menu(ui);
            // }
        });
        // ui.checkbox(&mut self.debug_panel, "Debug Panel");
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
    pub fn on_exit(&mut self) {
        if let Some(channel) = &self.channel {
            match channel.tx.send(RvcdMsg::StopService) {
                Ok(_) => {}
                Err(e) => warn!("cannot send stop msg: {}", e),
            };
        }
    }
}
