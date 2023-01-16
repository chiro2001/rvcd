use std::io::Read;
use anyhow::Result;
use log::info;
use vcd::Header;

pub fn vcd_header_show(header: &Header) {
    // info!("{header:#?}");
    info!("comment: {:#?}", header.comment);
    info!("date: {:#?}", header.date);
    info!("version: {:#?}", header.version);
    info!("timescale: {:#?}", header.timescale);
    // header.items.
    info!("done");
}

pub fn vcd_read(r: &mut dyn Read) -> Result<()> {
    let mut parser = vcd::Parser::new(r);
    let header = parser.parse_header()?;
    vcd_header_show(&header);
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