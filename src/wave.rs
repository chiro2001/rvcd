use std::io::Read;
use anyhow::{anyhow, Result};
use log::info;
use vcd::{Header, ScopeItem};
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

pub fn vcd_read(r: &mut dyn Read) -> Result<()> {
    let mut parser = vcd::Parser::new(r);
    let header = parser.parse_header()?;
    vcd_header_show(&header);
    let scope_item = header.items.first().ok_or(anyhow!("no root scope!"))?;
    match scope_item {
        Scope(scope) => {
            info!("scope: {} {} children:", scope.identifier, scope.scope_type);
            scope.children.iter().for_each(|item| {
                match item {
                    Scope(_scope) => {}
                    Var(var) => {
                        info!("var {}", var.reference);
                    }
                }
            });
        }
        Var(_var) => {}
    }
    vcd_tree_show(&header);
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