use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Default, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum FileSizeUnit {
    MiB,
    #[default]
    KiB,
    Byte,
}
impl Display for FileSizeUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self))
    }
}
impl FileSizeUnit {
    pub fn smaller(&self) -> Option<Self> {
        use FileSizeUnit::*;
        match self {
            MiB => Some(KiB),
            KiB => Some(Byte),
            Byte => None,
        }
    }
    pub fn larger(&self) -> Option<Self> {
        use FileSizeUnit::*;
        match self {
            MiB => None,
            KiB => Some(MiB),
            Byte => Some(KiB),
        }
    }
    pub fn from_bytes(bytes: usize) -> String {
        let mut sz = bytes;
        let mut unit = Self::Byte;
        while let Some(u) = unit.larger() {
            if sz < 1024 {
                break;
            }
            sz /= 1024;
            unit = u;
        }
        format!("{} {}", sz, unit)
    }
}
