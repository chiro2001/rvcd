use antlr_rust::InputStream;
use rvcd::verilog::{Module_declarationContext, VerilogLexer, VerilogParser, VerilogParserContextType, VerilogParserVisitorCompat};
use std::io::Read;
use antlr_rust::common_token_stream::CommonTokenStream;
use antlr_rust::token_factory::CommonTokenFactory;
use antlr_rust::tree::{ParseTree, ParseTreeVisitorCompat, TerminalNode, VisitChildren};
use tracing::info;

struct MyVerilogVisitor(Vec<String>);

impl<'i> ParseTreeVisitorCompat<'i> for MyVerilogVisitor {
    type Node = VerilogParserContextType;
    type Return = Vec<String>;

    fn temp_result(&mut self) -> &mut Self::Return {
        &mut self.0
    }
}

impl <'i> VerilogParserVisitorCompat<'i> for MyVerilogVisitor {
    fn visit_module_declaration(&mut self, ctx: &Module_declarationContext<'i>) -> Self::Return {
        vec![ctx.get_text()]
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let mut file = std::fs::File::open("data/code-sample/waterfall_tb.v")?;
    let mut data = "".to_string();
    file.read_to_string(&mut data)?;
    info!("code: {data:?}");
    let tf = CommonTokenFactory::default();
    let lexer = VerilogLexer::new_with_token_factory(InputStream::new(data.as_str()), &tf);
    let token_source = CommonTokenStream::new(lexer);
    let mut parser = VerilogParser::new(token_source);
    let result = parser.source_text().expect("parsed unsuccessfully");
    let mut visitor = MyVerilogVisitor(Vec::new());
    let visitor_result = visitor.visit(&*result);
    println!("modules: {:?}", visitor_result);
    Ok(())
}
