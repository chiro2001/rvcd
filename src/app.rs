use crate::message::RVCDMsg;
use crate::tree_view::TreeAction;
use crate::utils::execute;
use crate::wave::WaveTreeNode;
use crate::RVCD;
use eframe::emath::Align;
use egui::{vec2, Align2, Layout, ScrollArea, Sense};
use tracing::info;

impl eframe::App for RVCD {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
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
                                    sender.send(RVCDMsg::FileOpen(file)).ok();
                                }
                            });
                        }
                        ui.close_menu();
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
                let mut debug_on_hover = ui.ctx().debug_on_hover();
                ui.checkbox(&mut debug_on_hover, "🐛 Debug mode");
                ui.ctx().set_debug_on_hover(debug_on_hover);
                egui::warn_if_debug_build(ui);
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.with_layout(
                Layout::top_down(Align::LEFT).with_cross_justify(true),
                |ui| {
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
                                            let response = ui
                                                .add(egui::Label::new(name).sense(Sense::click()));
                                            if response.double_clicked() {
                                                if !self.signals.contains(id) {
                                                    self.signals.push(*id);
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
                                                    if !self.signals.contains(&d.0) {
                                                        self.signals.push(d.0);
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
                },
            );
        });

        const SIGNAL_HEIGHT: f32 = 30.0;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(
                Layout::top_down(Align::LEFT).with_cross_justify(true),
                |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        egui::SidePanel::left("signals")
                            .resizable(true)
                            .show_inside(ui, |ui| {
                                if let Some(info) = &self.wave_info {
                                    for id in self.signals.iter() {
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
                                for id in self.signals.iter() {
                                    ui.scope(|ui| {
                                        ui.set_height(SIGNAL_HEIGHT);
                                        ui.centered_and_justified(|ui| {
                                            let (mut _response, painter) = ui.allocate_painter(
                                                ui.available_size_before_wrap(),
                                                Sense::hover(),
                                            );
                                            let items =
                                                self.wave_data.iter().filter(|i| i.id == *id); //.collect::<Vec<_>>();
                                            let color = ui.visuals().strong_text_color();
                                            let rect = ui.max_rect();
                                            for item in items {
                                                let text = item.value.to_string();
                                                let width = rect.right() - rect.left();
                                                let percent = ((item.timestamp - info.range.0)
                                                    as f32)
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
                },
            );
        });

        if let Some(channel) = &self.channel {
            if let Ok(rx) = channel.rx.try_recv() {
                match rx {
                    RVCDMsg::UpdateInfo(info) => {
                        info!("ui recv info: {}", info);
                        self.wave_info = Some(info);
                    }
                    RVCDMsg::FileOpen(_path) => {
                        // self.filepath = path.to_str().unwrap().to_string();
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            self.filepath = _path.path().to_str().unwrap().to_string();
                        }
                    }
                    RVCDMsg::UpdateData(data) => {
                        self.wave_data = data;
                    }
                };
            }
        }

        // if false {
        //     egui::Window::new("Window").show(ctx, |ui| {
        //         ui.label("Windows can be moved by dragging them.");
        //         ui.label("They are automatically sized based on contents.");
        //         ui.label("You can turn on resizing and scrolling if you like.");
        //         ui.label("You would normally choose either panels OR windows.");
        //     });
        // }
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
