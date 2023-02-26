#![allow(warnings)]
mod veriloglexer;
mod verilogparser;
mod verilogparserlistener;
mod verilogparservisitor;

use antlr_rust::rule_context::CustomRuleContext;
use antlr_rust::tree::{ParseTree, ParseTreeListener, ParseTreeVisitorCompat, Tree};
use antlr_rust::BaseParser;
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
    // fn visit_module_declaration(&mut self, ctx: &Module_declarationContext<'i>) -> Self::Return {
    //     let name = ctx.get_text();
    //     info!("visit module {}", name);
    //     let mut v = self.visit_children(ctx);
    //     // vec![name]
    //     v.push(name);
    //     v
    // }

    // fn visit_module_keyword(&mut self, ctx: &Module_keywordContext<'i>) -> Self::Return {
    //     let name = ctx.get_text();
    //     info!("visit module {}", name);
    //     vec![name]
    // }
}

#[derive(Default, Debug)]
pub struct VerilogSource {
    pub modules: Vec<VerilogModule>,
}
#[derive(Default, Debug)]
pub struct VerilogModule {
    pub name: String,
    pub ports: Vec<VerilogPort>,
    pub regs: Vec<VerilogReg>,
    pub wires: Vec<VerilogWire>,
}
#[derive(Default, Debug)]
pub enum VerilogPortType {
    #[default]
    Input,
    Output,
    Inout,
}
#[derive(Default, Debug)]
pub struct VerilogPort {
    pub typ: VerilogPortType,
    pub name: String,
}
#[derive(Default, Debug)]
pub struct VerilogReg {
    pub name: String,
}
#[derive(Default, Debug)]
pub struct VerilogWire {
    pub name: String,
}
#[derive(Default, Debug)]
pub struct MyVerilogListener {
    pub source: VerilogSource,
    pub module: Option<VerilogModule>,
    pub port: Option<VerilogPort>,
    pub reg: Option<VerilogReg>,
    pub wire: Option<VerilogWire>,
}

impl MyVerilogListener {
    pub fn new() -> Self {
        Self {
            module: Some(VerilogModule::default()),
            port: Some(VerilogPort::default()),
            reg: Some(VerilogReg::default()),
            wire: Some(VerilogWire::default()),
            ..Self::default()
        }
    }
}

impl<'i> VerilogParserListener<'i> for MyVerilogListener {
    fn exit_module_identifier(&mut self, ctx: &Module_identifierContext<'i>) {
        info!("module identifier: {}", ctx.get_text());
        self.module.as_mut().unwrap().name = ctx.get_text();
    }

    fn exit_port_declaration(&mut self, _ctx: &Port_declarationContext<'i>) {
        self.module
            .as_mut()
            .unwrap()
            .ports
            .push(self.port.replace(Default::default()).unwrap());
    }

    fn exit_input_port_identifier(&mut self, _ctx: &Input_port_identifierContext<'i>) {
        self.port.as_mut().unwrap().typ = VerilogPortType::Input;
    }

    fn exit_output_port_identifier(&mut self, _ctx: &Output_port_identifierContext<'i>) {
        self.port.as_mut().unwrap().typ = VerilogPortType::Output;
    }

    fn exit_inout_port_identifier(&mut self, _ctx: &Inout_port_identifierContext<'i>) {
        self.port.as_mut().unwrap().typ = VerilogPortType::Inout;
    }

    fn exit_port_identifier(&mut self, ctx: &Port_identifierContext<'i>) {
        self.port.as_mut().unwrap().name = ctx.get_text();
    }

    fn exit_net_identifier(&mut self, ctx: &Net_identifierContext<'i>) {
        self.wire.as_mut().unwrap().name = ctx.get_text();
    }

    fn exit_net_declaration(&mut self, _ctx: &Net_declarationContext<'i>) {
        self.module
            .as_mut()
            .unwrap()
            .wires
            .push(self.wire.replace(Default::default()).unwrap());
    }

    fn exit_variable_identifier(&mut self, ctx: &Variable_identifierContext<'i>) {
        self.reg.as_mut().unwrap().name = ctx.get_text();
    }

    fn exit_reg_declaration(&mut self, _ctx: &Reg_declarationContext<'i>) {
        self.module
            .as_mut()
            .unwrap()
            .regs
            .push(self.reg.replace(Default::default()).unwrap());
    }

    fn exit_module_declaration(&mut self, ctx: &Module_declarationContext<'i>) {
        self.source
            .modules
            .push(self.module.replace(Default::default()).unwrap());
    }
}

impl<'i> ParseTreeListener<'i, VerilogParserContextType> for MyVerilogListener {
    fn enter_every_rule(&mut self, ctx: &dyn VerilogParserContext<'i>) {
        info!(
            "rule entered {}",
            verilogparser::ruleNames
                .get(ctx.get_rule_index())
                .unwrap_or(&"error")
        );
    }

    fn exit_every_rule(&mut self, ctx: &dyn VerilogParserContext<'i>) {
        info!(
            "rule exit {}",
            verilogparser::ruleNames
                .get(ctx.get_rule_index())
                .unwrap_or(&"error")
        );
    }
}

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
        let listener = MyVerilogListener::new();
        let listener_id = parser.add_parse_listener(Box::new(listener));
        let result = parser.source_text().expect("parsed unsuccessfully");
        let mut visitor = VerilogModulesVisitor(Vec::new());
        let visitor_result = visitor.visit(&*result);
        info!("modules: {:?}", visitor_result);
        let listener = parser.remove_parse_listener(listener_id);
        info!("tree: {:?}", listener);
    }
}
