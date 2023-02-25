use std::io::Read;
use tracing::info;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let mut file = std::fs::File::open("data/code-sample/waterfall_tb.v")?;
    let mut data = "".to_string();
    file.read_to_string(&mut data)?;
    info!("code: {data:?}");
    // let code: verilog::ast::Code = verilog::parse(data.as_str());
    // info!("parsed code: {code:?}");
    Ok(())
}