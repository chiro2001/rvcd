mod veriloglexer;
mod verilogparser;
mod verilogparservisitor;
mod verilogparserlistener;

use antlr_rust::tree::{ParseTree, ParseTreeVisitorCompat};
pub use veriloglexer::*;
pub use verilogparser::*;
pub use verilogparservisitor::*;
pub use verilogparserlistener::*;

pub struct VerilogModulesVisitor(pub Vec<String>);

impl<'i> ParseTreeVisitorCompat<'i> for VerilogModulesVisitor {
    type Node = VerilogParserContextType;
    type Return = Vec<String>;

    fn temp_result(&mut self) -> &mut Self::Return {
        &mut self.0
    }
}

impl <'i> VerilogParserVisitorCompat<'i> for VerilogModulesVisitor {
    fn visit_module_declaration(&mut self, ctx: &Module_declarationContext<'i>) -> Self::Return {
        vec![ctx.get_text()]
    }
}