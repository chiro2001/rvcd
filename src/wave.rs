use crate::radix::vcd_vector_to_string_n;
use anyhow::Result;
use log::info;
use std::collections::HashMap;
use std::io::Read;
use std::slice::Iter;
use vcd::Command::{ChangeScalar, ChangeVector, Timestamp};
use vcd::ScopeItem::{Scope, Var};
use vcd::{Header, IdCode, ScopeItem};

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
        }
    }
    let mut map = HashMap::new();
    add_to_map(&mut map, header.items.iter());
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
                vcd_vector_to_string_n(v, 4),
                get_name(i)
            ),
            c => println!("unknown: {:#?}", c),
        }
        cache.push(command);
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::wave::vcd_read;
    use anyhow::Result;
    use std::fs::File;

    fn init() {
        std::env::set_var("RUST_LOG", "debug");
        tracing_subscriber::fmt::init();
    }

    #[test]
    fn test_vcd() -> Result<()> {
        init();
        let mut input = File::open("data/cpu_ila_commit.vcd")?;
        vcd_read(&mut input)?;
        Ok(())
    }
}
