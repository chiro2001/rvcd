use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;

pub mod vcd;

/// like [vcd::Value], basically for (de)serialize
#[derive(Default, Serialize, Deserialize)]
pub enum WireValue {
    #[default]
    V0,
    V1,
    X,
    Z,
}

#[derive(Default, Serialize, Deserialize)]
pub enum WaveTimescaleUnit {
    S,
    MS,
    US,
    NS,
    #[default]
    PS,
    FS,
}

#[derive(Serialize, Deserialize)]
pub enum WaveDataValue {
    /// when vec empty, invalid
    Comp(Vec<u8>),
    Raw(Vec<WireValue>),
}

impl Default for WaveDataValue {
    fn default() -> Self {
        Self::Comp(vec![])
    }
}

/// item struct in data list
#[derive(Default, Serialize, Deserialize)]
pub struct WaveDataItem {
    id: u64,
    value: WaveDataValue,
}

/// loaded wave data in memory
#[derive(Default, Serialize, Deserialize)]
pub struct Wave {
    timescale: (u64, WaveTimescaleUnit),
    headers: HashMap<String, String>,
    code_names: HashMap<u64, String>,
    data: Vec<WaveDataItem>,
}

pub trait WaveLoader {
    fn load(reader: &mut dyn Read) -> Result<Wave>;
}
