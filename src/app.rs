use crate::frame_history::FrameHistory;
use crate::run_mode::RunMode;
use crate::Rvcd;
use eframe::glow::Context;
use eframe::Frame;
use egui::{CentralPanel, Id, Ui, Window};
use tracing::info;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct RvcdApp {
    pub apps: Vec<Rvcd>,
    pub app_now_id: Option<usize>,
    // #[serde(skip)]
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
    fn new_id(&self) -> usize {
        self.apps
            .iter()
            .map(|x| x.id)
            .max()
            .map(|x| x + 1)
            .unwrap_or(0)
    }
    fn new_window(&mut self, maximum: bool) {
        let id = self.new_id();
        self.apps.push(Rvcd::new(id).init());
        self.open_apps.push((id, true));
        if maximum {
            self.app_now_id = Some(id);
        }
    }
    pub fn close_all(&mut self) {
        self.apps.iter_mut().for_each(|app| app.on_exit());
        self.app_now_id = None;
        self.open_apps.clear();
        self.apps.clear();
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
                ui.checkbox(&mut self.debug_panel, "Debug Panel");
                ui.checkbox(&mut self.sst_enabled, "SST");
                if ui.button("New Window").clicked() {
                    self.new_window(false);
                }
                if self.app_now_id.is_some() {
                    if ui.button("Minimize").clicked() {
                        self.app_now_id = None;
                    }
                }
                if !self.apps.is_empty() {
                    if ui.button("Close All").clicked() {
                        self.close_all();
                    }
                }
                if ui.button("Quit").clicked() {
                    frame.close();
                }
            });
        });
        if self.debug_panel {
            egui::SidePanel::left("debug_panel").show(ctx, |ui| {
                self.debug_panel(ui);
            });
        }
        let app_now_id = self.app_now_id.clone();
        let mut show_app_in_window = |app: &mut Rvcd, ctx: &egui::Context, frame: &mut Frame| {
            let open_app = self.open_apps.iter_mut().find(|x| x.0 == app.id);
            if let Some((id, open)) = open_app {
                Window::new(app.title())
                    .min_height(200.0)
                    .default_width(ctx.used_size().x / 2.0)
                    .vscroll(false)
                    .open(open)
                    .id(Id::new(*id))
                    .title_bar(true)
                    .show(ctx, |ui| {
                        app.update(ui, frame, self.sst_enabled, false, || {
                            self.app_now_id = Some(*id)
                        });
                    });
            }
        };
        let mut will_minimum_this = false;
        if let Some(id) = app_now_id {
            if let Some(app) = self.apps.get_mut(id) {
                CentralPanel::default().show(ctx, |ui| {
                    app.update(ui, frame, self.sst_enabled, true, || {
                        will_minimum_this = true;
                    });
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
        if will_minimum_this {
            self.app_now_id = None;
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
        // if self.apps.len() > 1 {
        //     info!(
        //         "ids: {:?}, to_removes: {:?}, to_remove_indexes: {:?}",
        //         self.apps.iter().map(|x| x.id).collect::<Vec<_>>(),
        //         to_removes,
        //         to_remove_indexes
        //     );
        // }
        for i in to_remove_indexes {
            if i < self.apps.len() {
                let removed = self.apps.remove(i);
                // let removed = self.apps.get(i).unwrap();
                self.open_apps = self
                    .open_apps
                    .iter()
                    .filter(|x| x.0 != removed.id)
                    .map(|x| *x)
                    .collect();
                info!(
                    "remove rvcd: id={}, open_apps: {:?}",
                    removed.id, self.open_apps
                );
            }
        }
        // if empty, create new main window
        // if self.apps.is_empty() {
        //     self.new_window(true);
        // }
    }
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn on_exit(&mut self, _gl: Option<&Context>) {
        // self.close_all();
        // close all but save data
        for app in &mut self.apps {
            app.on_exit();
        }
    }
}
