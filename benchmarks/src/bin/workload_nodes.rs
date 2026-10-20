use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use taffy_benchmarks::pangui_template::{parse_test_case, LayoutNode};

fn count_nodes(node: &LayoutNode) -> usize {
    1 + node.children.iter().map(count_nodes).sum::<usize>()
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("pangui");
    let mut counts = BTreeMap::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().is_none_or(|extension| extension != "txt") {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| io::Error::other("Invalid template file name"))?;
        let text = fs::read_to_string(&path)?;
        let case = parse_test_case(file_name, &text).map_err(io::Error::other)?;
        let root = case
            .root_node
            .ok_or_else(|| io::Error::other(format!("Missing root in {file_name}")))?;
        if counts.insert(case.name, count_nodes(&root)).is_some() {
            return Err(io::Error::other("Duplicate scenario name").into());
        }
    }
    let mut output = io::stdout().lock();
    serde_json::to_writer_pretty(&mut output, &counts)?;
    writeln!(output)?;
    Ok(())
}
