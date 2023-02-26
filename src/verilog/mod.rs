#![allow(warnings)]
mod veriloglexer;
mod verilogparser;
mod verilogparserlistener;
mod verilogparservisitor;

use antlr_rust::common_token_stream::CommonTokenStream;
use antlr_rust::interval_set::Interval;
use antlr_rust::rule_context::CustomRuleContext;
use antlr_rust::token_factory::CommonTokenFactory;
use antlr_rust::token_stream::TokenStream;
use antlr_rust::tree::{ParseTree, ParseTreeListener, ParseTreeVisitorCompat, Tree};
use antlr_rust::{BaseParser, DefaultErrorStrategy, InputStream};
use queues::IsQueue;
use std::io::Read;
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

#[derive(Default)]
pub struct VerilogSimpleVisitor {
    temp: usize,
}

impl<'i> ParseTreeVisitorCompat<'i> for VerilogSimpleVisitor {
    type Node = VerilogParserContextType;
    type Return = usize;

    fn temp_result(&mut self) -> &mut Self::Return {
        &mut self.temp
    }
}

impl<'i> VerilogParserVisitorCompat<'i> for VerilogSimpleVisitor {}

#[derive(Default, Debug, Clone)]
pub struct VerilogSource {
    pub modules: Vec<VerilogModule>,
    pub source_path: String,
    pub source_code: debug_ignore::DebugIgnore<String>,
}

type VerilogSourceSearchResultType = Vec<Vec<String>>;
impl VerilogSource {
    pub fn search_path(&self, query: &Vec<String>) -> VerilogSourceSearchResultType {
        info!("search_path(query={:?})", query);
        type R = VerilogSourceSearchResultType;
        let mut result: R = vec![];
        let mut query = query.iter().map(|x| x.clone()).collect::<Vec<_>>();
        while result.is_empty() && !query.is_empty() {
            let mut queue = vec![];
            let mut do_if_insert = |queue: &mut Vec<&str>, result: &mut R| {
                if queue.len() >= query.len() && queue[queue.len() - query.len()..] == query {
                    result.push((queue.iter().map(|x| x.to_string()).collect()));
                }
                queue.pop().unwrap();
            };
            fn do_push_insert_pop<'t1: 't2, 't2, F>(
                s: VerilogNameInterval<'t1>,
                queue: &'t2 mut std::vec::Vec<&'t1 str>,
                result: &mut R,
                do_if_insert: F,
            ) where
                F: Fn(&'t2 mut Vec<&str>, &mut R),
            {
                queue.push(s.name);
                do_if_insert(queue, result);
            }
            for module in &self.modules {
                queue.push(module.name.as_str());
                for r in &module.ports {
                    do_push_insert_pop(
                        r.get_name_interval(),
                        &mut queue,
                        &mut result,
                        do_if_insert,
                    );
                }
                for r in &module.regs {
                    do_push_insert_pop(
                        r.get_name_interval(),
                        &mut queue,
                        &mut result,
                        do_if_insert,
                    );
                }
                for r in &module.wires {
                    do_push_insert_pop(
                        r.get_name_interval(),
                        &mut queue,
                        &mut result,
                        do_if_insert,
                    );
                }
                queue.pop().unwrap();
            }
            // pop top to find more paths
            query.remove(0);
        }
        result
    }

    pub fn offset_to_line_no(&self, offset: u64) -> (u64, u64) {
        // TODO: optimize source code
        let line_length = self
            .source_code
            .lines()
            .map(|x| x.len())
            .collect::<Vec<_>>();
        let mut line = 0u64;
        let mut offset_now = 0u64;
        while line < line_length.len() as u64 {
            offset_now += line_length[line as usize] as u64 + 1;
            line += 1;
            if line < line_length.len() as u64 {
                if offset_now + line_length[line as usize] as u64 + 1 > offset {
                    break;
                }
            }
        }
        (line, offset - offset_now)
    }

    // pub fn get_code_from_interval(&self, interval: &CodeInterval) -> u64 {
    //     let data = &self.source_code.0;
    //     let tf = CommonTokenFactory::default();
    //     let lexer = VerilogLexer::new_with_token_factory(InputStream::new(data.as_str()), &tf);
    //     let token_source = CommonTokenStream::new(lexer);
    //     token_source.get_text_from_interval()
    // }
}

#[derive(Debug, Clone)]
pub struct CodeInterval {
    pub a: isize,
    pub b: isize,
}
impl Default for CodeInterval {
    fn default() -> Self {
        Self { a: -1, b: -2 }
    }
}
impl From<Interval> for CodeInterval {
    fn from(value: Interval) -> Self {
        Self {
            a: value.a,
            b: value.b,
        }
    }
}
#[derive(Default, Debug, Clone)]
pub struct VerilogModule {
    pub name: String,
    pub interval: CodeInterval,
    pub ports: Vec<VerilogPort>,
    pub regs: Vec<VerilogReg>,
    pub wires: Vec<VerilogWire>,
}
#[derive(Default, Debug, Clone)]
pub enum VerilogPortType {
    #[default]
    Input,
    Output,
    Inout,
}
#[derive(Default, Debug, Clone)]
pub struct VerilogPort {
    pub typ: VerilogPortType,
    pub name: String,
    pub interval: CodeInterval,
}
#[derive(Default, Debug, Clone)]
pub struct VerilogReg {
    pub name: String,
    pub interval: CodeInterval,
}
#[derive(Default, Debug, Clone)]
pub struct VerilogWire {
    pub name: String,
    pub interval: CodeInterval,
}
#[derive(Default, Debug, Clone)]
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

pub struct VerilogNameInterval<'i> {
    pub name: &'i str,
    pub interval: &'i CodeInterval,
}
trait HaveNameInterval {
    fn get_name(&self) -> &str;
    fn get_interval(&self) -> &CodeInterval;
    fn get_name_interval<'i>(&'i self) -> VerilogNameInterval<'i> {
        VerilogNameInterval {
            name: self.get_name(),
            interval: self.get_interval(),
        }
    }
}
impl HaveNameInterval for VerilogModule {
    fn get_name(&self) -> &str {
        self.name.as_str()
    }
    fn get_interval(&self) -> &CodeInterval {
        &self.interval
    }
}
impl HaveNameInterval for VerilogPort {
    fn get_name(&self) -> &str {
        self.name.as_str()
    }
    fn get_interval(&self) -> &CodeInterval {
        &self.interval
    }
}
impl HaveNameInterval for VerilogReg {
    fn get_name(&self) -> &str {
        self.name.as_str()
    }
    fn get_interval(&self) -> &CodeInterval {
        &self.interval
    }
}
impl HaveNameInterval for VerilogWire {
    fn get_name(&self) -> &str {
        self.name.as_str()
    }
    fn get_interval(&self) -> &CodeInterval {
        &self.interval
    }
}

impl<'i> VerilogParserListener<'i> for MyVerilogListener {
    fn exit_module_identifier(&mut self, ctx: &Module_identifierContext<'i>) {
        info!("module identifier: {}", ctx.get_text());
        let interval = ctx.get_source_interval();
        if let Some(s) = self.module.as_mut() {
            s.name = ctx.get_text();
            s.interval = ctx.get_source_interval().into();
        }
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
        if let Some(s) = self.port.as_mut() {
            s.name = ctx.get_text();
            s.interval = ctx.get_source_interval().into();
        }
    }

    fn exit_net_identifier(&mut self, ctx: &Net_identifierContext<'i>) {
        if let Some(s) = self.wire.as_mut() {
            s.name = ctx.get_text();
            s.interval = ctx.get_source_interval().into();
        }
    }

    fn exit_net_declaration(&mut self, _ctx: &Net_declarationContext<'i>) {
        self.module
            .as_mut()
            .unwrap()
            .wires
            .push(self.wire.replace(Default::default()).unwrap());
    }

    fn exit_variable_identifier(&mut self, ctx: &Variable_identifierContext<'i>) {
        if let Some(s) = self.reg.as_mut() {
            s.name = ctx.get_text();
            s.interval = ctx.get_source_interval().into();
        }
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
        tracing::trace!(
            "rule entered {}",
            verilogparser::ruleNames
                .get(ctx.get_rule_index())
                .unwrap_or(&"error")
        );
    }

    fn exit_every_rule(&mut self, ctx: &dyn VerilogParserContext<'i>) {
        tracing::trace!(
            "rule exit {}",
            verilogparser::ruleNames
                .get(ctx.get_rule_index())
                .unwrap_or(&"error")
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_verilog_file(path: &str) -> anyhow::Result<VerilogSource> {
    let mut file = std::fs::File::open(path)?;
    let mut data = "".to_string();
    file.read_to_string(&mut data)?;
    let tf = CommonTokenFactory::default();
    let lexer = VerilogLexer::new_with_token_factory(InputStream::new(data.as_str()), &tf);
    let token_source = CommonTokenStream::new(lexer);
    let mut parser = VerilogParser::new(token_source);
    let listener = MyVerilogListener::new();
    let listener_id = parser.add_parse_listener(Box::new(listener));
    let result = parser.source_text().expect("parsed unsuccessfully");
    let mut visitor = VerilogSimpleVisitor::default();
    let _visitor_result = visitor.visit(&*result);
    let listener = parser.remove_parse_listener(listener_id);
    let mut parsed = listener.source;
    parsed.source_path = path.to_string();
    parsed.source_code = debug_ignore::DebugIgnore(data.clone());
    Ok(parsed)
}

#[cfg(test)]
mod test {
    use crate::verilog::{
        parse_verilog_file, MyVerilogListener, VerilogLexer, VerilogModulesVisitor, VerilogParser,
        VerilogSimpleVisitor,
    };
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
        let mut visitor = VerilogSimpleVisitor::default();
        let _visitor_result = visitor.visit(&*result);
        // info!("modules: {:?}", visitor_result);
        let listener = parser.remove_parse_listener(listener_id);
        info!("tree: {:?}", listener);
    }

    #[test]
    fn test_parse_verilog_file() {
        let r = parse_verilog_file("data/code-sample/waterfall.v").unwrap();
        println!("data: {:?}", r);
    }
}
