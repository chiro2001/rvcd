use anyhow::Result;
use rvcd::wave::vcd_parser::Vcd;
use rvcd::wave::WaveLoader;
use std::fs::File;
use std::io::{Cursor, Read};
use tracing::{info, warn};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("convertor -- test --");
    let path = "data/testbench.vcd";
    // let path = "data/cpu_ila_commit.vcd";
    if let Ok(mut input) = File::open(path) {
        let mut data = vec![];
        let sz = input.read_to_end(&mut data);
        if let Ok(_sz) = sz {
            let mut reader: Cursor<Vec<_>> = Cursor::new(data);
            Vcd::load(&mut reader)?;

        } else {
            warn!("cannot read file");
        }
    } else {
        warn!("file not found: {}", path);
    }
    Ok(())
}
