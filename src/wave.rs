use std::collections::HashMap;
use std::io::Read;
use anyhow::Result;
use log::info;
use vcd::{Header, IdCode, ScopeItem};
use vcd::Command::{ChangeScalar, ChangeVector, Timestamp};
use vcd::ScopeItem::{Scope, Var};

pub fn vcd_header_show(header: &Header) {
    header.comment.as_ref().map(|c| info!("comment: {}", c));
    header.date.as_ref().map(|c| info!("date: {}", c));
    header.version.as_ref().map(|c| info!("version: {}", c));
    header.timescale.as_ref().map(|c| info!("timescale: {} / {}", c.0, c.1));
}

pub fn vcd_tree_show(header: &Header) {
    fn show(item: &ScopeItem, level: usize) {
        match item {
            Scope(scope) => {
                println!("{}{}", (0..level).map(|_| "\t").collect::<Vec<&str>>().join(""), scope.identifier);
                scope.children.iter().for_each(|i| show(i, level + 1));
            }
            Var(var) => {
                println!("{}{} width={}",
                         (0..level).map(|_| "\t").collect::<Vec<&str>>().join(""),
                         var.reference, var.size);
            }
        }
    }
    header.items.iter().for_each(|item| show(item, 0));
}

pub fn vcd_code_name(header: &Header) -> HashMap<IdCode, String> {
    fn iterate(item: &ScopeItem) -> HashMap<IdCode, String> {
        match item {
            Scope(scope) => {
                let mut m: HashMap<IdCode, String> = HashMap::new();
                scope.children.iter().for_each(|c| iterate(c).iter().for_each(|(k, v)| {
                    m.insert(*k, v.to_string());
                }));
                m
            }
            Var(var) => HashMap::from([(var.code, var.reference.to_string())])
        }
    }
    let mut map = HashMap::new();
    header.items.iter().for_each(|i| iterate(i).iter().for_each(|(k, v)| {
        map.insert(*k, v.to_string());
    }));
    map
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
        match &command {
            Timestamp(i) => println!("#{}", i),
            ChangeScalar(i, v) => println!("code={}, value={}, name={}", i, v, match code_name.get(&i) {
                Some(v) => v,
                None => "None"
            }),
            ChangeVector(i, v) => {}
            c => println!("unknown: {:#?}", c)
        }
        cache.push(command);
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use anyhow::Result;
    use crate::wave::vcd_read;

    fn init() {
        std::env::set_var("RUST_LOG", "debug");
        env_logger::init();
    }

    #[test]
    fn test_vcd() -> Result<()> {
        init();
        let mut input = File::open("data/cpu_ila_commit.vcd")?;
        vcd_read(&mut input)?;
        Ok(())
    }
}