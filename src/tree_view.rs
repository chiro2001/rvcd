use crate::wave::WaveTreeNode;
use egui::{CollapsingHeader, PointerButton, Sense, Ui};
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
            let response = ui.add(egui::Label::new(node.to_string()).sense(Sense::click()));
            if response.double_clicked() {
                TreeAction::AddSignal(node.clone())
            } else {
                TreeAction::None
            }
        } else {
            let scope = CollapsingHeader::new(tree.data().to_string())
                .default_open(true)
                .show(ui, |ui| {
                    tree.iter()
                        .map(|child| self.ui(ui, child))
                        .find(|a| *a != TreeAction::None)
                });
            let child_signals = || tree.iter()
                .map(|n| n.data().clone())
                .map(|x| match x {
                    WaveTreeNode::WaveVar(x) => {
                        Some(WaveTreeNode::WaveVar(x))
                    }
                    _ => None,
                })
                .filter(|x| x.is_some())
                .map(|x| x.unwrap())
                .collect::<Vec<_>>();
            if scope.header_response.clicked() {
                TreeAction::SelectScope(child_signals())
            } else {
                if scope.header_response.clicked_by(PointerButton::Secondary) {
                    TreeAction::AddSignals(child_signals())
                } else {
                    match scope.body_returned {
                        None => TreeAction::None,
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
