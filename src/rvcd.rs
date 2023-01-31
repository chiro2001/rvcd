use crate::frame_history::FrameHistory;
use crate::message::{RvcdChannel, RvcdMsg};
use crate::run_mode::RunMode;
use crate::service::Service;
use crate::tree_view::{TreeAction, TreeView};
use crate::view::{SignalView, WaveView};
use crate::wave::{WaveDataItem, WaveInfo, WaveSignalInfo, WaveTreeNode};
use eframe::emath::Align;
use egui::{Layout, ScrollArea, Sense, Ui};
use egui_toast::Toasts;
use rfd::FileHandle;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;
use tracing::info;

#[derive(serde::Deserialize, serde::Serialize, Default)]
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
    #[serde(skip)]
    pub state: State,
    /// ui <- -> service
    #[serde(skip)]
    pub channel: Option<RvcdChannel>,

    pub filepath: String,

    #[serde(skip)]
    pub signal_leaves: Vec<WaveSignalInfo>,

    #[serde(skip)]
    pub tree: TreeView,
    #[serde(skip)]
    pub wave_info: Option<WaveInfo>,

    #[serde(skip)]
    pub wave_data: Vec<WaveDataItem>,

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
}

impl Default for Rvcd {
    fn default() -> Self {
        Self {
            state: State::default(),
            channel: None,
            filepath: "".to_string(),
            signal_leaves: vec![],
            tree: Default::default(),
            wave_info: None,
            wave_data: vec![],
            view: Default::default(),
            toasts: Toasts::new()
                .anchor((480.0, 300.0))
                .direction(egui::Direction::TopDown)
                .align_to_end(false),
            repaint_after_seconds: 1.0,
            run_mode: Default::default(),
            frame_history: Default::default(),
            debug_panel: false,
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
                tracing::info!("last file: {}", filepath);
                if !filepath.is_empty() {
                    channel_req_tx
                        .send(crate::message::RvcdMsg::FileOpen(rfd::FileHandle::from(
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
    fn signal_clicked(&mut self, id: u64) {
        if !self.view.signals.iter().any(|x| x.s.id == id) {
            if let Some(info) = &self.wave_info {
                self.view.signals.push(SignalView::from_id(id, info));
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
                                self.signal_clicked(clicked_id);
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
                        if let Some(info) = &self.wave_info {
                            match self.tree.ui(ui, info.tree.root()) {
                                TreeAction::None => {}
                                TreeAction::AddSignal(node) => match node {
                                    WaveTreeNode::WaveVar(d) => {
                                        self.signal_clicked(d.id);
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
                                                self.signal_clicked(d.id);
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
        self.view.panel(ui, &self.wave_info, &self.wave_data);
    }
    pub fn message_handler(&mut self, msg: RvcdMsg) {
        info!(
            "ui handle msg: {:?}; signals: {}",
            msg,
            self.view.signals.len()
        );
        match msg {
            RvcdMsg::UpdateInfo(info) => {
                info!("ui recv info: {}", info);
                self.wave_info = Some(info);
                self.signal_leaves.clear();
                if let Some(info) = &self.wave_info {
                    self.view.signals_clean_unavailable(info);
                }
            }
            RvcdMsg::FileOpen(_path) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let path_new = _path.path().to_str().unwrap().to_string();
                    if path_new != self.filepath {
                        info!("open new file, clear all signals");
                        self.view.signals.clear();
                    } else {
                        info!("open old file, remove unavailable signals");
                        if let Some(info) = &self.wave_info {
                            self.view.signals_clean_unavailable(info);
                        }
                    }
                    self.filepath = path_new;
                }
                self.signal_leaves.clear();
            }
            RvcdMsg::UpdateData(data) => {
                self.wave_data = data;
            }
            RvcdMsg::Reload => {
                self.reload();
            }
            RvcdMsg::Notification(toast) => {
                self.toasts.add(toast);
            }
            RvcdMsg::FileOpenFailed => {
                self.toasts.error("File not found!", Duration::from_secs(5));
                self.reset();
            }
        };
    }
    pub fn reload(&self) {
        info!("reloading file");
        if let Some(channel) = &self.channel {
            let sender = channel.tx.clone();
            sender
                .send(RvcdMsg::FileOpen(FileHandle::from(PathBuf::from(
                    self.filepath.to_string(),
                ))))
                .ok();
        }
    }
    pub fn reset(&mut self) {
        self.wave_info = None;
        self.wave_data.clear();
        self.filepath.clear();
        self.state = State::Idle;
        self.view.reset();
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
        egui::warn_if_debug_build(ui);
    }
}
