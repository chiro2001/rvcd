use crate::radix::{radix_value_big_uint, radix_vector_to_string, Radix};
use anyhow::{anyhow, Result};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Read;
use tracing::info;
use trees::Tree;

pub mod utils;
pub mod vcd;

/// like [vcd::Value], basically for (de)serialize
#[derive(Default, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug)]
pub enum WireValue {
    #[default]
    V0,
    V1,
    X,
    Z,
}

impl Display for WireValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WireValue::V0 => "0",
                WireValue::V1 => "1",
                WireValue::X => "x",
                WireValue::Z => "z",
            }
        )
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum WaveTimescaleUnit {
    S,
    MS,
    US,
    NS,
    #[default]
    PS,
    FS,
}
impl Display for WaveTimescaleUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_ascii_lowercase())
    }
}
impl WaveTimescaleUnit {
    pub fn smaller(&self) -> Option<Self> {
        use WaveTimescaleUnit::*;
        match self {
            S => Some(MS),
            MS => Some(US),
            US => Some(NS),
            NS => Some(PS),
            PS => Some(FS),
            FS => None,
        }
    }
    pub fn larger(&self) -> Option<Self> {
        use WaveTimescaleUnit::*;
        match self {
            S => None,
            MS => Some(S),
            US => Some(MS),
            NS => Some(US),
            PS => Some(NS),
            FS => Some(PS),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WaveDataValue {
    /// when vec empty, invalid
    Comp(Vec<u8>),
    Raw(Vec<WireValue>),
}

impl Into<Option<BigUint>> for &WaveDataValue {
    fn into(self) -> Option<BigUint> {
        match self {
            WaveDataValue::Comp(v) => Some(BigUint::from_bytes_le(v.as_slice())),
            _ => None,
        }
    }
}

impl WaveDataValue {
    pub fn as_radix(&self, radix: Radix) -> String {
        match self {
            WaveDataValue::Comp(v) => BigUint::from_bytes_le(v).to_str_radix(radix.to_number() as u32),
            WaveDataValue::Raw(v) => radix_vector_to_string(radix, v),
        }
    }
}

impl Display for WaveDataValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_radix(Radix::Hex))
    }
}

impl Default for WaveDataValue {
    fn default() -> Self {
        Self::Raw(vec![])
    }
}

/// item struct in data list
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct WaveDataItem {
    pub id: u64,
    pub value: WaveDataValue,
    pub timestamp: u64,
}

impl Display for WaveDataItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{} [{}] {}", self.timestamp, self.id, self.value)
    }
}

impl WaveDataItem {
    fn compress(self) -> Result<Self> {
        if match &self.value {
            WaveDataValue::Comp(v) => v.len(),
            WaveDataValue::Raw(v) => v.len(),
        } == 0
        {
            return Err(anyhow!("compressing invalid data!"));
        }
        match &self.value {
            WaveDataValue::Comp(_) => Ok(self),
            WaveDataValue::Raw(v) => {
                let ability = !v.iter().any(|i| i == &WireValue::X || i == &WireValue::Z);
                if ability {
                    let value = WaveDataValue::Comp(radix_value_big_uint(v).to_bytes_le());
                    Ok(Self { value, ..self })
                } else {
                    Ok(self)
                }
            }
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Default, Debug, PartialEq)]
pub struct WaveSignalInfo {
    pub id: u64,
    pub name: String,
    pub width: u64,
}
impl Display for WaveSignalInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.width {
                0 | 1 => self.name.to_string(),
                _ => format!("{}[{}:0]", self.name, self.width - 1),
            }
        )
    }
}

#[derive(Serialize, Clone, Default, Debug, PartialEq)]
pub enum WaveTreeNode {
    #[default]
    WaveRoot,
    WaveScope(String),
    WaveVar(WaveSignalInfo),
    // id only to save space
    WaveId(u64),
}

impl Display for WaveTreeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WaveTreeNode::WaveRoot => "root".to_string(),
                WaveTreeNode::WaveScope(scope) => scope.to_string(),
                WaveTreeNode::WaveVar(i) => i.to_string(),
                WaveTreeNode::WaveId(var) => format!("{}", var),
            }
        )
    }
}

#[derive(Clone, Debug)]
pub struct WaveInfo {
    pub timescale: (u64, WaveTimescaleUnit),
    pub range: (u64, u64),
    pub headers: HashMap<String, String>,
    pub code_name_width: HashMap<u64, (String, u64)>,
    pub code_paths: HashMap<u64, Vec<String>>,
    pub tree: Tree<WaveTreeNode>,
}

impl WaveInfo {
    pub fn copy(&self) -> Self {
        Self {
            timescale: self.timescale,
            range: self.range,
            headers: self.headers.clone(),
            code_name_width: self.code_name_width.clone(),
            code_paths: self.code_paths.clone(),
            tree: self.tree.deep_clone(),
        }
    }
}

/// loaded wave data in memory
#[derive(Clone)]
pub struct Wave {
    pub info: WaveInfo,
    pub data: Vec<WaveDataItem>,
}

impl Display for WaveInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Wave {}{} #{}~#{} {:?}",
            self.timescale.0, self.timescale.1, self.range.0, self.range.1, self.headers
        )
    }
}

impl Display for Wave {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.info.fmt(f)
    }
}

pub trait WaveLoader {
    fn load(reader: &mut dyn Read) -> Result<Wave>;
}

#[cfg(test)]
mod test {
    use crate::wave::vcd::Vcd;
    use crate::wave::WaveLoader;
    use std::fs::File;
    // use trees::Node;
    use crate::wave::utils::Node;

    #[test]
    fn test_load_wave() -> anyhow::Result<()> {
        let mut input = File::open("data/cpu_ila_commit.vcd")?;
        let wave = Vcd::load(&mut input)?;
        println!("loaded wave: {}", wave);
        // for item in &wave.data {
        //     println!("item: {}", item);
        // }
        println!("code paths:");
        for (id, path) in wave.info.code_paths.iter() {
            println!(
                "code: {}, name: {:?}, path: {:?}",
                id,
                wave.info.code_name_width.get(id).unwrap(),
                path
            );
        }
        println!("tree:");
        println!(
            "{}",
            serde_json::to_string(&Node(wave.info.tree.root())).unwrap()
        );

        use trees::tr;

        let tree = tr(0) / (tr(1) / tr(2) / tr(3)) / (tr(4) / tr(5) / tr(6));
        println!("{}", serde_json::to_string(&Node(tree.root())).unwrap());
        Ok(())
    }
}
