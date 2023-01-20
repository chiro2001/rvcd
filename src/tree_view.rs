use crate::wave::WaveTreeNode;
use egui::{CollapsingHeader, Sense, Ui};
use log::info;
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
    ClickLeaf(WaveTreeNode),
    Select(WaveTreeNode),
}

impl TreeView {
    pub fn ui(&mut self, ui: &mut Ui, tree: &Node<WaveTreeNode>) -> TreeAction {
        if tree.has_no_child() {
            let node = tree.data();
            // if ui.button(node.to_string()).clicked() {
            // let button = ui.label(node.to_string());
            let response = ui.add(
                egui::Label::new(node.to_string())
                    .sense(Sense::click())
            );
            // button.sense.click
            if response.clicked() {
                // info!("clicked: {}", node);
                TreeAction::ClickLeaf(node.clone())
            } else {
                TreeAction::None
            }
        } else {
            match CollapsingHeader::new(tree.data().to_string())
                .default_open(true)
                .show(ui, |ui| {
                    tree.iter()
                        .map(|child| self.ui(ui, child))
                        .find(|a| *a != TreeAction::None)
                })
                .body_returned
            {
                None => TreeAction::None,
                Some(a) => match a {
                    None => TreeAction::None,
                    Some(a) => a,
                },
            }
        }
    }
}
