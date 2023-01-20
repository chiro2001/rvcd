use crate::wave::WaveDataValue::Raw;
use crate::wave::WaveTreeNode::WaveRoot;
use crate::wave::{Wave, WaveDataItem, WaveLoader, WaveTimescaleUnit, WaveTreeNode, WireValue};
use anyhow::{anyhow, Result};
use log::info;
use queues::{IsQueue, Queue};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::io::Read;
use std::slice::Iter;
use trees::{Node, Tree};
use vcd::{Command, Header, IdCode, Scope, ScopeItem, TimescaleUnit, Value, Var};

pub fn vcd_header_show(header: &Header) {
    if let Some(c) = header.comment.as_ref() {
        info!("comment: {}", c)
    }
    if let Some(c) = header.date.as_ref() {
        info!("date: {}", c)
    }
    if let Some(c) = header.version.as_ref() {
        info!("version: {}", c)
    }
    if let Some(c) = header.timescale.as_ref() {
        info!("timescale: {} / {}", c.0, c.1)
    }
}

pub fn vcd_tree_show(header: &Header) {
    fn show(item: &ScopeItem, level: usize) {
        match item {
            ScopeItem::Scope(scope) => {
                println!(
                    "{}{}",
                    (0..level).map(|_| "\t").collect::<Vec<&str>>().join(""),
                    scope.identifier
                );
                scope.children.iter().for_each(|i| show(i, level + 1));
            }
            ScopeItem::Var(var) => {
                println!(
                    "{}{} width={}",
                    (0..level).map(|_| "\t").collect::<Vec<&str>>().join(""),
                    var.reference,
                    var.size
                );
            }
            _ => {}
        }
    }
    header.items.iter().for_each(|item| show(item, 0));
}

fn vcd_iterate_path(
    result: &mut HashMap<IdCode, Vec<String>>,
    path: &mut Queue<String>,
    items: &[ScopeItem],
    on_scope: fn(
        result: &mut HashMap<IdCode, Vec<String>>,
        &mut Queue<String>,
        &Scope,
    ) -> Result<()>,
    on_var: fn(result: &mut HashMap<IdCode, Vec<String>>, &mut Queue<String>, &Var) -> Result<()>,
) -> Result<()> {
    for item in items.iter() {
        match item {
            ScopeItem::Scope(scope) => on_scope(result, path, scope)?,
            ScopeItem::Var(var) => on_var(result, path, var)?,
            _ => {}
        }
    }
    Ok(())
}

pub fn vcd_code_name(header: &Header) -> HashMap<IdCode, String> {
    fn add_to_map(m: &mut HashMap<IdCode, String>, it: Iter<'_, ScopeItem>) {
        it.for_each(|c| {
            iterate(c).iter().for_each(|(k, v)| {
                m.insert(*k, v.to_string());
            })
        });
    }
    fn iterate(item: &ScopeItem) -> HashMap<IdCode, String> {
        match item {
            ScopeItem::Scope(scope) => {
                let mut m: HashMap<IdCode, String> = HashMap::new();
                add_to_map(&mut m, scope.children.iter());
                m
            }
            ScopeItem::Var(var) => HashMap::from([(var.code, var.reference.to_string())]),
            _ => HashMap::new(),
        }
    }
    let mut map = HashMap::new();
    add_to_map(&mut map, header.items.iter());
    map
}

fn vcd_iterate_tree(
    mut tree: Box<Tree<WaveTreeNode>>,
    items: &[ScopeItem],
    on_scope: fn(Box<Tree<WaveTreeNode>>, &Scope) -> Tree<WaveTreeNode>,
    on_var: fn(Box<Tree<WaveTreeNode>>, &Var) -> Tree<WaveTreeNode>,
) -> Tree<WaveTreeNode> {
    for item in items.iter() {
        match item {
            ScopeItem::Scope(scope) => {
                let mut node = Box::new(Tree::new(WaveTreeNode::WaveScope(
                    scope.identifier.to_string(),
                )));
                tree.push_back(on_scope(node, scope));
            }
            ScopeItem::Var(var) => {
                let IdCode(id) = var.code;
                let mut node = Box::new(Tree::new(WaveTreeNode::WaveVar(id)));
                tree.push_back(on_var(node, var));
            }
            _ => {}
        }
    }
    tree.deep_clone()
}

pub fn vcd_tree(header: &Header) -> Result<Tree<WaveTreeNode>> {
    let mut root = Box::new(Tree::new(WaveRoot));
    fn on_scope(tree: Box<Tree<WaveTreeNode>>, scope: &Scope) -> Tree<WaveTreeNode> {
        vcd_iterate_tree(tree, scope.children.as_slice(), on_scope, on_var)
    }
    fn on_var(tree: Box<Tree<WaveTreeNode>>, var: &Var) -> Tree<WaveTreeNode> {
        tree.deep_clone()
    }
    Ok(vcd_iterate_tree(
        root,
        header.items.as_slice(),
        on_scope,
        on_var,
    ))
}

pub fn vcd_code_path(header: &Header) -> Result<HashMap<IdCode, Vec<String>>> {
    let mut map: HashMap<IdCode, Vec<String>> = HashMap::new();
    /*
        1 <-- header.items
       / \
      2   3 <-- scope.children
     / | | \
    4  5 6  7
    */
    let mut path: Queue<String> = Queue::new();
    fn on_var(
        result: &mut HashMap<IdCode, Vec<String>>,
        path: &mut Queue<String>,
        var: &Var,
    ) -> Result<()> {
        let mut vec = vec![];
        while path.size() > 0 {
            let v = path.remove().map_err(|e| anyhow!("cannot pop: {}", e))?;
            vec.push(v);
        }
        for v in vec.iter() {
            path.add(v.to_string())
                .map_err(|e| anyhow!("cannot push: {}", e))?;
        }
        result.insert(var.code, vec);
        Ok(())
    }
    fn on_scope(
        result: &mut HashMap<IdCode, Vec<String>>,
        path: &mut Queue<String>,
        scope: &Scope,
    ) -> Result<()> {
        path.add(scope.identifier.to_string())
            .map_err(|e| anyhow!("cannot add path queue: {}", e))?;
        vcd_iterate_path(result, path, scope.children.as_slice(), on_scope, on_var)
    }
    vcd_iterate_path(
        &mut map,
        &mut path,
        header.items.as_slice(),
        on_scope,
        on_var,
    )?;
    Ok(map)
}

impl From<Value> for WireValue {
    fn from(value: Value) -> Self {
        match value {
            Value::V0 => WireValue::V0,
            Value::V1 => WireValue::V1,
            Value::X => WireValue::X,
            Value::Z => WireValue::Z,
        }
    }
}

impl From<TimescaleUnit> for WaveTimescaleUnit {
    fn from(value: TimescaleUnit) -> Self {
        match value {
            TimescaleUnit::S => WaveTimescaleUnit::S,
            TimescaleUnit::MS => WaveTimescaleUnit::MS,
            TimescaleUnit::US => WaveTimescaleUnit::US,
            TimescaleUnit::NS => WaveTimescaleUnit::NS,
            TimescaleUnit::PS => WaveTimescaleUnit::PS,
            TimescaleUnit::FS => WaveTimescaleUnit::FS,
        }
    }
}

pub struct Vcd;
impl WaveLoader for Vcd {
    fn load(reader: &mut dyn Read) -> Result<Wave> {
        let mut parser = vcd::Parser::new(reader);
        let header = parser.parse_header()?;
        let code_names = vcd_code_name(&header)
            .into_iter()
            .map(|i| {
                let IdCode(id) = i.0;
                (id, i.1)
            })
            .collect();
        let code_paths = vcd_code_path(&header)?
            .into_iter()
            .map(|(i, path)| {
                let IdCode(id) = i;
                (id, path)
            })
            .collect();
        let tree = vcd_tree(&header)?;
        let mut headers: HashMap<String, String> = HashMap::new();
        if let Some(c) = header.comment.as_ref() {
            headers.insert("comment".to_string(), c.to_string());
        }
        if let Some(c) = header.date.as_ref() {
            headers.insert("date".to_string(), c.to_string());
        }
        if let Some(c) = header.version.as_ref() {
            headers.insert("version".to_string(), c.to_string());
        }
        let timescale = if let Some(c) = header.timescale.as_ref() {
            (c.0 as u64, c.1.into())
        } else {
            (1, WaveTimescaleUnit::default())
        };
        let mut data = vec![];
        let mut timestamp = 0u64;
        for command_result in parser {
            let command = command_result?;
            match command {
                Command::Timestamp(t) => timestamp = t,
                Command::ChangeScalar(i, v) => {
                    let IdCode(id) = i;
                    data.push(
                        WaveDataItem {
                            id,
                            value: Raw(vec![v.into()]),
                            timestamp,
                        }
                        .compress()?,
                    );
                }
                Command::ChangeVector(i, v) => {
                    let IdCode(id) = i;
                    data.push(
                        WaveDataItem {
                            id,
                            value: Raw(v.into_iter().map(|x| x.into()).collect()),
                            timestamp,
                        }
                        .compress()?,
                    );
                }
                Command::ChangeReal(_, _) => {}
                Command::ChangeString(_, _) => {}
                _ => {}
            }
        }
        Ok(Wave {
            timescale,
            headers,
            code_names,
            code_paths,
            tree,
            data,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::radix::radix_vector_to_string_n;
    use crate::wave::vcd::{vcd_code_name, vcd_header_show, vcd_tree_show};
    use anyhow::Result;
    use std::fs::File;
    use std::io::Read;
    use vcd::Command::{ChangeScalar, ChangeVector, Timestamp};
    use vcd::IdCode;

    fn init() {
        std::env::set_var("RUST_LOG", "debug");
        tracing_subscriber::fmt::init();
    }

    pub fn vcd_read(r: &mut dyn Read) -> Result<()> {
        let mut parser = vcd::Parser::new(r);
        let header = parser.parse_header()?;
        vcd_header_show(&header);
        vcd_tree_show(&header);
        let mut cache = vec![];
        let code_name = vcd_code_name(&header);
        for command_result in parser {
            let command = command_result?;
            let get_name = |code: &IdCode| match code_name.get(code) {
                Some(v) => v,
                None => "None",
            };
            match &command {
                Timestamp(i) => println!("#{}", i),
                ChangeScalar(i, v) => println!("code={}, value={}, name={}", i, v, get_name(i)),
                ChangeVector(i, v) => println!(
                    "code={}, value={}, name={}",
                    i,
                    radix_vector_to_string_n(&v.iter().map(|x| (*x).into()).collect(), 4),
                    get_name(i)
                ),
                c => println!("unknown: {:#?}", c),
            }
            cache.push(command);
        }
        Ok(())
    }

    #[test]
    fn test_vcd() -> Result<()> {
        init();
        let mut input = File::open("data/cpu_ila_commit.vcd")?;
        vcd_read(&mut input)?;
        Ok(())
    }
}
