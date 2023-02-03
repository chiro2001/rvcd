use crate::frame_history::FrameHistory;
use crate::run_mode::RunMode;
use crate::Rvcd;
use eframe::glow::Context;
use eframe::Frame;
use egui::{CentralPanel, Ui, Window};
use tracing::info;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct RvcdApp {
    pub apps: Vec<Rvcd>,
    pub app_now_id: Option<usize>,
    #[serde(skip)]
    pub open_apps: Vec<(usize, bool)>,
    #[serde(skip)]
    pub repaint_after_seconds: f32,
    #[serde(skip)]
    pub run_mode: RunMode,
    #[serde(skip)]
    pub frame_history: FrameHistory,
    pub debug_panel: bool,
    pub sst_enabled: bool,
}

impl RvcdApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut def: RvcdApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };
        if def.apps.is_empty() {
            def.apps = vec![Rvcd::new(0)];
        }
        def.open_apps = def.apps.iter().map(|x| (x.id, true)).collect();
        def.apps = def.apps.into_iter().map(|a| a.init()).collect();
        def
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
        ui.horizontal(|ui| {
            if let Some(id) = self.app_now_id {
                if ui.button("Reset this rvcd").clicked() {
                    if let Some(app) = self.apps.get_mut(id) {
                        app.reset();
                    }
                }
            }
            if ui.button("Reset app").clicked() {
                for app in &mut self.apps {
                    app.on_exit();
                }
                self.apps.clear();
            }
        });
        ui.horizontal(|ui| {
            if ui
                .button("Reset egui")
                .on_hover_text("Forget scroll, positions, sizes etc")
                .clicked()
            {
                *ui.ctx().memory() = Default::default();
            }

            if ui.button("Reset everything").clicked() {
                *ui.ctx().memory() = Default::default();
            }
        });
        egui::warn_if_debug_build(ui);
    }
}

impl eframe::App for RvcdApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame) {
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
        egui::TopBottomPanel::top("global_menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                if ui.checkbox(&mut self.debug_panel, "Debug Panel").clicked() {
                    ui.close_menu();
                }
                if ui.checkbox(&mut self.sst_enabled, "SST").clicked() {
                    ui.close_menu();
                }
            });
        });
        if self.debug_panel {
            egui::SidePanel::left("debug_panel").show(ctx, |ui| {
                self.debug_panel(ui);
            });
        }
        let mut show_app_in_window = |app: &mut Rvcd, ctx: &egui::Context, frame: &mut Frame| {
            let open_app = self.open_apps.iter_mut().find(|x| x.0 == app.id);
            if let Some((_id, open)) = open_app {
                Window::new(app.title())
                    .min_height(200.0)
                    .default_width(ctx.used_size().x / 2.0)
                    .vscroll(false)
                    .open(open)
                    .show(ctx, |ui| {
                        app.update(ui, frame, self.sst_enabled);
                    });
            }
        };
        if let Some(id) = self.app_now_id {
            if let Some(app) = self.apps.get_mut(id) {
                CentralPanel::default().show(ctx, |ui| {
                    app.update(ui, frame, self.sst_enabled);
                });
                for app in &mut self.apps {
                    if id != app.id {
                        show_app_in_window(app, ctx, frame);
                    }
                }
            }
        } else {
            for app in &mut self.apps {
                show_app_in_window(app, ctx, frame);
            }
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
            let removed = self.apps.remove(i);
            // let removed = self.apps.get(i).unwrap();
            info!("remove rvcd: id={}", removed.id);
        }
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn on_exit(&mut self, _gl: Option<&Context>) {
        for app in &mut self.apps {
            app.on_exit();
        }
    }
}
