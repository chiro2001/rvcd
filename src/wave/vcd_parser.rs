use crate::wave::WaveDataValue::Raw;
use crate::wave::WaveTreeNode::WaveRoot;
use crate::wave::{
    Wave, WaveDataItem, WaveInfo, WaveLoader, WavePreLoader, WaveScopeInfo, WaveSignalInfo,
    WaveTimescaleUnit, WaveTreeNode, WireValue,
};
use anyhow::{anyhow, Result};
use queues::{IsQueue, Queue};
use regex::Regex;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::{BufReader, Read, Seek};
use std::slice::Iter;
use tracing::info;
use trees::Tree;
use vcd::{Command, Header, IdCode, Scope, ScopeItem, TimescaleUnit, Value, Var};

pub struct Vcd;

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

type OnScopeType =
    fn(result: &mut HashMap<IdCode, Vec<String>>, &mut Queue<String>, &Scope) -> Result<()>;

type OnVarType =
    fn(result: &mut HashMap<IdCode, Vec<String>>, &mut Queue<String>, &Var) -> Result<()>;

fn vcd_iterate_path(
    result: &mut HashMap<IdCode, Vec<String>>,
    path: &mut Queue<String>,
    items: &[ScopeItem],
    on_scope: OnScopeType,
    on_var: OnVarType,
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

pub fn vcd_code_name(header: &Header) -> HashMap<IdCode, WaveSignalInfo> {
    fn add_to_map(m: &mut HashMap<IdCode, WaveSignalInfo>, it: Iter<'_, ScopeItem>) {
        it.for_each(|c| {
            iterate(c).iter().for_each(|(k, s)| {
                m.insert(*k, s.clone());
            })
        });
    }
    fn iterate(item: &ScopeItem) -> HashMap<IdCode, WaveSignalInfo> {
        match item {
            ScopeItem::Scope(scope) => {
                let mut m: HashMap<IdCode, WaveSignalInfo> = HashMap::new();
                add_to_map(&mut m, scope.children.iter());
                m
            }
            ScopeItem::Var(var) => HashMap::from([(var.code, var.into())]),
            _ => HashMap::new(),
        }
    }
    let mut map = HashMap::new();
    add_to_map(&mut map, header.items.iter());
    map
}

fn vcd_iterate_tree(
    mut tree: Tree<WaveTreeNode>,
    items: &[ScopeItem],
    on_scope: fn(Tree<WaveTreeNode>, &Scope, u64) -> Tree<WaveTreeNode>,
    on_var: fn(Tree<WaveTreeNode>, &Var) -> Tree<WaveTreeNode>,
    scope_id: u64,
) -> Tree<WaveTreeNode> {
    let mut vars = vec![];
    for item in items.iter() {
        match item {
            ScopeItem::Scope(scope) => {
                let mut hasher = DefaultHasher::new();
                scope.identifier.hash(&mut hasher);
                scope_id.hash(&mut hasher);
                let id = hasher.finish();
                let node = Tree::new(WaveTreeNode::WaveScope(
                    // bfs cannot specify id, so use hash now. TODO: dfs
                    WaveScopeInfo::from_scope(id, scope),
                ));
                tree.push_back(on_scope(node, scope, id));
            }
            ScopeItem::Var(var) => {
                let node = Tree::new(WaveTreeNode::WaveVar(var.into()));
                vars.push(on_var(node, var));
            }
            _ => {}
        }
    }
    for v in vars {
        tree.push_back(v);
    }
    tree.deep_clone()
}

fn merge_scope_items(items: Vec<ScopeItem>) -> Vec<ScopeItem> {
    let mut results: Vec<ScopeItem> = vec![];
    for item in items {
        match item {
            ScopeItem::Scope(scope) => {
                if let Some(r) = results.iter_mut().find(|x| match &x {
                    ScopeItem::Scope(x) => x.identifier.as_str(),
                    ScopeItem::Var(_) => "",
                    ScopeItem::Comment(_) => "",
                } == scope.identifier.as_str()) {
                    match r {
                        ScopeItem::Scope(s) => {
                            for child in scope.children {
                                s.children.push(child);
                            }
                        }
                        ScopeItem::Var(_) => {}
                        ScopeItem::Comment(_) => {}
                    }
                } else {
                    results.push(ScopeItem::Scope(scope));
                }
            }
            ScopeItem::Var(x) => results.push(ScopeItem::Var(x)),
            ScopeItem::Comment(_) => {}
        }
    }
    results
}

pub fn vcd_tree(header: &Header) -> Result<Tree<WaveTreeNode>> {
    let root = Tree::new(WaveRoot);
    fn on_scope(tree: Tree<WaveTreeNode>, scope: &Scope, scope_id: u64) -> Tree<WaveTreeNode> {
        let children = merge_scope_items(scope.children.clone());
        vcd_iterate_tree(tree, children.as_slice(), on_scope, on_var, scope_id + 1)
    }
    fn on_var(tree: Tree<WaveTreeNode>, _var: &Var) -> Tree<WaveTreeNode> {
        tree.deep_clone()
    }
    let items = merge_scope_items(header.items.clone());
    let result = vcd_iterate_tree(root, items.as_slice(), on_scope, on_var, 0);
    Ok(result)
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
        vec.push(var.reference.to_string());
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

/// Parse vcd file to get last timestamp
/// ```rust
/// let file = std::fs::File::open("data/cpu_ila_commit.vcd").unwrap();
/// let rev_lines = rev_lines::RevLines::new(std::io::BufReader::new(file)).unwrap();
/// for line in rev_lines {
///     println!("{}", line);
///     if line.starts_with("#") { break; }
/// }
/// ```
///
/// # Example
/// ```rust
/// # fn main() -> anyhow::Result<()> {
/// use rvcd::wave::vcd_parser::vcd_get_last_timestamp;
/// let file = std::fs::File::open("data/cpu_ila_commit.vcd").unwrap();
/// let stamp = vcd_get_last_timestamp(std::io::BufReader::new(file)).0.ok_or(anyhow::Error::msg("cannot get timestamp"))?;
/// println!("got stamp: {}", stamp);
/// Ok(())
/// # }
/// ```
pub fn vcd_get_last_timestamp<T>(
    reader: BufReader<T>,
) -> (Option<u64>, std::io::Result<BufReader<T>>)
where
    T: Read + Seek,
{
    // trying to get last timestamp
    let limit_lines = 1024;
    let mut result = None;
    let re = Regex::new("^#(\\w+)$").unwrap();
    if let Ok(mut lines) = rev_lines::RevLines::new(reader) {
        let mut cnt = 0;
        for line in lines.iter() {
            if let Some(cap) = re.captures(line.as_str()) {
                if let Some(number) = cap.get(1) {
                    result = number.as_str().parse().ok();
                }
            }
            if cnt >= limit_lines {
                break;
            }
            cnt += 1;
        }
        (result, lines.move_reader_out())
    } else {
        (
            result,
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                anyhow::anyhow!("Cannot find last timestamp"),
            )),
        )
    }
}

impl WavePreLoader for Vcd {
    fn last_timestamp<T>(reader: BufReader<T>) -> (Option<u64>, std::io::Result<BufReader<T>>)
    where
        T: Read + Seek,
    {
        vcd_get_last_timestamp(reader)
    }
}

impl WaveLoader for Vcd {
    fn load<F>(
        reader: &mut dyn Read,
        progress_handler: F,
        last_timestamp: Option<u64>,
    ) -> Result<Wave>
    where
        F: Fn(f32, u64),
    {
        info!("start parsing vcd file");
        #[cfg(not(target_arch = "wasm32"))]
        let perf_start = std::time::Instant::now();
        let mut parser = vcd::Parser::new(reader);
        let header = parser.parse_header()?;
        let code_info = vcd_code_name(&header)
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
        let mut data: HashMap<u64, Vec<WaveDataItem>> = HashMap::new();
        let mut timestamp = 0u64;
        let mut time_start = 0xfffffffffffffu64;
        let mut time_stop = 0u64;
        let timestamp_skip = if let Some(last) = last_timestamp {
            last / 1000
        } else {
            0
        };
        let mut timestamp_notified = 0u64;
        for command_result in parser {
            let command = command_result?;
            match command {
                Command::Timestamp(t) => {
                    if time_start > t {
                        time_start = t;
                    }
                    if time_stop < t {
                        time_stop = t;
                    }
                    timestamp = t;
                    if timestamp_skip > 0 && timestamp > time_start {
                        if let Some(last_timestamp) = last_timestamp {
                            if timestamp_notified + timestamp_skip < timestamp {
                                let progress =
                                    (timestamp - time_start) as f32 / last_timestamp as f32;
                                progress_handler(progress, timestamp);
                                timestamp_notified = timestamp;
                            }
                        }
                    }
                }
                Command::ChangeScalar(i, v) => {
                    let IdCode(id) = i;
                    let item = WaveDataItem {
                        value: Raw(vec![v.into()]),
                        timestamp,
                    }
                    .compress()?;
                    if let Some(list) = data.get_mut(&id) {
                        list.push(item);
                    } else {
                        data.insert(id, vec![item]);
                    }
                }
                Command::ChangeVector(i, v) => {
                    let IdCode(id) = i;
                    let item = WaveDataItem {
                        value: Raw(v.into_iter().map(|x| x.into()).collect()),
                        timestamp,
                    }
                    .compress()?;
                    if let Some(list) = data.get_mut(&id) {
                        list.push(item);
                    } else {
                        data.insert(id, vec![item]);
                    }
                }
                Command::ChangeReal(_, _) => {}
                Command::ChangeString(_, _) => {}
                _ => {}
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let perf_stop = std::time::Instant::now();
            info!("parse vcd use time: {:?}", perf_stop - perf_start);
        }
        Ok(Wave {
            info: WaveInfo {
                timescale,
                range: (time_start, time_stop),
                headers,
                code_signal_info: code_info,
                code_paths,
                tree,
            },
            data,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::radix::radix_vector_to_string_n;
    use crate::wave::vcd_parser::{Vcd, vcd_code_name, vcd_header_show, vcd_tree_show};
    use anyhow::Result;
    use std::fs::File;
    use std::io::Read;
    use tracing::{info, warn};
    use vcd::Command::{ChangeScalar, ChangeVector, Timestamp};
    use vcd::IdCode;
    use crate::wave::WaveLoader;

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
                Some(s) => s.name.as_str(),
                None => "None",
            };
            match &command {
                Timestamp(i) => println!("#{i}"),
                ChangeScalar(i, v) => println!("code={}, value={}, name={}", i, v, get_name(i)),
                ChangeVector(i, v) => println!(
                    "code={}, value={}, name={}",
                    i,
                    radix_vector_to_string_n(&v.iter().map(|x| (*x).into()).collect(), 4),
                    get_name(i)
                ),
                c => println!("unknown: {c:#?}"),
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

    fn testing_vcd_parser(path: &str) -> Result<()> {
        info!("optimize_vcd_parser({})", path);
        if let Ok(mut input) = File::open(path) {
            let v = Vcd::load(&mut input, |_, _| {}, None)?;
            info!("code path: {:#?}", v.info.code_paths);
        } else {
            warn!("file not found: {}", path);
        }
        Ok(())
    }

    #[test]
    pub fn test_vcd_parser() {
        tracing_subscriber::fmt::init();
        let files = [
            // "data/testbench.vcd",
            "data/cpu_ila_commit.vcd",
            "/home/chiro/programs/scaleda-sample-project/.sim/Icarus-Run iverilog simulation/tb_waterfall_waveform.vcd"
        ];
        for file in files {
            let id = format!("load {file}");
            info!("id: {id}");
            testing_vcd_parser(file).unwrap();
        }
    }
}
