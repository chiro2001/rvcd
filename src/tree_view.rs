use crate::wave::WaveTreeNode;
use egui::{CollapsingHeader, Sense, Ui};
use trees::{Node, Tree};

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
            if scope.header_response.clicked() {
                TreeAction::SelectScope(
                    tree.iter()
                        .map(|n| n.data().clone())
                        .map(|x| match x {
                            WaveTreeNode::WaveVar(x) => {
                                Some(WaveTreeNode::WaveVar((x.0, x.1.to_string())))
                            }
                            _ => None,
                        })
                        .filter(|x| x.is_some())
                        .map(|x| x.unwrap())
                        .collect(),
                )
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
