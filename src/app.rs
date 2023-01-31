use crate::message::RvcdMsg;
use crate::utils::execute;
use crate::Rvcd;
use eframe::emath::Align;
use egui::Layout;

impl eframe::App for Rvcd {
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
                                    sender.send(RvcdMsg::FileOpen(file)).ok();
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
                self.view.menu(ui);
                let mut debug_on_hover = ui.ctx().debug_on_hover();
                ui.checkbox(&mut debug_on_hover, "🐛 Debug mode");
                ui.ctx().set_debug_on_hover(debug_on_hover);
                egui::warn_if_debug_build(ui);
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
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }
}
