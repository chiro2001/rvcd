use crate::wave::WaveTreeNode;
use egui::{CollapsingHeader, Ui};
use trees::{Node, Tree};

pub struct TreeView(Tree<WaveTreeNode>);

impl Default for TreeView {
    fn default() -> Self {
        Self(Tree::new(WaveTreeNode::WaveRoot))
    }
}

pub enum TreeAction {
    Keep,
    Delete,
}

impl TreeView {
    pub fn item_ui(&mut self, ui: &mut Ui, tree: &Node<WaveTreeNode>) {
        if tree.has_no_child() {
            ui.label(tree.data().to_string());
        } else {
            CollapsingHeader::new(tree.data().to_string())
                .default_open(true)
                .show(ui, |ui| {
                    for child in tree.iter() {
                        self.item_ui(ui, child)
                    }
                });
        }
    }
    pub fn ui(&mut self, ui: &mut Ui) -> TreeAction {
        self.ui_impl(ui, self.0.root().data().to_string().as_str(), true)
    }
    fn ui_impl(&mut self, ui: &mut Ui, name: &str, default_open: bool) -> TreeAction {
        CollapsingHeader::new(name)
            .default_open(default_open)
            .show(ui, |ui| self.children_ui(ui))
            .body_returned
            .unwrap_or(TreeAction::Keep)
    }
    fn children_ui(&mut self, _ui: &mut Ui) -> TreeAction {
        // self.0 = std::mem::take(self)
        //     .0
        //     .into_iter()
        //     .enumerate()
        //     .filter_map(|(i, mut tree)| {
        //         if tree.
        //     });
        TreeAction::Keep
    }
}
