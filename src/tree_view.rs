use crate::view::{BG_MULTIPLY, SIGNAL_TREE_HEIGHT_DEFAULT};
use crate::wave::WaveTreeNode;
use egui::{vec2, Align2, CollapsingHeader, Color32, PointerButton, Pos2, Response, Sense, Ui};
use trees::Node;

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct TreeView {
    pub show_leaves: bool,
    pub highlight_scope_id: Option<u64>,
}

#[derive(PartialEq)]
pub enum TreeAction {
    None,
    AddSignal(WaveTreeNode),
    AddSignals(Vec<WaveTreeNode>),
    SelectScope(Vec<WaveTreeNode>),
}

impl TreeView {
    pub fn ui(&mut self, ui: &mut Ui, tree: &Node<WaveTreeNode>) -> TreeAction {
        let child_signals = |tree: &Node<WaveTreeNode>| {
            tree.iter()
                .map(|n| n.data().clone())
                .map(|x| match x {
                    WaveTreeNode::WaveVar(x) => Some(WaveTreeNode::WaveVar(x)),
                    _ => None,
                })
                .filter(|x| x.is_some())
                .map(|x| x.unwrap())
                .collect::<Vec<_>>()
        };
        let recurse_child_signals = |tree: &Node<WaveTreeNode>| {
            let mut queue = vec![tree];
            let mut signal_collect = vec![];
            while !queue.is_empty() {
                let top = queue.last().unwrap().clone();
                queue.remove(queue.len() - 1);
                queue.extend(top.iter().filter(|x| match x.data() {
                    WaveTreeNode::WaveScope(_) => true,
                    _ => false,
                }));
                signal_collect.extend(child_signals(top));
            }
            signal_collect
        };
        let handle_scope_response = |response: Response| {
            let mut add_all = false;
            let mut recurse_add_all = false;
            response.context_menu(|ui| {
                if ui.button("Add all").clicked() {
                    add_all = true;
                    ui.close_menu();
                }
                if ui.button("Recurse add all").clicked() {
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
        if tree.has_no_child()
            || (!self.show_leaves
                && !tree.iter().any(|x| match x.data() {
                    WaveTreeNode::WaveScope(_) => true,
                    _ => false,
                }))
        {
            // paint as leaf
            let node = tree.data();
            let painter = ui.painter();
            let get_text_size = |text: &str| {
                painter.text(
                    Pos2::ZERO,
                    Align2::RIGHT_BOTTOM,
                    text,
                    Default::default(),
                    Color32::TRANSPARENT,
                )
            };
            let text = node.to_string();
            let text_size = get_text_size(text.as_str()).size();
            let (response, painter) = ui.allocate_painter(
                vec2(
                    f32::max(ui.max_rect().width(), text_size.x),
                    f32::max(SIGNAL_TREE_HEIGHT_DEFAULT, text_size.y),
                ),
                Sense::click_and_drag(),
            );
            let mut text_color = if ui.rect_contains_pointer(response.rect) {
                ui.visuals().strong_text_color()
            } else {
                ui.visuals().text_color()
            };
            if let Some(highlight_scope_id) = self.highlight_scope_id {
                text_color = match node {
                    WaveTreeNode::WaveScope(s) => {
                        if s.id == highlight_scope_id {
                            painter.rect_filled(
                                response.rect,
                                0.0,
                                Color32::YELLOW.linear_multiply(BG_MULTIPLY),
                            )
                        }
                        ui.visuals().hyperlink_color
                    }
                    _ => text_color,
                };
            }
            painter.text(
                response.rect.left_center(),
                Align2::LEFT_CENTER,
                text,
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
            match tree.data() {
                WaveTreeNode::WaveRoot => tree
                    .iter()
                    .map(|child| self.ui(ui, child))
                    .find(|a| *a != TreeAction::None)
                    .unwrap_or(TreeAction::None),
                data => {
                    let scope = CollapsingHeader::new(data.to_string())
                        .default_open(true)
                        .show(ui, |ui| {
                            tree.iter()
                                .map(|child| self.ui(ui, child))
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
            .checkbox(&mut self.show_leaves, "Show Tree Leaves")
            .clicked()
        {
            ui.close_menu();
        }
    }
}
