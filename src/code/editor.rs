#![cfg(not(target_arch = "wasm32"))]

use crate::code::highlight::code_view_ui;
use crate::utils::file_basename;
use crate::verilog::CodeLocation;
use egui::{Label, RichText};
use std::io::{Read, Write};
use std::time::SystemTime;
use tracing::info;

#[derive(Debug, Eq, PartialEq)]
pub enum CodeEditorState {
    FirstLoad,
    FileOpenFailed,
    Idle,
    Modified,
    NeedReload,
}

pub struct CodeEditor {
    pub file: String,
    pub text: String,
    pub read_time: Option<SystemTime>,
    pub state: CodeEditorState,
    pub open: bool,
    pub goto: Option<CodeLocation>,
    pub goto_offset: Option<usize>,
}

impl CodeEditor {
    pub fn new(path: &str, goto: Option<CodeLocation>) -> Self {
        Self {
            file: path.to_string(),
            text: "".to_string(),
            read_time: None,
            state: CodeEditorState::FirstLoad,
            open: true,
            goto,
            goto_offset: None,
        }
    }
    pub fn ui(&mut self, ctx: &egui::Context) {
        egui::Window::new(format!(
            "{}{}",
            match self.state {
                CodeEditorState::Modified => "🏀 ",
                CodeEditorState::NeedReload => "⚠️ ",
                _ => "",
            },
            file_basename(self.file.as_str())
        ))
        .id(format!("code-editor-{}", self.file).into())
        .open(&mut self.open)
        .resizable(true)
        .show(ctx, |ui| match &self.state {
            CodeEditorState::FirstLoad => {
                if let Ok(mut f) = std::fs::File::open(self.file.as_str()) {
                    if let Ok(sz) = f.read_to_string(&mut self.text) {
                        info!("load {} done with {} bytes", self.file, sz);
                        self.state = CodeEditorState::Idle;
                        self.read_time = Some(SystemTime::now());
                    } else {
                        self.state = CodeEditorState::FileOpenFailed;
                    }
                } else {
                    self.state = CodeEditorState::FileOpenFailed;
                }
            }
            CodeEditorState::FileOpenFailed => {
                ui.horizontal_centered(|ui| {
                    ui.add(Label::new(
                        RichText::new(t!("editor.open_file_failed", file = self.file.as_str()))
                            .color(ui.visuals().warn_fg_color),
                    ));
                    if ui.button(t!("editor.refresh")).clicked() {
                        self.state = CodeEditorState::FirstLoad;
                    }
                });
            }
            CodeEditorState::Idle | CodeEditorState::Modified | CodeEditorState::NeedReload => {
                if let Some(goto) = self.goto.take() {
                    let mut line = 1isize;
                    let mut offset = 0usize;
                    for (i, c) in self.text.chars().enumerate() {
                        if line >= goto.line {
                            offset = i + goto.column as usize;
                            break;
                        }
                        if c == '\n' {
                            line += 1;
                        }
                    }
                    self.goto_offset = Some(offset);
                }
                let goto_offset = self.goto_offset.take();
                egui::ScrollArea::both()
                    .id_source(format!("code-window-{}", self.file))
                    .show(ui, |ui| {
                        let output = code_view_ui(ui, &mut self.text, goto_offset);
                        let update_outdated = || {
                            if let Ok(file) = std::fs::File::open(self.file.as_str()) {
                                if let Ok(meta) = file.metadata() {
                                    if let Ok(last_modified) = meta.modified() {
                                        if last_modified > self.read_time.unwrap() {
                                            return true;
                                        }
                                    }
                                }
                            }
                            false
                        };
                        if output.response.changed() {
                            if update_outdated() {
                                self.state = CodeEditorState::NeedReload;
                            }
                        }
                        if output.response.changed() && self.state != CodeEditorState::NeedReload {
                            self.state = CodeEditorState::Modified;
                        }
                        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
                            if update_outdated() {
                                self.state = CodeEditorState::NeedReload;
                            }
                            match self.state {
                                CodeEditorState::Modified => {
                                    if let Ok(mut file) = std::fs::File::create(self.file.as_str())
                                    {
                                        file.write_all(self.text.as_bytes()).unwrap();
                                    }
                                }
                                CodeEditorState::NeedReload => {}
                                _ => {}
                            }
                        }
                    });
            }
        });
    }
}
