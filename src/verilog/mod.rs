#![allow(warnings)]
mod veriloglexer;
mod verilogparser;
mod verilogparserlistener;
mod verilogparservisitor;

use antlr_rust::BaseParser;
use antlr_rust::tree::{ParseTree, ParseTreeListener, ParseTreeVisitorCompat};
use tracing::info;
pub use veriloglexer::*;
pub use verilogparser::*;
pub use verilogparserlistener::*;
pub use verilogparservisitor::*;

pub struct VerilogModulesVisitor(pub Vec<String>);

impl<'i> ParseTreeVisitorCompat<'i> for VerilogModulesVisitor {
    type Node = VerilogParserContextType;
    type Return = Vec<String>;

    fn temp_result(&mut self) -> &mut Self::Return {
        &mut self.0
    }
}

impl<'i> VerilogParserVisitorCompat<'i> for VerilogModulesVisitor {
    fn visit_module_declaration(&mut self, ctx: &Module_declarationContext<'i>) -> Self::Return {
        let name = ctx.get_text();
        info!("visit module {}", name);
        vec![name]
    }
}

pub struct MyVerilogListener {}

impl<'i> ParseTreeListener<'i, VerilogParserContextType> for MyVerilogListener {
    fn enter_every_rule(&mut self, ctx: &dyn VerilogParserContext<'i>) {
        info!(
            "rule entered {}",
            verilogparser::ruleNames
                .get(ctx.get_rule_index())
                .unwrap_or(&"error")
        )
    }
}

impl<'i> verilogparserlistener::VerilogParserListener<'i> for MyVerilogListener {}

#[cfg(test)]
mod test {
    use crate::verilog::{MyVerilogListener, VerilogLexer, VerilogModulesVisitor, VerilogParser};
    use antlr_rust::common_token_stream::CommonTokenStream;
    use antlr_rust::token_factory::CommonTokenFactory;
    use antlr_rust::tree::ParseTreeVisitorCompat;
    use antlr_rust::InputStream;
    use std::io::Read;
    use tracing::info;

    #[test]
    fn parse_modules() {
        tracing_subscriber::fmt::init();
        let mut file = std::fs::File::open("data/code-sample/waterfall.v").unwrap();
        let mut data = "".to_string();
        file.read_to_string(&mut data).unwrap();
        info!("code: {data:?}");
        let tf = CommonTokenFactory::default();
        let lexer = VerilogLexer::new_with_token_factory(InputStream::new(data.as_str()), &tf);
        let token_source = CommonTokenStream::new(lexer);
        let mut parser = VerilogParser::new(token_source);
        parser.add_parse_listener(Box::new(MyVerilogListener {}));
        let result = parser.source_text().expect("parsed unsuccessfully");
        let mut visitor = VerilogModulesVisitor(Vec::new());
        let visitor_result = visitor.visit(&*result);
        info!("modules: {:?}", visitor_result);
    }
}
