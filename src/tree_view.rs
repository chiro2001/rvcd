use crate::view::SIGNAL_TREE_HEIGHT_DEFAULT;
use crate::wave::WaveTreeNode;
use egui::{vec2, Align2, CollapsingHeader, Color32, PointerButton, Pos2, Sense, Ui};
use trees::{Node, Tree};

#[derive(Debug)]
pub struct TreeView(Tree<WaveTreeNode>);

impl Default for TreeView {
    fn default() -> Self {
        Self(Tree::new(WaveTreeNode::WaveRoot))
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
    pub fn ui(&mut self, ui: &mut Ui, tree: &Node<WaveTreeNode>) -> TreeAction {
        if tree.has_no_child() {
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
            painter.text(
                response.rect.left_center(),
                Align2::LEFT_CENTER,
                text,
                Default::default(),
                ui.visuals().text_color(),
            );
            if response.double_clicked() {
                TreeAction::AddSignal(node.clone())
            } else {
                TreeAction::None
            }
        } else {
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
                            None => {
                                let mut add_all = false;
                                let mut recurse_add_all = false;
                                scope.header_response.context_menu(|ui| {
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
                            }
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
}
