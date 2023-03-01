use crate::app::RvcdAppMessage;
use crate::code::highlight::code_view_ui;
use crate::manager::{RvcdRpcMessage, MANAGER_PORT};
use crate::message::{RvcdChannel, RvcdMsg};
use crate::rpc::rvcd_rpc_client::RvcdRpcClient;
use crate::rpc::{RvcdEmpty, RvcdRemoveClient};
use crate::service::Service;
use crate::size::FileSizeUnit;
use crate::tree_view::{TreeAction, TreeView};
use crate::utils::{execute, file_basename};
use crate::verilog::{parse_verilog_file, VerilogGotoSource, VerilogViewSource};
use crate::view::signal::SignalView;
use crate::view::{WaveView, SIGNAL_LEAF_HEIGHT_DEFAULT};
use crate::wave::{Wave, WaveSignalInfo, WaveTreeNode};
use eframe::emath::Align;
use egui::{
    vec2, Align2, Color32, Direction, DroppedFile, Id, Label, Layout, ProgressBar, RichText,
    ScrollArea, Sense, Ui, Widget,
};
use egui_extras::{Column, TableBuilder};
use egui_toast::Toasts;
use num_traits::Float;
use regex::Regex;
use rfd::FileHandle;
use std::fmt::{Debug, Display, Formatter};
#[allow(unused_imports)]
use std::path::PathBuf;
use std::sync::{mpsc, Arc};
use tonic::IntoRequest;
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
    #[serde(skip)]
    pub state: State,
    /// ui <- -> service
    #[serde(skip)]
    pub channel: Option<RvcdChannel>,
    #[serde(skip)]
    pub loop_self: Option<mpsc::Sender<RvcdMsg>>,
    /// Loaded file path.
    ///
    /// **Only available on native**
    pub filepath: String,
    #[serde(skip)]
    pub load_progress: (f32, usize),
    #[serde(skip)]
    pub parse_progress: (f32, u64),
    #[serde(skip)]
    pub last_progress_msg: RvcdMsg,
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

    pub search_text: String,
    pub search_tree: bool,
    pub search_regex: bool,

    #[cfg(not(target_arch = "wasm32"))]
    pub source_dir: String,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub sources_update_started: bool,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub sources_updated: bool,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub alternative_goto_sources: Vec<VerilogGotoSource>,
    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub alternative_view_source: Option<VerilogViewSource>,

    #[cfg(not(target_arch = "wasm32"))]
    #[serde(skip)]
    pub client: Arc<crate::client::RvcdManagedClient>,

    #[serde(skip)]
    pub upper_tx: Option<mpsc::Sender<RvcdAppMessage>>,
    #[serde(skip)]
    pub rpc_rx: Option<mpsc::Receiver<RvcdRpcMessage>>,
}

impl Display for Rvcd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.id, self.title)
    }
}

impl Debug for Rvcd {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Default for Rvcd {
    fn default() -> Self {
        Self {
            id: 0,
            title: "Rvcd".to_string(),
            state: State::default(),
            channel: None,
            loop_self: None,
            filepath: "".to_string(),
            load_progress: (0.0, 0),
            parse_progress: (0.0, 0),
            last_progress_msg: RvcdMsg::LoadingProgress(0.0, 0),
            signal_leaves: vec![],
            wave: None,
            view: Default::default(),
            toasts: Toasts::new()
                .direction(Direction::BottomUp)
                .align_to_end(true),
            #[cfg(target_arch = "wasm32")]
            file: None,
            tree: Default::default(),
            search_text: "".to_string(),
            search_tree: false,
            search_regex: false,
            #[cfg(not(target_arch = "wasm32"))]
            source_dir: "".to_string(),
            #[cfg(not(target_arch = "wasm32"))]
            sources_update_started: false,
            #[cfg(not(target_arch = "wasm32"))]
            sources_updated: false,
            #[cfg(not(target_arch = "wasm32"))]
            alternative_goto_sources: vec![],
            #[cfg(not(target_arch = "wasm32"))]
            alternative_view_source: None,
            #[cfg(not(target_arch = "wasm32"))]
            client: Arc::new(Default::default()),
            upper_tx: None,
            rpc_rx: None,
        }
    }
}

impl Rvcd {
    /// Called once before the first frame.
    pub fn new(id: usize) -> Self {
        info!("create rvcd id={}", id);
        Self {
            id,
            title: format!("Rvcd-{id}"),
            ..Default::default()
        }
    }
    pub fn init(&mut self) {
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
            info!(
                "last file: {}; last source dir: {}",
                filepath, self.source_dir
            );
            if !filepath.is_empty() {
                channel_req_tx
                    .send(RvcdMsg::FileOpen(rfd::FileHandle::from(
                        std::path::PathBuf::from(filepath),
                    )))
                    .unwrap();
            }
            let client = self.client.clone();
            tokio::spawn(async move {
                client.run().await;
            });
        }
        let loop_self = channel_resp_tx.clone();
        self.loop_self = Some(loop_self);
        self.channel = Some(RvcdChannel {
            tx: channel_req_tx,
            rx: channel_resp_rx,
        });
        self.view.set_id(self.id);
        self.view.set_tx(channel_resp_tx);
        let (rpc_tx, rpc_rx) = mpsc::channel();
        self.rpc_rx = Some(rpc_rx);
        self.client.set_tx(rpc_tx);
        // self.view.set_sources(self.);
        info!("last loaded {} signals", self.view.signals.len());
    }
    pub fn title(&self) -> String {
        match self.state {
            State::Working => self.title.to_string(),
            _ => format!("Rvcd-{}", self.id),
        }
    }
    pub fn set_upper_tx(&mut self, tx: mpsc::Sender<RvcdAppMessage>) {
        self.upper_tx = Some(tx);
    }
    pub fn update<F>(
        &mut self,
        ui: &mut Ui,
        frame: &mut eframe::Frame,
        sst_enabled: bool,
        maximize: bool,
        do_min_max: F,
    ) where
        F: FnOnce(),
    {
        if !maximize {
            egui::TopBottomPanel::top(format!("top_panel_{}", self.id)).show_inside(ui, |ui| {
                // The top panel is often a good place for a menu bar:
                egui::menu::bar(ui, |ui| {
                    self.menubar(ui, frame, maximize);
                    if maximize {
                        if ui.button(t!("menu.minimize")).clicked() {
                            do_min_max();
                        }
                    } else if ui.button(t!("menu.maximize")).clicked() {
                        do_min_max();
                    }
                });
            });
            egui::TopBottomPanel::bottom(format!("bottom_panel_{}", self.id))
                .min_height(0.0)
                .resizable(false)
                .show_inside(ui, |_ui| {
                    // ui.label("bottom");
                });
        }
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.add_enabled_ui(self.state == State::Working, |ui| {
                if sst_enabled {
                    egui::SidePanel::left(format!("side_panel_{}", self.id))
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

        if let Some(rx) = &self.rpc_rx {
            let mut messages = vec![];
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);
            }
            for msg in messages {
                self.handle_rpc_message(msg);
            }
        }

        let ctx = ui.ctx();
        if self.state == State::Loading {
            egui::Window::new(t!("loading.title"))
                .id(Id::from(format!("loading_rvcd_{}", self.id)))
                .resizable(false)
                .show(ctx, |ui| {
                    let handle_cancel = |ui: &mut Ui| {
                        ui.vertical_centered_justified(|ui| {
                            if ui.button(t!("loading.cancel")).clicked() {
                                if let Some(channel) = &self.channel {
                                    info!("sent FileLoadCancel");
                                    channel.tx.send(RvcdMsg::FileLoadCancel).unwrap();
                                }
                            }
                        });
                    };
                    if let RvcdMsg::LoadingProgress(..) = self.last_progress_msg {
                        ui.label(t!(
                            "loading.load_progress",
                            percent = format!("{:.1}", self.load_progress.0 * 100.0).as_str(),
                            bytes = FileSizeUnit::from_bytes(self.load_progress.1)
                                .to_string()
                                .as_str()
                        ));
                        ProgressBar::new(self.load_progress.0).ui(ui);
                        handle_cancel(ui);
                    } else {
                        ui.label(t!(
                            "loading.parse_progress",
                            percent = format!("{:.1}", self.parse_progress.0 * 100.0).as_str(),
                            pos = self.load_progress.1.to_string().as_str()
                        ));
                        ProgressBar::new(self.parse_progress.0).ui(ui);
                        // handle_cancel(ui);
                    }
                });
            ctx.request_repaint();
        }
        // auto update sources
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(channel) = &self.channel {
            if !self.sources_update_started && !self.sources_updated && !self.source_dir.is_empty()
            {
                self.sources_update_started = true;
                let tx = &channel.tx;
                tx.send(RvcdMsg::UpdateSourceDir(self.source_dir.to_string()))
                    .unwrap();
            }
        }

        let mut _open_alternative_goto_sources = true;
        #[cfg(not(target_arch = "wasm32"))]
        if !self.alternative_goto_sources.is_empty() {
            egui::Window::new("选择要跳转到的目标")
                .open(&mut _open_alternative_goto_sources)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        TableBuilder::new(ui).column(Column::auto()).body(|body| {
                            body.heterogeneous_rows(
                                (0..self.alternative_goto_sources.len()).map(|_| 40.0),
                                |index, mut row| {
                                    if let Some(a) =
                                        self.alternative_goto_sources.get(index).map(|x| x.clone())
                                    {
                                        row.col(|ui| {
                                            let resp = ui.add(
                                                Label::new(format!(
                                                    "{}:{}:{}",
                                                    file_basename(a.file.as_str()),
                                                    a.location.line,
                                                    a.location.column
                                                ))
                                                .sense(Sense::click()),
                                            );
                                            if resp.double_clicked() {
                                                if let Some(loop_self) = &self.loop_self {
                                                    loop_self
                                                        .send(RvcdMsg::CallGotoSources(a))
                                                        .unwrap();
                                                    self.alternative_goto_sources.clear();
                                                }
                                            } else if resp.clicked()
                                                || self.alternative_view_source.is_none()
                                            {
                                                let f = self
                                                    .view
                                                    .sources
                                                    .iter()
                                                    .filter(|x| x.source_path == a.file)
                                                    .map(|x| x.source_code.0.as_str())
                                                    .collect::<Vec<_>>();
                                                if let Some(f) = f.first() {
                                                    let code = f.to_string();
                                                    let mut line = 1isize;
                                                    let mut offset = 0usize;
                                                    for (i, c) in code.chars().enumerate() {
                                                        if line >= a.location.line {
                                                            offset = i + a.location.column as usize;
                                                            break;
                                                        }
                                                        if c == '\n' {
                                                            line += 1;
                                                        }
                                                    }
                                                    self.alternative_view_source =
                                                        Some(VerilogViewSource {
                                                            file: a.file,
                                                            path: a.path,
                                                            text: code,
                                                            offset,
                                                        });
                                                }
                                            }
                                        });
                                    }
                                },
                            );
                        });
                        if let Some(v) = &mut self.alternative_view_source {
                            ScrollArea::both().id_source("code-preview").show(ui, |ui| {
                                code_view_ui(ui, &mut v.text, Some(v.offset));
                            });
                        }
                    });
                });
        }
        if !_open_alternative_goto_sources {
            self.alternative_goto_sources.clear();
            self.alternative_view_source = None;
        }

        self.toasts
            .show_with_anchor(ctx, ctx.available_rect().max - vec2(20.0, 10.0));

        // TODO: fix files drop
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
        // test if regex is valid
        let test_regex = Regex::new(if self.search_regex {
            self.search_text.as_str()
        } else {
            ""
        });
        egui::TopBottomPanel::bottom(format!("signal_search_{}", self.id))
            .resizable(false)
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if test_regex.is_err() {
                        ui.label(RichText::new("❌").color(Color32::RED));
                    } else {
                        ui.label("🔍");
                    }
                    ui.text_edit_singleline(&mut self.search_text);
                });
                match test_regex {
                    Ok(_) => {}
                    Err(e) => {
                        ui.label(RichText::new(e.to_string()).color(Color32::RED));
                    }
                };
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.search_regex, "Regex");
                    ui.checkbox(&mut self.search_tree, "Tree");
                    if ui.button("Append").clicked() {}
                    if ui.button("Replace").clicked() {}
                });
            });
        egui::TopBottomPanel::bottom(format!("signal_leaf_{}", self.id))
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
                    .column(Column::auto())
                    .column(Column::remainder())
                    .min_scrolled_height(0.0)
                    .max_scroll_height(f32::infinity())
                    .header(SIGNAL_LEAF_HEIGHT_DEFAULT, |mut header| {
                        header.col(|ui| {
                            ui.label(t!("sidebar.leaf.type"));
                        });
                        header.col(|ui| {
                            ui.label(t!("sidebar.leaf.signals"));
                        });
                    })
                    .body(|body| {
                        // TODO: reduce this clone
                        let signal_leaves = if self.search_text.is_empty() {
                            self.signal_leaves
                                .iter()
                                .map(|x| x.clone())
                                .collect::<Vec<_>>()
                        } else {
                            let search_text = self.search_text.as_str();
                            let re = if self.search_regex && !self.search_tree {
                                if let Ok(re) = Regex::new(search_text) {
                                    Some(re)
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            self.signal_leaves
                                .iter()
                                .filter(|x| {
                                    if !self.search_tree {
                                        if self.search_regex {
                                            if let Some(re) = &re {
                                                re.captures(x.name.as_str()).is_some()
                                            } else {
                                                false
                                            }
                                        } else {
                                            x.name.contains(search_text)
                                        }
                                    } else {
                                        true
                                    }
                                })
                                .map(|x| x.clone())
                                .collect::<Vec<_>>()
                        };
                        body.rows(text_height, signal_leaves.len(), |row_index, mut row| {
                            if let Some(signal) = signal_leaves.get(row_index) {
                                let signal = signal.clone();
                                let mut handle_draw_response = |ui: &mut Ui, text: String| {
                                    let (response, painter) = ui.allocate_painter(
                                        ui.max_rect().size(),
                                        Sense::click_and_drag(),
                                    );
                                    let on_hover = ui.ui_contains_pointer();
                                    let color = if on_hover {
                                        ui.visuals().strong_text_color()
                                    } else {
                                        ui.visuals().text_color()
                                    };
                                    painter.text(
                                        response.rect.left_center(),
                                        Align2::LEFT_CENTER,
                                        text,
                                        Default::default(),
                                        color,
                                    );
                                    if response.double_clicked() {
                                        self.signal_clicked(signal.id, true);
                                    }
                                };
                                row.col(|ui| {
                                    handle_draw_response(ui, signal.typ.to_string());
                                });
                                row.col(|ui| {
                                    handle_draw_response(ui, signal.to_string());
                                });
                            } else {
                                row.col(|ui| {
                                    ui.label("-");
                                });
                                row.col(|ui| {
                                    ui.label("-");
                                });
                            }
                        });
                    });
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ScrollArea::both().show(ui, |ui| {
                // ui.centered_and_justified(|ui| {
                if let Some(wave) = &self.wave {
                    match self.tree.ui(
                        ui,
                        wave.info.tree.root(),
                        if self.search_tree {
                            self.search_text.as_str()
                        } else {
                            ""
                        },
                        self.search_regex,
                    ) {
                        TreeAction::None => {}
                        TreeAction::AddSignal(node) => {
                            if let WaveTreeNode::WaveVar(d) = node {
                                self.signal_clicked(d.id, true);
                            }
                        }
                        TreeAction::SelectScope(nodes) => {
                            self.signal_leaves = nodes
                                .into_iter()
                                .filter_map(|node| match node {
                                    WaveTreeNode::WaveVar(v) => Some(v),
                                    _ => None,
                                })
                                .collect();
                        }
                        TreeAction::AddSignals(nodes) => {
                            for node in nodes {
                                if let WaveTreeNode::WaveVar(d) = node {
                                    self.signal_clicked(d.id, false);
                                }
                            }
                        }
                    }
                } else {
                    ui.centered_and_justified(|ui| ui.label(t!("sidebar.leaf.no_file")));
                }
                // });
            });
        });
    }
    pub fn wave_panel(&mut self, ui: &mut Ui) {
        if let Some(wave) = &self.wave {
            self.view.panel(ui, wave);
        } else {
            ui.centered_and_justified(|ui| {
                ui.heading(t!("panel.no_file"));
            });
        }
    }
    pub fn message_handler(&mut self, msg: RvcdMsg) {
        match msg {
            RvcdMsg::LoadingProgress(p, s) => {
                self.last_progress_msg = RvcdMsg::LoadingProgress(p, s);
            }
            RvcdMsg::ParsingProgress(p, s) => {
                self.last_progress_msg = RvcdMsg::ParsingProgress(p, s);
            }
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
                let text = t!("msg.open_file_failed");
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.toasts.error(text, std::time::Duration::from_secs(5));
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.toasts.error(text, egui_toast::ToastOptions::default());
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
            RvcdMsg::ParsingProgress(progress, pos) => {
                self.parse_progress = (progress, pos);
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
                    self.filepath = _filepath.clone();
                    // self.title = format!("Rvcd-{_filepath}");
                    self.client.data.lock().unwrap().wave_file = _filepath.clone();
                    self.title = format!("Rvcd-{}", file_basename(_filepath.as_str()));
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
            RvcdMsg::UpdateSourceDir(_path) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    info!("update self.source_dir to {}", _path);
                    self.toasts.info(
                        format!("update self.source_dir to {}", _path),
                        std::time::Duration::from_secs(5),
                    );
                    self.source_dir = _path;
                }
            }
            RvcdMsg::UpdateSources(_sources) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    info!("self.sources updated");
                    let paths = _sources
                        .iter()
                        .map(|x| x.source_path.to_string())
                        .collect::<Vec<_>>();
                    let mut new_paths = self.client.data.lock().unwrap().paths.clone();
                    for path in paths {
                        new_paths.push(path);
                    }
                    self.client.set_paths(&new_paths);
                    // self.view.set_sources(_sources);
                    self.view.sources.extend_from_slice(&_sources);
                    self.sources_updated = true;
                }
            }
            RvcdMsg::CallGotoSources(goto) => {
                info!("ui CallGotoSources({:?})", goto);
                if let Some(tx) = &self.upper_tx {
                    tx.send(RvcdAppMessage::CreateCodeEditor(goto)).unwrap();
                }
            }
            RvcdMsg::SetAlternativeGotoSources(v) => {
                self.alternative_goto_sources = v;
            }
            RvcdMsg::GotNoSource => {
                self.toasts
                    .warning("找不到对应源文件", egui_toast::ToastOptions::default());
            }
            RvcdMsg::SetGotoSignals(list) => {
                let add_ids = list
                    .iter()
                    .filter(|v| !self.view.signals.iter().any(|x| x.s.id == **v))
                    .collect::<Vec<_>>();
                for v in &add_ids {
                    self.signal_clicked(**v, true);
                }
                self.view.highlight_signals = add_ids.into_iter().map(|x| x.clone()).collect();
            }
            RvcdMsg::UpdateSource(file) => {
                let tx = self.loop_self.clone();
                execute(async move {
                    if let Some(tx) = tx {
                        if let Ok(r) = parse_verilog_file(file.as_str()) {
                            tx.send(RvcdMsg::UpdateSources(vec![r])).unwrap();
                        }
                    }
                });
            }
        };
    }
    pub fn handle_rpc_message(&mut self, msg: RvcdRpcMessage) -> bool {
        match msg {
            RvcdRpcMessage::GotoPath(path) => {
                if self.filepath == path.file {
                    // self.view.do_source_goto(path.path);
                    if let Some(wave) = &self.wave {
                        self.view.do_signal_goto(path.path, &wave.info);
                    }
                } else {
                    return if path.file.is_empty() {
                        if let Some(wave) = &self.wave {
                            self.view.do_signal_goto(path.path, &wave.info);
                        }
                        false
                    } else {
                        true
                    };
                }
            }
            RvcdRpcMessage::OpenWaveFile(path) => {
                if let Some(channel) = &self.channel {
                    channel
                        .tx
                        .send(RvcdMsg::FileOpen(FileHandle::from(PathBuf::from(path))))
                        .unwrap();
                }
                return true;
            }
            RvcdRpcMessage::OpenSourceFile(file) => {
                if let Some(loop_self) = &self.loop_self {
                    loop_self.send(RvcdMsg::UpdateSource(file)).unwrap();
                }
            }
            RvcdRpcMessage::OpenSourceDir(path) => {
                if let Some(loop_self) = &self.loop_self {
                    loop_self.send(RvcdMsg::UpdateSourceDir(path)).unwrap();
                }
            }
        }
        false
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
        self.signal_leaves.clear();
        self.state = State::Idle;
        self.view = self.view.reset();
        self.title = format!("Rvcd-{}", self.id);
    }
    pub fn menubar(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame, _maximum: bool) {
        egui::widgets::global_dark_light_mode_switch(ui);
        ui.menu_button(t!("menu.file"), |ui| {
            // #[cfg(not(target_arch = "wasm32"))]
            if ui.button(t!("menu.open")).clicked() {
                if let Some(channel) = &self.channel {
                    let task = rfd::AsyncFileDialog::new()
                        .add_filter(t!("menu.vcd_file").as_str(), &["vcd"])
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
            #[cfg(not(target_arch = "wasm32"))]
            if ui.button(t!("menu.open_source_dir")).clicked() {
                if let Some(channel) = &self.channel {
                    self.sources_update_started = true;
                    let task = rfd::AsyncFileDialog::new().pick_folder();
                    let tx = channel.tx.clone();
                    let loop_self = self.loop_self.clone();
                    execute(async move {
                        let dir = task.await;
                        if let Some(path) = dir {
                            let path = path.path().to_str().unwrap().to_string();
                            if let Some(loop_self) = loop_self {
                                loop_self
                                    .send(RvcdMsg::UpdateSourceDir(path.to_string()))
                                    .unwrap();
                            }
                            tx.send(RvcdMsg::UpdateSourceDir(path)).unwrap();
                        }
                    });
                }
                ui.close_menu();
            }
            ui.add_enabled_ui(self.state == State::Working, |ui| {
                if ui.button(t!("menu.close")).clicked() {
                    ui.close_menu();
                    self.reset();
                }
            });
            #[cfg(not(target_arch = "wasm32"))]
            if _maximum && ui.button(t!("menu.quit")).clicked() {
                _frame.close();
            }
        });
        self.view.menu(ui);
        ui.menu_button(t!("menu.sst"), |ui| {
            // if ui.checkbox(&mut self.sst_enabled, "Enable SST").clicked() {
            //     ui.close_menu();
            // };
            // if self.sst_enabled {
            self.tree.menu(ui);
            // }
        });
        // ui.checkbox(&mut self.debug_panel, "Debug Panel");
        // if ui.button("Test Toast").clicked() {
        //     self.toasts.info("Test Toast", ToastOptions::default());
        // }
        ui.label(format!("{}: {:?}", t!("menu.state"), self.state));
    }
    pub fn handle_dropping_file(&mut self, dropped_file: &DroppedFile) {
        // Collect dropped files:
        // if !ctx.input().raw.dropped_files.is_empty() {
        //     let dropped_files: Vec<DroppedFile> = ctx.input().raw.dropped_files.clone();
        //     info!("drag {} files!", dropped_files.len());
        //     dropped_files.first().map(|dropped_file| {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = &dropped_file.path {
            if path.is_file() {
                let file = FileHandle::from(path.clone());
                self.message_handler(RvcdMsg::FileDrag(file));
            } else {
                if path.is_dir() {
                    if let Some(channel) = &self.channel {
                        channel
                            .tx
                            .send(RvcdMsg::UpdateSourceDir(
                                path.clone().to_str().unwrap().to_string(),
                            ))
                            .unwrap();
                    }
                } else {
                    self.message_handler(RvcdMsg::FileOpenFailed);
                }
            }
        } else if let Some(data) = &dropped_file.bytes {
            self.message_handler(RvcdMsg::FileOpenData(data.clone()));
        }
        #[cfg(target_arch = "wasm32")]
        if let Some(data) = &dropped_file.bytes {
            self.message_handler(RvcdMsg::FileOpenData(data.clone()));
        }
        //     });
        // }
    }
    pub fn on_exit(&mut self) {
        info!("rvcd-{} {} exiting", self.id, self.filepath);
        if let Some(channel) = &self.channel {
            match channel.tx.send(RvcdMsg::StopService) {
                Ok(_) => {}
                Err(e) => warn!("cannot send stop msg: {}", e),
            };
        }
        info!("setting client stop to true");
        *self.client.stop.lock().unwrap() = true;
        let remove_info = RvcdRemoveClient {
            key: self.client.data.lock().unwrap().port as u32,
        };
        execute(async move {
            let mut client = RvcdRpcClient::connect(format!("http://127.0.0.1:{}", MANAGER_PORT))
                .await
                .unwrap();
            info!("rpc call built");
            client
                .remove_client(remove_info.into_request())
                .await
                .unwrap();
            info!("rpc call remove_client done");
            client
                .ping(RvcdEmpty::default().into_request())
                .await
                .unwrap();
            info!("rpc call ping done");
        });
    }
}
