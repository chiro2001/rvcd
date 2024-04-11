use crate::utils::get_text_size;
use crate::view::{BG_MULTIPLY, SIGNAL_TREE_HEIGHT_DEFAULT, TEXT_BG_MULTIPLY};
use crate::wave::{WaveScopeType, WaveTreeNode};
use egui::{vec2, Align2, CollapsingHeader, Color32, PointerButton, Response, Sense, Ui};
use regex::Regex;
use trees::Node;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TreeView {
    pub show_tail_leaves: bool,
    pub show_leaves: bool,
    pub show_modules_only: bool,
    pub highlight_scope_id: Option<u64>,
}
impl Default for TreeView {
    fn default() -> Self {
        Self {
            show_tail_leaves: false,
            show_leaves: false,
            show_modules_only: true,
            highlight_scope_id: None,
        }
    }
}

#[derive(PartialEq)]
pub enum TreeAction {
    None,
    AddSignal(WaveTreeNode),
    AddSignals(Vec<WaveTreeNode>),
    SelectScope(Vec<WaveTreeNode>),
}

impl TreeView {
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        tree: &Node<WaveTreeNode>,
        search_text: &str,
        is_regex: bool,
    ) -> TreeAction {
        let child_signals = |tree: &Node<WaveTreeNode>| {
            tree.iter()
                .map(|n| n.data().clone())
                .filter_map(|x| match x {
                    WaveTreeNode::WaveVar(x) => Some(WaveTreeNode::WaveVar(x)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let recurse_child_signals = |tree: &Node<WaveTreeNode>| {
            let mut queue = vec![tree];
            let mut signal_collect = vec![];
            while !queue.is_empty() {
                let top = *queue.last().unwrap();
                queue.remove(queue.len() - 1);
                queue.extend(
                    top.iter()
                        .filter(|x| matches!(x.data(), WaveTreeNode::WaveScope(_))),
                );
                signal_collect.extend(child_signals(top));
            }
            signal_collect
        };
        let handle_scope_response = |response: Response| {
            let mut add_all = false;
            let mut recurse_add_all = false;
            response.context_menu(|ui| {
                if ui.button(t!("sst.signal.add_all")).clicked() {
                    add_all = true;
                    ui.close_menu();
                }
                if ui.button(t!("sst.signal.recursive_add_all")).clicked() {
                    recurse_add_all = true;
                    ui.close_menu();
                }
            });
            if add_all {
                TreeAction::AddSignals(child_signals(tree))
            } else if recurse_add_all {
                TreeAction::AddSignals(recurse_child_signals(tree))
            } else {
                TreeAction::None
            }
        };
        if !tree.iter().any(|n| match n.data() {
            WaveTreeNode::WaveRoot => false,
            WaveTreeNode::WaveScope(s) => match &s.typ {
                WaveScopeType::Module => true,
                _ => !self.show_modules_only,
            },
            WaveTreeNode::WaveVar(_) => self.show_leaves,
            WaveTreeNode::WaveId(_) => false,
        }) || (!self.show_tail_leaves
            && !tree
                .iter()
                .any(|x| matches!(x.data(), WaveTreeNode::WaveScope(_))))
        {
            // paint as leaf
            let node = tree.data();
            let node_string = node.to_string();
            let regex_show = if is_regex {
                if let Ok(re) = Regex::new(search_text) {
                    if re.captures(node_string.as_str()).is_none() {
                        false
                    } else {
                        true
                    }
                } else {
                    true
                }
            } else {
                node_string.contains(search_text)
            };
            let show_item = match node {
                WaveTreeNode::WaveScope(s) => match &s.typ {
                    WaveScopeType::Module => true,
                    _ => !self.show_modules_only,
                },
                WaveTreeNode::WaveVar(_) => self.show_leaves,
                _ => false,
            };
            if show_item && regex_show {
                let text = node.to_string();
                let text_right = match node {
                    WaveTreeNode::WaveScope(s) => s.typ.to_string(),
                    WaveTreeNode::WaveVar(s) => s.typ.to_string(),
                    _ => "".to_string(),
                };
                let text_size = get_text_size(ui, text.as_str(), Default::default());
                let text_right_size = get_text_size(ui, text_right.as_str(), Default::default());
                let (response, painter) = ui.allocate_painter(
                    vec2(
                        f32::max(ui.max_rect().width(), text_size.x + text_right_size.x),
                        f32::max(SIGNAL_TREE_HEIGHT_DEFAULT, text_size.y),
                    ),
                    Sense::click_and_drag(),
                );
                let on_hover = ui.rect_contains_pointer(response.rect);
                let mut text_color = if on_hover {
                    ui.visuals().strong_text_color()
                } else {
                    ui.visuals().text_color()
                };
                text_color = match node {
                    WaveTreeNode::WaveScope(s) => {
                        if let Some(highlight_scope_id) = self.highlight_scope_id {
                            if s.id == highlight_scope_id {
                                painter.rect_filled(
                                    response.rect,
                                    0.0,
                                    Color32::YELLOW.linear_multiply(BG_MULTIPLY),
                                )
                            }
                        }
                        if on_hover {
                            ui.visuals()
                                .hyperlink_color
                                .linear_multiply(TEXT_BG_MULTIPLY)
                        } else {
                            ui.visuals().hyperlink_color
                        }
                    }
                    _ => text_color,
                };
                painter.text(
                    response.rect.left_center(),
                    Align2::LEFT_CENTER,
                    text,
                    Default::default(),
                    text_color,
                );
                painter.text(
                    response.rect.right_center(),
                    Align2::RIGHT_CENTER,
                    text_right,
                    Default::default(),
                    text_color,
                );
                match node {
                    WaveTreeNode::WaveScope(s) => {
                        if response.clicked_by(PointerButton::Primary) {
                            self.highlight_scope_id = Some(s.id);
                            if response.double_clicked() {
                                TreeAction::AddSignals(child_signals(tree))
                            } else {
                                TreeAction::SelectScope(child_signals(tree))
                            }
                        } else {
                            handle_scope_response(response)
                        }
                    }
                    _ => {
                        if response.double_clicked() {
                            TreeAction::AddSignal(node.clone())
                        } else {
                            TreeAction::None
                        }
                    }
                }
            } else {
                TreeAction::None
            }
        } else {
            match tree.data() {
                WaveTreeNode::WaveRoot => tree
                    .iter()
                    .map(|child| self.ui(ui, child, search_text, is_regex))
                    .find(|a| *a != TreeAction::None)
                    .unwrap_or(TreeAction::None),
                data => {
                    let scope = CollapsingHeader::new(data.to_string())
                        .default_open(true)
                        .show(ui, |ui| {
                            tree.iter()
                                .map(|child| self.ui(ui, child, search_text, is_regex))
                                .find(|a| *a != TreeAction::None)
                        });
                    if scope.header_response.clicked_by(PointerButton::Primary) {
                        TreeAction::SelectScope(child_signals(tree))
                    } else {
                        match scope.body_returned {
                            None => handle_scope_response(scope.header_response),
                            Some(a) => match a {
                                None => TreeAction::None,
                                Some(a) => a,
                            },
                        }
                    }
                }
            }
        }
    }
    pub fn menu(&mut self, ui: &mut Ui) {
        if ui
            .checkbox(&mut self.show_leaves, t!("sst.show_leaves"))
            .clicked()
        {
            ui.close_menu();
        }
        if ui
            .checkbox(&mut self.show_tail_leaves, t!("sst.show_tail_leaves"))
            .clicked()
        {
            ui.close_menu();
        }
        if ui
            .checkbox(&mut self.show_modules_only, t!("sst.show_modules_only"))
            .clicked()
        {
            ui.close_menu();
        }
    }
}
