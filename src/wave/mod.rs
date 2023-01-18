use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
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

#[derive(Default, Serialize, Deserialize, Debug)]
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
    timestamp: u64,
}

/// loaded wave data in memory
#[derive(Default, Serialize, Deserialize)]
pub struct Wave {
    timescale: (u64, WaveTimescaleUnit),
    headers: HashMap<String, String>,
    code_names: HashMap<u64, String>,
    data: Vec<WaveDataItem>,
}

impl Display for Wave {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Wave {}{} {:?}", self.timescale.0, self.timescale.1, self.headers)
    }
}

pub trait WaveLoader {
    fn load(reader: &mut dyn Read) -> Result<Wave>;
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use crate::wave::vcd::Vcd;
    use crate::wave::WaveLoader;

    #[test]
    fn test_load_vcd() -> anyhow::Result<()> {
        let mut input = File::open("data/cpu_ila_commit.vcd")?;
        let wave = Vcd::load(&mut input)?;
        println!("loaded wave: {}", wave);
        Ok(())
    }
}
