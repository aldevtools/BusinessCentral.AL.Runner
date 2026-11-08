use super::alcpuprofile::{DATABASE_NODE, codeunit_file, merged_file};
use super::*;
use serde_json::Value;
use std::collections::HashMap;

fn node(function: &str, parent: Option<usize>, children: Vec<usize>, total_ms: u64, child_ms: u64) -> Node {
    Node {
        parent,
        children,
        function: function.into(),
        object_type: "CodeUnit",
        keyword: "Codeunit",
        object_name: "Calib".into(),
        object_id: 50100,
        app: AppRef {
            id: "6b2a1f3e-0000-4000-8000-0000000000aa".into(),
            name: "Calib".into(),
            publisher: "Me".into(),
            version: "1.0.0.0".into(),
        },
        file: Some("src/Calib.al".into()),
        line: 9,
        column: 8,
        calls: 1,
        total_ns: total_ms * 1_000_000,
        child_ns: child_ms * 1_000_000,
        frame_name: format!("codeunit Calib.{function}"),
        ..Node::default()
    }
}

/// root -> A (10 ms) -> B (8 ms) -> C (5 ms).
fn chain() -> CodeunitProfile {
    let mut root = node("Thread 1 (main)", None, vec![1], 10, 10);
    root.object_type = "System";
    root.object_id = 0;
    let mut a = node("A", Some(0), vec![2], 10, 8);
    a.statements = vec![Statement {
        line: 9,
        column: 8,
        hits: 1,
        ns: 10_000_000,
    }];
    CodeunitProfile {
        codeunit: 50120,
        name: "Calib Tests".into(),
        app: AppRef::default(),
        thread: Thread {
            id: 1,
            label: "Thread 1 (main)".into(),
        },
        sampling: false,
        start_unix_us: 1_700_000_000_000_000,
        wall_ns: 12_000_000,
        samples: 0,
        nodes: vec![
            root,
            a,
            node("B", Some(1), vec![3], 8, 5),
            node("C", Some(2), vec![], 5, 0),
        ],
        tests: vec![("TestA".into(), Duration::from_millis(11))],
    }
}

fn instr() -> Options {
    Options::default()
}

fn sampling_opts() -> Options {
    Options {
        mode: Mode::Sampling,
        ..Options::default()
    }
}

/// The AL extension's model of a file (`SamplingModelBuilder` / `IntrumentationModelBuilder` of AL extension 18.1,
/// re-done here from its observed arithmetic): (self µs, total µs) per node id.
fn viewer_model(v: &Value) -> HashMap<u64, (i64, i64)> {
    let nodes = v["nodes"].as_array().unwrap();
    let samples: Vec<u64> = v["samples"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap())
        .collect();
    let children: HashMap<u64, Vec<u64>> = nodes
        .iter()
        .map(|n| {
            (
                n["id"].as_u64().unwrap(),
                n["children"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| c.as_u64().unwrap())
                    .collect(),
            )
        })
        .collect();
    let kind = v.get("kind").and_then(Value::as_i64).unwrap_or(0);
    let mut own: HashMap<u64, i64> = HashMap::new();
    let mut out = HashMap::new();
    if kind == 1 {
        let deltas = v["timeDeltas"].as_array().unwrap();
        for (i, s) in samples.iter().enumerate() {
            *own.entry(*s).or_default() += deltas[i].as_i64().unwrap();
        }
        fn total(id: u64, own: &HashMap<u64, i64>, ch: &HashMap<u64, Vec<u64>>) -> i64 {
            own.get(&id).copied().unwrap_or(0) + ch[&id].iter().map(|&c| total(c, own, ch)).sum::<i64>()
        }
        for id in children.keys() {
            out.insert(*id, (own.get(id).copied().unwrap_or(0), total(*id, &own, &children)));
        }
    } else {
        let times = v["sampleExecutionTimes"].as_array().unwrap();
        let mut agg: HashMap<u64, i64> = HashMap::new();
        for (i, s) in samples.iter().enumerate() {
            *agg.entry(*s).or_default() += times[i].as_i64().unwrap();
        }
        // computeSelfTime: total minus the children's SELF times (not their totals).
        fn self_of(id: u64, agg: &HashMap<u64, i64>, ch: &HashMap<u64, Vec<u64>>) -> i64 {
            agg.get(&id).copied().unwrap_or(0) - ch[&id].iter().map(|&c| self_of(c, agg, ch)).sum::<i64>()
        }
        for id in children.keys() {
            out.insert(*id, (self_of(*id, &agg, &children), agg.get(id).copied().unwrap_or(0)));
        }
    }
    out
}

/// What VS Code needs from a file (phase 1 report, 1.2 and 1.3).
pub(crate) fn assert_schema(v: &Value) {
    for k in ["startTime", "endTime"] {
        assert!(v[k].is_u64(), "{k}: {}", v[k]);
    }
    assert!(v["endTime"].as_u64() >= v["startTime"].as_u64());
    let nodes = v["nodes"].as_array().expect("nodes");
    assert!(!nodes.is_empty());
    let ids: std::collections::HashSet<u64> = nodes.iter().map(|n| n["id"].as_u64().unwrap()).collect();
    assert_eq!(ids.len(), nodes.len(), "ids are unique");
    let mut parents: HashMap<u64, u64> = HashMap::new();
    for n in nodes {
        let cf = &n["callFrame"];
        assert!(cf["functionName"].is_string() && cf["url"].is_string() && cf["scriptId"].is_string());
        assert!(cf["lineNumber"].is_u64() && cf["columnNumber"].is_u64());
        assert!(n["hitCount"].is_u64());
        for k in ["appName", "appPublisher", "appVersion", "appId"] {
            assert!(n["declaringApplication"][k].is_string(), "{k}");
        }
        assert!(n["applicationDefinition"]["objectType"].is_string());
        assert!(n["applicationDefinition"]["objectName"].is_string());
        assert!(n["applicationDefinition"]["objectId"].is_u64());
        for c in n["children"].as_array().unwrap() {
            let c = c.as_u64().unwrap();
            assert!(ids.contains(&c), "child {c} exists");
            assert!(
                parents.insert(c, n["id"].as_u64().unwrap()).is_none(),
                "child {c} has one parent"
            );
        }
        if let Some(pt) = n.get("positionTicks") {
            for t in pt.as_array().unwrap() {
                for k in ["line", "column", "ticks", "executionTime"] {
                    assert!(t[k].is_u64(), "positionTicks.{k}");
                }
            }
        }
    }
    assert!(
        !parents.contains_key(&nodes[0]["id"].as_u64().unwrap()),
        "nodes[0] is the root"
    );
    let samples = v["samples"].as_array().unwrap();
    assert_eq!(samples.len(), v["timeDeltas"].as_array().unwrap().len());
    match v.get("kind") {
        Some(k) => {
            assert_eq!(k, 1);
            assert!(v.get("sampleExecutionTimes").is_none());
            let s: Vec<u64> = samples.iter().map(|x| x.as_u64().unwrap()).collect();
            let mut sorted = s.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(s, sorted, "sampling samples are distinct and sorted");
        }
        None => assert_eq!(samples.len(), v["sampleExecutionTimes"].as_array().unwrap().len()),
    }
    assert_eq!(v["alRunner"]["unit"], "microseconds");
}

fn by_name<'a>(v: &'a Value, name: &str) -> &'a Value {
    v["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["callFrame"]["functionName"] == name)
        .unwrap_or_else(|| panic!("no node {name}"))
}

#[test]
fn instrumentation_file_is_written_as_bc_writes_it() {
    let v = codeunit_file(&chain(), &instr(), &|_| None);
    assert_schema(&v);
    assert!(
        v.get("kind").is_none(),
        "the AL extension writes no kind for instrumentation"
    );
    let a = by_name(&v, "A");
    assert_eq!(a["hitCount"], 1);
    assert_eq!(a["callFrame"]["lineNumber"], 9);
    assert_eq!(a["callFrame"]["scriptId"], "CodeUnit_50100");
    assert_eq!(
        a["callFrame"]["url"],
        "al-preview://allang/Codeunit/50100/Codeunit_50100.dal"
    );
    assert_eq!(
        a["declaringApplication"]["appId"], "6b2a1f3e0000400080000000000000aa",
        "GUID in N format"
    );
    assert_eq!(a["positionTicks"][0]["executionTime"], 10_000);
    assert_eq!(
        v["startTime"].as_u64().unwrap(),
        UNIX_EPOCH_FROM_YEAR_1_US + 1_700_000_000_000_000
    );
    assert_eq!(
        v["endTime"].as_u64().unwrap() - v["startTime"].as_u64().unwrap(),
        12_000
    );
    assert_eq!(v["alRunner"]["codeunits"][0]["tests"][0]["totalUs"], 11_000);
}

#[test]
fn the_viewer_shows_wrong_self_time_for_instrumentation_files_as_for_bcs() {
    let v = codeunit_file(&chain(), &instr(), &|_| None);
    let m = viewer_model(&v);
    let id = |n: &str| by_name(&v, n)["id"].as_u64().unwrap();
    assert_eq!(m[&id("C")], (5_000, 5_000));
    assert_eq!(m[&id("B")], (3_000, 8_000));
    // True self time of A is 2 ms; the AL extension shows 10 - self(B) = 10 - 3 = 7 ms.
    assert_eq!(m[&id("A")].1, 10_000);
    assert_eq!(
        m[&id("A")].0,
        7_000,
        "computeSelfTime subtracts the children's self times"
    );
}

#[test]
fn sampling_file_gives_the_viewer_exact_self_and_total() {
    let mut p = chain();
    p.sampling = true;
    for (i, ms) in [(1, 2u64), (2, 3), (3, 5)] {
        p.nodes[i].total_ns = 0;
        p.nodes[i].child_ns = 0;
        p.nodes[i].sample_ns = ms * 1_000_000;
        p.nodes[i].samples = ms;
    }
    p.nodes[0].total_ns = 0;
    p.nodes[0].child_ns = 0;
    let v = codeunit_file(&p, &sampling_opts(), &|_| None);
    assert_schema(&v);
    assert_eq!(v["kind"], 1);
    let m = viewer_model(&v);
    let id = |n: &str| by_name(&v, n)["id"].as_u64().unwrap();
    assert_eq!(m[&id("A")], (2_000, 10_000));
    assert_eq!(m[&id("C")], (5_000, 5_000));
    assert_eq!(by_name(&v, "C")["hitCount"], 5);
    assert!(by_name(&v, "A").get("positionTicks").is_none());
    assert_eq!(v["alRunner"]["intervalUs"], 1000);
}

#[test]
fn record_operations_hang_under_a_database_node_like_bcs_sql_nodes() {
    let mut p = chain();
    p.nodes[3].db = vec![
        al_vm::DbStat {
            op: al_builtins::record_op::RecordOp::FindSet,
            table: "Calib Line".into(),
            temporary: false,
            count: 3,
            ns: 30_000,
            samples: 0,
        },
        al_vm::DbStat {
            op: al_builtins::record_op::RecordOp::Insert,
            table: "Buffer".into(),
            temporary: true,
            count: 20,
            ns: 50_000,
            samples: 0,
        },
    ];
    let v = codeunit_file(&p, &instr(), &|_| None);
    assert_schema(&v);
    let c = by_name(&v, "C");
    let db = by_name(&v, DATABASE_NODE);
    assert!(c["children"].as_array().unwrap().contains(&db["id"]));
    assert_eq!(db["isSynthetic"], true);
    assert_eq!(db["applicationDefinition"]["objectType"], "System");
    assert_eq!(db["hitCount"], 23);
    let find = by_name(&v, "FindSet \"Calib Line\"");
    assert_eq!(find["hitCount"], 3);
    assert!(db["children"].as_array().unwrap().contains(&find["id"]));
    assert_eq!(by_name(&v, "Insert \"Buffer\" (temporary)")["hitCount"], 20);
    // The link of a database node is the routine's position.
    assert_eq!(find["callFrame"]["lineNumber"], 9);
}

#[test]
fn a_file_link_wins_over_the_preview_link() {
    let v = codeunit_file(&chain(), &instr(), &|n| {
        n.file.as_ref().map(|f| format!("file:///work/app/{f}"))
    });
    assert_eq!(by_name(&v, "B")["callFrame"]["url"], "file:///work/app/src/Calib.al");
}

#[test]
fn merged_file_nests_threads_then_codeunits() {
    let a = chain();
    let mut b = chain();
    b.codeunit = 50121;
    b.name = "Other Tests".into();
    b.thread = Thread {
        id: 2,
        label: "Thread 2 (worker)".into(),
    };
    let c = Collected {
        options: instr(),
        codeunits: vec![a, b],
    };
    let v = merged_file(&c, &|_| None);
    assert_schema(&v);
    let root = &v["nodes"][0];
    assert_eq!(root["callFrame"]["functionName"], "al-runner run");
    assert_eq!(root["children"].as_array().unwrap().len(), 2);
    let t2 = by_name(&v, "Thread 2 (worker)");
    let cu = by_name(&v, "Codeunit 50121 \"Other Tests\"");
    assert_eq!(t2["children"][0], cu["id"]);
    assert_eq!(cu["applicationDefinition"]["objectType"], "CodeUnit");
    let m = viewer_model(&v);
    assert_eq!(
        m[&root["id"].as_u64().unwrap()].1,
        20_000,
        "two threads of 10 ms busy time"
    );
    assert_eq!(v["alRunner"]["threads"].as_array().unwrap().len(), 2);
}

#[test]
fn summary_names_the_unit_and_the_codeunits() {
    let c = Collected {
        options: sampling_opts(),
        codeunits: vec![chain()],
    };
    let s = summary::render(&c);
    assert!(
        s.starts_with("samples 0 of 1 ms on 1 threads (each thread sampled every tick); times in ms"),
        "{s}"
    );
    assert!(s.contains("codeunit 50120 \"Calib Tests\"  [Thread 1 (main)]"), "{s}");
    assert!(s.contains("TestA"));
    let c = Collected {
        options: instr(),
        codeunits: vec![chain()],
    };
    let s = summary::render(&c);
    assert!(s.starts_with("instrumentation on 1 threads"), "{s}");
    assert!(s.contains("codeunit Calib.A"));
}

#[test]
fn file_names_are_safe() {
    assert_eq!(write::file_name(50120, "Calib Tests"), "50120-Calib Tests.alcpuprofile");
    assert_eq!(write::file_name(1, "A/B: C?"), "1-A_B_ C_.alcpuprofile");
    assert!(write::is_merged(std::path::Path::new("x/all.alcpuprofile")));
    assert!(!write::is_merged(std::path::Path::new("x/out")));
}

#[test]
fn modes_parse() {
    assert_eq!(Mode::parse("Sampling"), Ok(Mode::Sampling));
    assert_eq!(Mode::parse("instrumentation"), Ok(Mode::Instrumentation));
    assert!(Mode::parse("trace").is_err());
}
