#![allow(unused)]

use anyhow::Result;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rvcd::wave::vcd_parser::Vcd;
use rvcd::wave::WaveLoader;
use std::fs::File;
use tracing::{info, warn};

fn optimize_vcd_parser(path: &str) -> Result<()> {
    info!("optimize_vcd_parser({})", path);
    if let Ok(mut input) = File::open(path) {
        Vcd::load(&mut input)?;
    } else {
        warn!("file not found: {}", path);
    }
    Ok(())
}

pub fn criterion_benchmark(c: &mut Criterion) {
    let files = [
        // "data/testbench.vcd",
        "data/cpu_ila_commit.vcd",
    ];
    for file in files {
        let id = format!("load {file}");
        println!("id: {id}");
        c.bench_function(id.as_ref(), |b| {
            b.iter(|| black_box(optimize_vcd_parser(file)))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
