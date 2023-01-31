use crate::message::RvcdChannel;
use crate::service::Service;
use crate::tree_view::{TreeAction, TreeView};
use crate::view::WaveView;
use crate::wave::{WaveDataItem, WaveInfo, WaveTreeNode};
use eframe::emath::Align;
use egui::{vec2, Align2, Layout, ScrollArea, Sense, Ui};
use std::sync::mpsc;

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
    pub(crate) state: State,
    /// ui <- -> service
    #[serde(skip)]
    pub(crate) channel: Option<RvcdChannel>,

    pub(crate) filepath: String,

    #[serde(skip)]
    pub(crate) signal_leaves: Vec<(u64, String)>,

    #[serde(skip)]
    pub(crate) tree: TreeView,
    #[serde(skip)]
    pub(crate) wave_info: Option<WaveInfo>,

    #[serde(skip)]
    pub(crate) wave_data: Vec<WaveDataItem>,

    pub(crate) view: WaveView,
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
            tx: channel_resp_tx,
            rx: channel_req_rx,
        });

        let def = if let Some(storage) = cc.storage {
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
            def
        } else {
            Default::default()
        };
        Self {
            channel: Some(RvcdChannel {
                tx: channel_req_tx,
                rx: channel_resp_rx,
            }),
            ..def
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
                            for (id, name) in self.signal_leaves.iter() {
                                let response = ui.add(egui::Label::new(name).sense(Sense::click()));
                                if response.double_clicked() {
                                    if !self.view.signals.contains(id) {
                                        self.view.signals.push(*id);
                                    }
                                }
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
                                        if !self.view.signals.contains(&d.0) {
                                            self.view.signals.push(d.0);
                                        }
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
    pub fn wave_panel(&self, ui: &mut Ui) {
        const SIGNAL_HEIGHT: f32 = 30.0;
        ScrollArea::vertical().show(ui, |ui| {
            egui::SidePanel::left("signals")
                .resizable(true)
                .show_inside(ui, |ui| {
                    if let Some(info) = &self.wave_info {
                        for id in self.view.signals.iter() {
                            if let Some(name) = info.code_names.get(id) {
                                ui.scope(|ui| {
                                    ui.set_height(SIGNAL_HEIGHT);
                                    ui.centered_and_justified(|ui| {
                                        ui.add(egui::Label::new(name).wrap(false));
                                    });
                                });
                            }
                        }
                    }
                });
            egui::CentralPanel::default().show_inside(ui, |ui| {
                if let Some(info) = &self.wave_info {
                    for id in self.view.signals.iter() {
                        ui.scope(|ui| {
                            ui.set_height(SIGNAL_HEIGHT);
                            ui.centered_and_justified(|ui| {
                                let (mut _response, painter) = ui.allocate_painter(
                                    ui.available_size_before_wrap(),
                                    Sense::hover(),
                                );
                                let items = self.wave_data.iter().filter(|i| i.id == *id); //.collect::<Vec<_>>();
                                let color = ui.visuals().strong_text_color();
                                let rect = ui.max_rect();
                                for item in items {
                                    let text = item.value.to_string();
                                    let width = rect.right() - rect.left();
                                    let percent = ((item.timestamp - info.range.0) as f32)
                                        / ((info.range.1 - info.range.0) as f32);
                                    let pos = rect.left_center() + vec2(width * percent, 0.0);
                                    painter.text(
                                        pos,
                                        Align2::CENTER_CENTER,
                                        text,
                                        Default::default(),
                                        color,
                                    );
                                }
                            });
                        });
                    }
                }
            });
        });
    }
}
