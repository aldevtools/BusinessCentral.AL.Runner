//! The `.alcpuprofile` JSON of a profile, in the shape BC and the AL extension write it (V8 CPU profile contract):
//! `nodes` with `callFrame`, `hitCount`, `children`, `positionTicks`, `declaringApplication` and
//! `applicationDefinition`; `startTime`/`endTime` in µs since 0001-01-01; `samples` plus `timeDeltas` (sampling,
//! `kind` 1: self time per node) or `sampleExecutionTimes` (instrumentation, no `kind`, as the AL extension writes
//! it: total time per node). Lines and columns are 0-based. Unknown top-level fields are ignored by VS Code and by
//! BC's Performance Profiler page, so the runner adds an `alRunner` block (unit, threads, codeunits and tests).
//!
//! Record operations become synthetic child nodes, laid out like BC's `SQL` nodes: under the AL node, one
//! `Database (in memory)` node, and under it one node per operation and table (`FindSet "Sales Line"`), with the
//! count as `hitCount` and the runner's in-memory time. They carry no SQL text: the runner runs none.

use super::{CodeunitProfile, Collected, Node, Thread, UNIX_EPOCH_FROM_YEAR_1_US};
use serde_json::{Value, json};

/// The label of the synthetic node that holds a routine's record operations.
pub const DATABASE_NODE: &str = "Database (in memory)";

/// Finds the source link of a node: `file:///...` for code with a file on disk, `None` for the `al-preview` link.
pub type UrlOf<'a> = &'a dyn Fn(&Node) -> Option<String>;

/// The file of one test codeunit.
pub fn codeunit_file(p: &CodeunitProfile, options: &super::Options, url_of: UrlOf) -> Value {
    let body = emit(&p.nodes, p.sampling, url_of);
    let wall_us = p.wall_ns / 1000;
    let mut v = frame(body, p.sampling, p.start_unix_us, wall_us);
    v["activityDescription"] = json!(format!("al-runner: codeunit {} \"{}\"", p.codeunit, p.name));
    v["alRunner"] = runner_block(options, std::slice::from_ref(p));
    v
}

/// One file for the whole run: root `al-runner run`, a node per thread, a node per test codeunit, then its frames.
/// Totals per thread are that thread's busy time, so the threads together exceed the wall time.
pub fn merged_file(c: &Collected, url_of: UrlOf) -> Value {
    let sampling = c.options.mode == super::Mode::Sampling;
    let nodes = merged_nodes(&c.codeunits);
    let body = emit(&nodes, sampling, url_of);
    let start = c.codeunits.iter().map(|p| p.start_unix_us).min().unwrap_or(0);
    let end = c
        .codeunits
        .iter()
        .map(|p| p.start_unix_us + p.wall_ns / 1000)
        .max()
        .unwrap_or(start);
    let mut v = frame(body, sampling, start, end - start);
    v["activityDescription"] = json!(format!(
        "al-runner: {} test codeunits on {} threads; thread totals are busy time and together exceed the wall time",
        c.codeunits.len(),
        threads(&c.codeunits).len()
    ));
    v["alRunner"] = runner_block(&c.options, &c.codeunits);
    v
}

struct Body {
    nodes: Vec<Value>,
    samples: Vec<u64>,
    deltas: Vec<u64>,
    totals: Vec<u64>,
}

fn frame(b: Body, sampling: bool, start_unix_us: u64, wall_us: u64) -> Value {
    let start = UNIX_EPOCH_FROM_YEAR_1_US + start_unix_us;
    let mut v = json!({
        "nodes": b.nodes,
        "startTime": start,
        "endTime": start + wall_us,
        "samples": b.samples,
        "timeDeltas": b.deltas,
    });
    if sampling {
        v["kind"] = json!(1);
    } else {
        v["sampleExecutionTimes"] = json!(b.totals);
    }
    v
}

fn app_id_n(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_lowercase()
}

/// A 32-bit FNV-1a hash of a node's call path, as BC's `frameIdentifier` (an int).
fn frame_id(parent: i32, parts: &[&str]) -> i32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in parent
        .to_le_bytes()
        .into_iter()
        .chain(parts.iter().flat_map(|p| p.bytes().chain([0])))
    {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h as i32
}

fn preview_url(n: &Node) -> String {
    if n.object_id == 0 {
        return String::new();
    }
    format!(
        "al-preview://allang/{k}/{id}/{k}_{id}.dal",
        k = n.keyword,
        id = n.object_id
    )
}

#[allow(clippy::too_many_arguments)]
fn node_json(
    id: usize,
    function: &str,
    n: &Node,
    url: &str,
    hit_count: u64,
    children: Vec<usize>,
    frame_identifier: i32,
    synthetic: bool,
) -> Value {
    let script = if n.object_id == 0 {
        String::new()
    } else {
        format!("{}_{}", n.object_type, n.object_id)
    };
    let mut v = json!({
        "id": id,
        "callFrame": {
            "functionName": function,
            "scriptId": script,
            "url": url,
            "lineNumber": n.line,
            "columnNumber": n.column,
        },
        "hitCount": hit_count,
        "children": children,
        "declaringApplication": {
            "appName": n.app.name,
            "appPublisher": n.app.publisher,
            "appVersion": n.app.version,
            "appId": app_id_n(&n.app.id),
        },
        "applicationDefinition": {
            "objectType": if synthetic { "System" } else { n.object_type },
            "objectName": if synthetic { DATABASE_NODE } else { n.object_name.as_str() },
            "objectId": if synthetic { 0 } else { n.object_id },
        },
        "frameIdentifier": frame_identifier,
        "isIncompleteMeasurement": false,
        "isBuiltinCodeUnitCall": false,
    });
    if synthetic {
        v["isSynthetic"] = json!(true);
    }
    v
}

/// The label of one record-operation node: `FindSet "Sales Line"`, `Insert "Buffer" (temporary)`.
pub fn db_label(d: &al_vm::DbStat) -> String {
    let temp = if d.temporary { " (temporary)" } else { "" };
    format!("{} \"{}\"{temp}", d.op, d.table)
}

fn emit(nodes: &[Node], sampling: bool, url_of: UrlOf) -> Body {
    let mut b = Body {
        nodes: Vec::new(),
        samples: Vec::new(),
        deltas: Vec::new(),
        totals: Vec::new(),
    };
    if !nodes.is_empty() {
        emit_node(nodes, 0, 0, sampling, url_of, &mut b);
    }
    if sampling {
        // BC lists each sampled node once, sorted (its Performance Profiler page needs them distinct).
        let mut pairs: Vec<(u64, u64)> = b
            .samples
            .iter()
            .copied()
            .zip(b.deltas.iter().copied())
            .filter(|&(_, d)| d > 0)
            .collect();
        pairs.sort_unstable();
        b.samples = pairs.iter().map(|p| p.0).collect();
        b.deltas = pairs.iter().map(|p| p.1).collect();
    }
    b
}

/// Emits node `i` and its subtree; returns its file id (1-based, in depth-first order as BC numbers them).
fn emit_node(nodes: &[Node], i: usize, parent_fid: i32, sampling: bool, url_of: UrlOf, b: &mut Body) -> usize {
    let n = &nodes[i];
    let id = b.nodes.len() + 1;
    b.nodes.push(Value::Null);
    b.samples.push(id as u64);
    b.deltas.push(0);
    b.totals.push(0);
    let slot = id - 1;
    let fid = frame_id(parent_fid, &[n.object_type, &n.object_id.to_string(), &n.function]);
    let mut children: Vec<usize> = n
        .children
        .iter()
        .map(|&c| emit_node(nodes, c, fid, sampling, url_of, b))
        .collect();
    if !n.db.is_empty() {
        children.push(emit_db(n, fid, sampling, b));
    }
    let url = url_of(n).unwrap_or_else(|| preview_url(n));
    let hit = if sampling { n.samples } else { n.calls };
    let mut v = node_json(id, &n.function, n, &url, hit, children, fid, false);
    if !sampling && !n.statements.is_empty() {
        v["positionTicks"] = Value::Array(
            n.statements
                .iter()
                .map(|s| json!({"line": s.line, "column": s.column, "ticks": s.hits, "executionTime": s.ns / 1000}))
                .collect(),
        );
    }
    b.nodes[slot] = v;
    b.totals[slot] = n.total_ns / 1000;
    b.deltas[slot] = super::self_ns(n) / 1000;
    id
}

fn emit_db(n: &Node, parent_fid: i32, sampling: bool, b: &mut Body) -> usize {
    let id = b.nodes.len() + 1;
    let slot = id - 1;
    b.nodes.push(Value::Null);
    b.samples.push(id as u64);
    b.deltas.push(0);
    b.totals.push(0);
    let fid = frame_id(parent_fid, &[DATABASE_NODE]);
    let mut children = Vec::new();
    let mut total = 0;
    for d in &n.db {
        let cid = b.nodes.len() + 1;
        let label = db_label(d);
        let cfid = frame_id(fid, &[&label]);
        b.nodes
            .push(node_json(cid, &label, n, "", d.count, Vec::new(), cfid, true));
        b.samples.push(cid as u64);
        b.deltas.push(d.ns / 1000);
        b.totals.push(d.ns / 1000);
        total += d.ns / 1000;
        children.push(cid);
    }
    let hit = if sampling {
        0
    } else {
        n.db.iter().map(|d| d.count).sum()
    };
    b.nodes[slot] = node_json(id, DATABASE_NODE, n, "", hit, children, fid, true);
    b.totals[slot] = total;
    id
}

fn threads(ps: &[CodeunitProfile]) -> Vec<Thread> {
    let mut t: Vec<Thread> = ps.iter().map(|p| p.thread.clone()).collect();
    t.sort_by_key(|t| t.id);
    t.dedup();
    t
}

/// The tree of the merged file, as resolved nodes.
fn merged_nodes(ps: &[CodeunitProfile]) -> Vec<Node> {
    let mut nodes = vec![Node {
        function: "al-runner run".into(),
        object_name: "al-runner run".into(),
        object_type: "System",
        keyword: "Object",
        ..Node::default()
    }];
    nodes[0].app.name = "System".into();
    for t in threads(ps) {
        let ti = nodes.len();
        let mut tn = Node {
            children: Vec::new(),
            total_ns: 0,
            child_ns: 0,
            ..nodes[0].clone()
        };
        tn.function = t.label.clone();
        tn.object_name = t.label.clone();
        tn.parent = Some(0);
        nodes.push(tn);
        nodes[0].children.push(ti);
        for p in ps.iter().filter(|p| p.thread == t) {
            let ci = nodes.len();
            let base = ci + 1;
            let root = &p.nodes[0];
            nodes.push(Node {
                parent: Some(ti),
                function: format!("Codeunit {} \"{}\"", p.codeunit, p.name),
                object_type: "CodeUnit",
                keyword: "Codeunit",
                object_name: p.name.clone(),
                object_id: p.codeunit,
                app: p.app.clone(),
                calls: 1,
                total_ns: root.total_ns,
                child_ns: root.child_ns,
                samples: root.samples,
                sample_ns: root.sample_ns,
                children: root.children.iter().map(|&c| c - 1 + base).collect(),
                ..Node::default()
            });
            nodes[ti].children.push(ci);
            for n in &p.nodes[1..] {
                let mut n = n.clone();
                n.parent = n.parent.map(|q| if q == 0 { ci } else { q - 1 + base });
                n.children = n.children.iter().map(|&c| c - 1 + base).collect();
                nodes.push(n);
            }
            nodes[ti].total_ns += root.total_ns;
            nodes[ti].child_ns += root.total_ns;
        }
        nodes[0].total_ns += nodes[ti].total_ns;
        nodes[0].child_ns += nodes[ti].total_ns;
    }
    nodes
}

/// The runner's own block: the unit of every time, the threads and, per test codeunit, its total and its tests.
fn runner_block(options: &super::Options, ps: &[CodeunitProfile]) -> Value {
    let mut busy: std::collections::BTreeMap<usize, u64> = Default::default();
    for p in ps {
        *busy.entry(p.thread.id).or_default() += p.wall_ns / 1000;
    }
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "mode": options.mode.name(),
        "unit": "microseconds",
        "intervalUs": if options.mode == super::Mode::Sampling { json!(options.interval_us) } else { Value::Null },
        "samples": ps.iter().map(|p| p.samples).sum::<u64>(),
        "threads": threads(ps).iter().map(|t| json!({"id": t.id, "label": t.label, "busyUs": busy.get(&t.id)})).collect::<Vec<_>>(),
        "codeunits": ps.iter().map(|p| json!({
            "id": p.codeunit,
            "name": p.name,
            "thread": p.thread.id,
            "totalUs": p.wall_ns / 1000,
            "tests": p.tests.iter().map(|(n, d)| json!({"name": n, "totalUs": d.as_micros() as u64})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "databaseNote": "Database (in memory) nodes count the record operations that are SQL calls on BC; their time here is in-memory and far below BC's",
    })
}
