use antlr_rust::InputStream;
use rvcd::verilog::{VerilogLexer, VerilogParser};
use std::io::Read;
use antlr_rust::common_token_stream::CommonTokenStream;
use antlr_rust::token_factory::CommonTokenFactory;
use tracing::info;

struct MyVerilogVisitor<'i>(Vec<&'i str>);

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let mut file = std::fs::File::open("data/code-sample/waterfall_tb.v")?;
    let mut data = "".to_string();
    file.read_to_string(&mut data)?;
    info!("code: {data:?}");
    let tf = CommonTokenFactory::default();
    let mut lexer = VerilogLexer::new_with_token_factory(InputStream::new(data.as_str()), &tf);
    let token_source = CommonTokenStream::new(lexer);
    let mut parser = VerilogParser::new(token_source);

    // let result = parser..expect("parsed unsuccessfully");
    // let visitor = MyVerilogVisitor(Vec::new());

    Ok(())
}
