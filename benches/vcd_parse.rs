#![allow(unused)]

use anyhow::Result;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rvcd::wave::vcd_parser::Vcd;
use rvcd::wave::WaveLoader;
use std::fs::File;
use tracing::warn;

fn main() {
    fn optimize_vcd_parser(path: &str) -> Result<()> {
        tracing_subscriber::fmt::init();
        if let Ok(mut input) = File::open(path) {
            Vcd::load(&mut input)?;
        } else {
            warn!("file not found: {}", path);
        }
        Ok(())
    }

    fn criterion_benchmark(c: &mut Criterion) {
        let files = ["data/testbench.vcd"];
        for file in files {
            c.bench_function(format!("load {}", file).as_str(), |b| {
                b.iter(|| black_box(optimize_vcd_parser(file)))
            });
        }
    }

    criterion_group!(benches, criterion_benchmark);
    criterion_main!(benches);
}
