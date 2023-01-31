use crate::message::RvcdMsg;
use crate::run_mode::RunMode;
use crate::rvcd::State;
use crate::utils::execute;
use crate::Rvcd;
use eframe::emath::Align;
use egui::Layout;
use tracing::info;

impl eframe::App for Rvcd {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.frame_history
            .on_new_frame(ctx.input().time, frame.info().cpu_usage);
        match self.run_mode {
            RunMode::Continuous => {
                // Tell the backend to repaint as soon as possible
                ctx.request_repaint();
            }
            RunMode::Reactive => {
                // let the computer rest for a bit
                ctx.request_repaint_after(std::time::Duration::from_secs_f32(
                    self.repaint_after_seconds,
                ));
            }
        }
        if self.debug_panel {
            egui::SidePanel::left("debug_panel").show(ctx, |ui| {
                self.debug_panel(ui);
            });
        }
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
                                    sender.send(RvcdMsg::FileOpen(file)).ok();
                                }
                            });
                        }
                        ui.close_menu();
                    }
                    ui.add_enabled_ui(self.state == State::Working, |ui| {
                        if ui.button("Close").clicked() {
                            self.reset();
                        }
                    });
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });
                self.view.menu(ui);
                ui.checkbox(&mut self.debug_panel, "Debug Panel");
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            ui.with_layout(
                Layout::top_down(Align::LEFT).with_cross_justify(true),
                |ui| self.sidebar(ui),
            );
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(
                Layout::top_down(Align::LEFT).with_cross_justify(true),
                |ui| self.wave_panel(ui),
            );
        });

        if let Some(channel) = &self.channel {
            if let Ok(rx) = channel.rx.try_recv() {
                self.message_handler(rx);
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
        self.toasts.show(ctx);
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        info!("saving with {} signals loaded", self.view.signals.len());
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
