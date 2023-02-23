use crate::radix::{radix_value_big_uint, radix_vector_to_string, Radix};
use anyhow::{anyhow, Result};
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Read;
use trees::Tree;
use vcd::{IdCode, Scope, ScopeType, Var, VarType};

pub mod utils;
pub mod vcd_parser;

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
        write!(f, "{}", format!("{self:?}").to_ascii_lowercase())
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

impl From<&WaveDataValue> for Option<BigUint> {
    fn from(val: &WaveDataValue) -> Self {
        match val {
            WaveDataValue::Comp(v) => Some(BigUint::from_bytes_le(v.as_slice())),
            _ => None,
        }
    }
}

impl WaveDataValue {
    /// to string in radix
    pub fn as_radix(&self, radix: Radix) -> String {
        match self {
            WaveDataValue::Comp(v) => {
                BigUint::from_bytes_le(v).to_str_radix(radix.to_number() as u32)
            }
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
    // pub id: u64,
    pub value: WaveDataValue,
    pub timestamp: u64,
}

impl Display for WaveDataItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{} {}", self.timestamp, self.value)
    }
}

impl WaveDataItem {
    /// Compress [WaveDataItem], may change `Raw(_)` to `Comp(_)`
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
pub enum WaveSignalType {
    Event,
    Integer,
    Parameter,
    Real,
    #[default]
    Reg,
    Supply0,
    Supply1,
    Time,
    Tri,
    TriAnd,
    TriOr,
    TriReg,
    Tri0,
    Tri1,
    WAnd,
    Wire,
    WOr,
    String,
}
impl Display for WaveSignalType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{self:?}").to_ascii_lowercase())
    }
}
impl From<VarType> for WaveSignalType {
    fn from(value: VarType) -> Self {
        match value {
            VarType::Event => Self::Event,
            VarType::Integer => Self::Integer,
            VarType::Parameter => Self::Parameter,
            VarType::Real => Self::Real,
            VarType::Reg => Self::Reg,
            VarType::Supply0 => Self::Supply0,
            VarType::Supply1 => Self::Supply1,
            VarType::Time => Self::Time,
            VarType::Tri => Self::Tri,
            VarType::TriAnd => Self::TriAnd,
            VarType::TriOr => Self::TriOr,
            VarType::TriReg => Self::TriReg,
            VarType::Tri0 => Self::Tri0,
            VarType::Tri1 => Self::Tri1,
            VarType::WAnd => Self::WAnd,
            VarType::Wire => Self::Wire,
            VarType::WOr => Self::WOr,
            VarType::String => Self::String,
            _ => panic!("error converting var type"),
        }
    }
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Default, Debug, PartialEq)]
pub enum WaveScopeType {
    #[default]
    Module,
    Task,
    Function,
    Begin,
    Fork,
}
impl From<ScopeType> for WaveScopeType {
    fn from(value: ScopeType) -> Self {
        match value {
            ScopeType::Module => Self::Module,
            ScopeType::Task => Self::Task,
            ScopeType::Function => Self::Function,
            ScopeType::Begin => Self::Begin,
            ScopeType::Fork => Self::Fork,
            _ => panic!("error converting scope type"),
        }
    }
}
impl Display for WaveScopeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{self:?}").to_ascii_lowercase())
    }
}
#[derive(serde::Deserialize, serde::Serialize, Clone, Default, Debug, PartialEq)]
pub struct WaveScopeInfo {
    pub id: u64,
    pub name: String,
    pub typ: WaveScopeType,
}
impl WaveScopeInfo {
    fn from_scope(id: u64, value: &Scope) -> Self {
        Self {
            id,
            name: value.identifier.to_string(),
            typ: value.scope_type.into(),
        }
    }
}
impl Display for WaveScopeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Default, Debug, PartialEq)]
pub struct WaveSignalInfo {
    pub id: u64,
    pub name: String,
    pub width: u64,
    pub typ: WaveSignalType,
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
impl From<&Var> for WaveSignalInfo {
    fn from(value: &Var) -> Self {
        let IdCode(id) = value.code;
        Self {
            id,
            name: value.reference.to_string(),
            width: value.size.into(),
            typ: value.var_type.into(),
        }
    }
}

#[derive(Serialize, Clone, Default, Debug, PartialEq)]
pub enum WaveTreeNode {
    #[default]
    WaveRoot,
    WaveScope(WaveScopeInfo),
    WaveVar(WaveSignalInfo),
    /// id only to save space (Not available now)
    WaveId(u64),
}

impl Display for WaveTreeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WaveTreeNode::WaveRoot => "root".to_string(),
                WaveTreeNode::WaveScope(s) => s.to_string(),
                WaveTreeNode::WaveVar(i) => i.to_string(),
                WaveTreeNode::WaveId(var) => format!("{var}"),
            }
        )
    }
}

#[derive(Clone, Debug)]
pub struct WaveInfo {
    pub timescale: (u64, WaveTimescaleUnit),
    /// Position range
    pub range: (u64, u64),
    /// Extra information
    pub headers: HashMap<String, String>,
    /// Signal info: (name, width) indexed by id
    pub code_signal_info: HashMap<u64, WaveSignalInfo>,
    /// Signal path indexed by id
    pub code_paths: HashMap<u64, Vec<String>>,
    /// Signal scope and vars tree
    pub tree: Tree<WaveTreeNode>,
}

/// loaded wave data in memory
#[derive(Clone)]
pub struct Wave {
    pub info: WaveInfo,
    pub data: HashMap<u64, Vec<WaveDataItem>>,
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

impl Wave {
    /// Find *nearest* value
    pub fn find_value(&self, id: u64, pos: u64) -> Option<WaveDataItem> {
        // assert: pos keeps increase
        if let Some(data) = self.data.get(&id) {
            match data.binary_search_by_key(&pos, |x| x.timestamp) {
                Ok(index) => {
                    if data[index].timestamp == pos {
                        Some(data[index].clone())
                    } else {
                        data.get(index - 1).cloned()
                    }
                }
                Err(index) => data.get(index - 1).cloned(),
            }
        } else {
            None
        }
    }
}

/// To support other file formats
pub trait WaveLoader {
    fn load<F>(
        reader: &mut dyn Read,
        progress_handler: F,
        last_timestamp: Option<u64>,
    ) -> Result<Wave>
    where
        F: Fn(f32, u64);
}

/// To support preloader
pub trait WavePreLoader {
    fn last_timestamp<T>(
        reader: std::io::BufReader<T>,
    ) -> (Option<u64>, std::io::Result<std::io::BufReader<T>>)
    where
        T: Read + std::io::Seek;
}

#[cfg(test)]
mod test {
    use crate::wave::vcd_parser::Vcd;
    use crate::wave::WaveLoader;
    use std::fs::File;
    // use trees::Node;
    use crate::wave::utils::Node;

    #[test]
    fn test_load_wave() -> anyhow::Result<()> {
        let mut input = File::open("data/cpu_ila_commit.vcd")?;
        let wave = Vcd::load(&mut input)?;
        println!("loaded wave: {wave}");
        // for item in &wave.data {
        //     println!("item: {}", item);
        // }
        println!("code paths:");
        for (id, path) in wave.info.code_paths.iter() {
            println!(
                "code: {}, name: {:?}, path: {:?}",
                id,
                wave.info.code_signal_info.get(id).unwrap(),
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
