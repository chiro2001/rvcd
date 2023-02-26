use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

pub mod editor;
pub mod highlight;

#[derive(Default, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum CodeEditorType {
    #[default]
    Internal,
    Scaleda,
    VsCode,
}
impl Display for CodeEditorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
