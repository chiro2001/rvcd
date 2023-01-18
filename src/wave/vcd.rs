use crate::wave::WaveDataValue::Raw;
use crate::wave::{Wave, WaveDataItem, WaveLoader, WaveTimescaleUnit, WireValue};
use anyhow::Result;
use log::info;
use std::collections::HashMap;
use std::io::Read;
use std::slice::Iter;
use vcd::ScopeItem::{Scope, Var};
use vcd::{Command, Header, IdCode, ScopeItem, TimescaleUnit, Value};

pub fn vcd_header_show(header: &Header) {
    if let Some(c) = header.comment.as_ref() {
        info!("comment: {}", c)
    }
    if let Some(c) = header.date.as_ref() {
        info!("date: {}", c)
    }
    if let Some(c) = header.version.as_ref() {
        info!("version: {}", c)
    }
    if let Some(c) = header.timescale.as_ref() {
        info!("timescale: {} / {}", c.0, c.1)
    }
}

pub fn vcd_tree_show(header: &Header) {
    fn show(item: &ScopeItem, level: usize) {
        match item {
            Scope(scope) => {
                println!(
                    "{}{}",
                    (0..level).map(|_| "\t").collect::<Vec<&str>>().join(""),
                    scope.identifier
                );
                scope.children.iter().for_each(|i| show(i, level + 1));
            }
            Var(var) => {
                println!(
                    "{}{} width={}",
                    (0..level).map(|_| "\t").collect::<Vec<&str>>().join(""),
                    var.reference,
                    var.size
                );
            }
            _ => {}
        }
    }
    header.items.iter().for_each(|item| show(item, 0));
}

pub fn vcd_code_name(header: &Header) -> HashMap<IdCode, String> {
    fn add_to_map(m: &mut HashMap<IdCode, String>, it: Iter<'_, ScopeItem>) {
        it.for_each(|c| {
            iterate(c).iter().for_each(|(k, v)| {
                m.insert(*k, v.to_string());
            })
        });
    }
    fn iterate(item: &ScopeItem) -> HashMap<IdCode, String> {
        match item {
            Scope(scope) => {
                let mut m: HashMap<IdCode, String> = HashMap::new();
                add_to_map(&mut m, scope.children.iter());
                m
            }
            Var(var) => HashMap::from([(var.code, var.reference.to_string())]),
            _ => HashMap::new(),
        }
    }
    let mut map = HashMap::new();
    add_to_map(&mut map, header.items.iter());
    map
}

impl From<Value> for WireValue {
    fn from(value: Value) -> Self {
        match value {
            Value::V0 => WireValue::V0,
            Value::V1 => WireValue::V1,
            Value::X => WireValue::X,
            Value::Z => WireValue::Z,
        }
    }
}

impl From<TimescaleUnit> for WaveTimescaleUnit {
    fn from(value: TimescaleUnit) -> Self {
        match value {
            TimescaleUnit::S => WaveTimescaleUnit::S,
            TimescaleUnit::MS => WaveTimescaleUnit::MS,
            TimescaleUnit::US => WaveTimescaleUnit::US,
            TimescaleUnit::NS => WaveTimescaleUnit::NS,
            TimescaleUnit::PS => WaveTimescaleUnit::PS,
            TimescaleUnit::FS => WaveTimescaleUnit::FS,
        }
    }
}

pub struct Vcd;
impl WaveLoader for Vcd {
    fn load(reader: &mut dyn Read) -> Result<Wave> {
        let mut parser = vcd::Parser::new(reader);
        let header = parser.parse_header()?;
        let code_names = vcd_code_name(&header)
            .into_iter()
            .map(|i| {
                let IdCode(id) = i.0;
                (id, i.1)
            })
            .collect();
        let mut headers: HashMap<String, String> = HashMap::new();
        if let Some(c) = header.comment.as_ref() {
            headers.insert("comment".to_string(), c.to_string());
        }
        if let Some(c) = header.date.as_ref() {
            headers.insert("date".to_string(), c.to_string());
        }
        if let Some(c) = header.version.as_ref() {
            headers.insert("version".to_string(), c.to_string());
        }
        let timescale = if let Some(c) = header.timescale.as_ref() {
            (c.0 as u64, c.1.into())
        } else {
            (1, WaveTimescaleUnit::default())
        };
        let mut data = vec![];
        let mut timestamp = 0u64;
        for command_result in parser {
            let command = command_result?;
            match command {
                Command::Timestamp(t) => timestamp = t,
                Command::ChangeScalar(i, v) => {
                    let IdCode(id) = i;
                    data.push(
                        WaveDataItem {
                            id,
                            value: Raw(vec![v.into()]),
                            timestamp,
                        }
                        .compress()?,
                    );
                }
                Command::ChangeVector(i, v) => {
                    let IdCode(id) = i;
                    data.push(
                        WaveDataItem {
                            id,
                            value: Raw(v.into_iter().map(|x| x.into()).collect()),
                            timestamp,
                        }
                        .compress()?,
                    );
                }
                Command::ChangeReal(_, _) => {}
                Command::ChangeString(_, _) => {}
                _ => {}
            }
        }
        Ok(Wave {
            timescale,
            headers,
            code_names,
            data,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::radix::radix_vector_to_string_n;
    use crate::wave::vcd::{vcd_code_name, vcd_header_show, vcd_tree_show};
    use anyhow::Result;
    use std::fs::File;
    use std::io::Read;
    use vcd::Command::{ChangeScalar, ChangeVector, Timestamp};
    use vcd::IdCode;

    fn init() {
        std::env::set_var("RUST_LOG", "debug");
        tracing_subscriber::fmt::init();
    }

    pub fn vcd_read(r: &mut dyn Read) -> Result<()> {
        let mut parser = vcd::Parser::new(r);
        let header = parser.parse_header()?;
        vcd_header_show(&header);
        vcd_tree_show(&header);
        let mut cache = vec![];
        let code_name = vcd_code_name(&header);
        for command_result in parser {
            let command = command_result?;
            let get_name = |code: &IdCode| match code_name.get(code) {
                Some(v) => v,
                None => "None",
            };
            match &command {
                Timestamp(i) => println!("#{}", i),
                ChangeScalar(i, v) => println!("code={}, value={}, name={}", i, v, get_name(i)),
                ChangeVector(i, v) => println!(
                    "code={}, value={}, name={}",
                    i,
                    radix_vector_to_string_n(&v.iter().map(|x| (*x).into()).collect(), 4),
                    get_name(i)
                ),
                c => println!("unknown: {:#?}", c),
            }
            cache.push(command);
        }
        Ok(())
    }

    #[test]
    fn test_vcd() -> Result<()> {
        init();
        let mut input = File::open("data/cpu_ila_commit.vcd")?;
        vcd_read(&mut input)?;
        Ok(())
    }
}
