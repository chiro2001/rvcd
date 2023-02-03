use crate::files::preview_files_being_dropped;
use crate::message::RvcdMsg;
use crate::run_mode::RunMode;
use crate::rvcd::State;
use crate::size::FileSizeUnit;
use crate::Rvcd;
use eframe::glow::Context;
use egui::{vec2, ProgressBar, Widget};
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
                self.menubar(ui, frame);
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(self.state == State::Working, |ui| {
                if self.sst_enabled {
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

        preview_files_being_dropped(ctx);
        self.handle_dropping_file(ctx);
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        info!("saving with {} signals loaded", self.view.signals.len());
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn on_exit(&mut self, _gl: Option<&Context>) {
        if let Some(channel) = &self.channel {
            channel.tx.send(RvcdMsg::StopService).unwrap();
        }
    }
}
