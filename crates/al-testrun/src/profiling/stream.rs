//! Writes an `.alcpuprofile` straight to a file, node by node (al-runner issue 6924: a merged profile of a long run
//! was built as one `serde_json::Value` and then one `String`, several times the size of the profile it held, at the
//! end of the run and past `--max-memory`). The document is the one `alcpuprofile.rs` describes; only a node's own
//! JSON is ever in memory at once. The node ids are depth-first, as BC numbers them, so a node's children's ids come
//! from the sizes of the subtrees before them, computed first ([`Tree::sizes`]).

use super::alcpuprofile::{UrlOf, db_label, frame_id, node_json, preview_url, runner_block, threads};
use super::{CodeunitProfile, Collected, Node, Options, UNIX_EPOCH_FROM_YEAR_1_US};
use serde_json::{Value, json};
use std::io::{self, Write};

/// A node of the file: one of the run's own nodes (root, threads, codeunit heads of the merged file) or node `i` of
/// codeunit `c`.
#[derive(Clone, Copy)]
enum At {
    Own(usize),
    Of(usize, usize),
}

/// The tree of one file over the codeunits' nodes, without copying them.
struct Tree<'a> {
    codeunits: &'a [&'a CodeunitProfile],
    own: Vec<Node>,
    own_children: Vec<Vec<At>>,
    /// Ids a subtree takes: the node, its database node and that node's operations, its children's subtrees.
    own_size: Vec<usize>,
    size: Vec<Vec<u32>>,
}

impl<'a> Tree<'a> {
    fn node(&self, at: At) -> &Node {
        match at {
            At::Own(i) => &self.own[i],
            At::Of(c, i) => &self.codeunits[c].nodes[i],
        }
    }

    fn children(&self, at: At) -> Vec<At> {
        match at {
            At::Own(i) => self.own_children[i].clone(),
            At::Of(c, i) => self.codeunits[c].nodes[i]
                .children
                .iter()
                .map(|&k| At::Of(c, k))
                .collect(),
        }
    }

    fn size(&self, at: At) -> usize {
        match at {
            At::Own(i) => self.own_size[i],
            At::Of(c, i) => self.size[c][i] as usize,
        }
    }

    fn own_ids(n: &Node) -> usize {
        1 + if n.db.is_empty() { 0 } else { 1 + n.db.len() }
    }

    /// Computes the subtree sizes, children before parents.
    fn sizes(&mut self) {
        self.size = self.codeunits.iter().map(|p| vec![0; p.nodes.len()]).collect();
        for (c, p) in self.codeunits.iter().enumerate() {
            if !p.nodes.is_empty() {
                self.size_of(At::Of(c, 0));
            }
        }
        self.own_size = vec![0; self.own.len()];
        if !self.own.is_empty() {
            self.size_of(At::Own(0));
        }
    }

    fn size_of(&mut self, at: At) -> usize {
        let mut s = Self::own_ids(self.node(at));
        for k in self.children(at) {
            s += self.size_of(k);
        }
        match at {
            At::Own(i) => self.own_size[i] = s,
            At::Of(c, i) => self.size[c][i] = s as u32,
        }
        s
    }
}

/// Times of one id, in the order of the ids: (self time, total time) in µs.
fn each_time(t: &Tree, at: At, f: &mut dyn FnMut(u64, u64)) {
    let n = t.node(at);
    f(super::self_ns(n) / 1000, n.total_ns / 1000);
    for k in t.children(at) {
        each_time(t, k, f);
    }
    if !n.db.is_empty() {
        f(0, n.db.iter().map(|d| d.ns / 1000).sum());
        for d in &n.db {
            f(d.ns / 1000, d.ns / 1000);
        }
    }
}

struct Out<'w> {
    w: &'w mut dyn Write,
    first: bool,
    /// Asked between nodes: `true` stops the write ([`Stopped`]).
    stop: &'w dyn Fn() -> bool,
    written: usize,
}

/// The write stopped because `stop` said so (the run's memory limit).
#[derive(Debug)]
pub struct Stopped;

fn stopped() -> io::Error {
    io::Error::other(Stopped)
}

impl std::error::Error for Stopped {}

impl std::fmt::Display for Stopped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("stopped at the memory limit")
    }
}

impl Out<'_> {
    fn item(&mut self, v: &Value) -> io::Result<()> {
        if !std::mem::take(&mut self.first) {
            self.w.write_all(b",")?;
        }
        serde_json::to_writer(&mut *self.w, v).map_err(io::Error::other)?;
        self.written += 1;
        if self.written % 4096 == 0 && (self.stop)() {
            return Err(stopped());
        }
        Ok(())
    }

    fn number(&mut self, n: u64) -> io::Result<()> {
        if !std::mem::take(&mut self.first) {
            self.w.write_all(b",")?;
        }
        write!(self.w, "{n}")
    }

    fn open(&mut self, key: &str) -> io::Result<()> {
        write!(self.w, ",\"{key}\":[")?;
        self.first = true;
        Ok(())
    }
}

/// Writes node `at` with id `id` and its subtree.
fn write_node(
    t: &Tree,
    at: At,
    id: usize,
    parent_fid: i32,
    sampling: bool,
    url_of: UrlOf,
    o: &mut Out,
) -> io::Result<()> {
    let n = t.node(at);
    let fid = frame_id(parent_fid, &[n.object_type, &n.object_id.to_string(), &n.function]);
    let kids = t.children(at);
    let mut ids = Vec::with_capacity(kids.len() + 1);
    let mut next = id + 1;
    for &k in &kids {
        ids.push(next);
        next += t.size(k);
    }
    let db_id = next;
    if !n.db.is_empty() {
        ids.push(db_id);
    }
    let url = url_of(n).unwrap_or_else(|| preview_url(n));
    let hit = if sampling { n.samples } else { n.calls };
    let mut v = node_json(id, &n.function, n, &url, hit, ids, fid, false);
    if !sampling && !n.statements.is_empty() {
        v["positionTicks"] = Value::Array(
            n.statements
                .iter()
                .map(|s| json!({"line": s.line, "column": s.column, "ticks": s.hits, "executionTime": s.ns / 1000}))
                .collect(),
        );
    }
    o.item(&v)?;
    drop(v);
    let mut next = id + 1;
    for k in kids {
        write_node(t, k, next, fid, sampling, url_of, o)?;
        next += t.size(k);
    }
    if !n.db.is_empty() {
        write_db(n, db_id, fid, sampling, o)?;
    }
    Ok(())
}

fn write_db(n: &Node, id: usize, parent_fid: i32, sampling: bool, o: &mut Out) -> io::Result<()> {
    let fid = frame_id(parent_fid, &[super::alcpuprofile::DATABASE_NODE]);
    let children: Vec<usize> = (0..n.db.len()).map(|k| id + 1 + k).collect();
    let hit = if sampling {
        0
    } else {
        n.db.iter().map(|d| d.count).sum()
    };
    let db = super::alcpuprofile::DATABASE_NODE;
    o.item(&node_json(id, db, n, "", hit, children, fid, true))?;
    for (k, d) in n.db.iter().enumerate() {
        let label = db_label(d);
        let cfid = frame_id(fid, &[&label]);
        o.item(&node_json(id + 1 + k, &label, n, "", d.count, Vec::new(), cfid, true))?;
    }
    Ok(())
}

/// Writes the whole document of `t` (rooted at `root`).
#[allow(clippy::too_many_arguments)]
fn document(
    w: &mut dyn Write,
    t: &Tree,
    root: Option<At>,
    sampling: bool,
    start_unix_us: u64,
    wall_us: u64,
    activity: &str,
    runner: &Value,
    url_of: UrlOf,
    stop: &dyn Fn() -> bool,
) -> io::Result<()> {
    let mut o = Out {
        w,
        first: true,
        stop,
        written: 0,
    };
    o.w.write_all(b"{\"nodes\":[")?;
    if let Some(r) = root {
        write_node(t, r, 1, 0, sampling, url_of, &mut o)?;
    }
    let start = UNIX_EPOCH_FROM_YEAR_1_US + start_unix_us;
    write!(o.w, "],\"startTime\":{start},\"endTime\":{}", start + wall_us)?;
    // Sampling: BC lists each sampled node once, sorted by id (its Performance Profiler page needs them distinct),
    // and only those with time. Instrumentation: every node, with its total time in `sampleExecutionTimes`.
    let mut err = Ok(());
    let mut list = |o: &mut Out, key: &str, pick: &dyn Fn(usize, u64, u64) -> Option<u64>| -> io::Result<()> {
        o.open(key)?;
        if let Some(r) = root {
            let mut id = 0;
            each_time(t, r, &mut |d, tot| {
                id += 1;
                if let Some(v) = pick(id, d, tot)
                    && err.is_ok()
                {
                    err = o.number(v);
                }
            });
        }
        o.w.write_all(b"]")
    };
    list(&mut o, "samples", &|id, d, _| (!sampling || d > 0).then_some(id as u64))?;
    list(&mut o, "timeDeltas", &|_, d, _| (!sampling || d > 0).then_some(d))?;
    if sampling {
        o.w.write_all(b",\"kind\":1")?;
    } else {
        list(&mut o, "sampleExecutionTimes", &|_, _, tot| Some(tot))?;
    }
    err?;
    write!(o.w, ",\"activityDescription\":")?;
    serde_json::to_writer(&mut *o.w, activity).map_err(io::Error::other)?;
    write!(o.w, ",\"alRunner\":")?;
    serde_json::to_writer(&mut *o.w, runner).map_err(io::Error::other)?;
    o.w.write_all(b"}")?;
    o.w.flush()
}

/// Writes the file of one test codeunit.
pub fn write_codeunit(
    w: &mut dyn Write,
    p: &CodeunitProfile,
    options: &Options,
    url_of: UrlOf,
    stop: &dyn Fn() -> bool,
) -> io::Result<()> {
    let list = [p];
    let mut t = Tree {
        codeunits: &list,
        own: Vec::new(),
        own_children: Vec::new(),
        own_size: Vec::new(),
        size: Vec::new(),
    };
    t.sizes();
    let root = (!p.nodes.is_empty()).then_some(At::Of(0, 0));
    let activity = format!("al-runner: codeunit {} \"{}\"", p.codeunit, p.name);
    let runner = runner_block(options, std::slice::from_ref(p));
    document(
        w,
        &t,
        root,
        p.sampling,
        p.start_unix_us,
        p.wall_ns / 1000,
        &activity,
        &runner,
        url_of,
        stop,
    )
}

/// One file for the whole run: root `al-runner run`, a node per thread, a node per test codeunit, then its frames.
/// Totals per thread are that thread's busy time, so the threads together exceed the wall time.
pub fn write_merged(w: &mut dyn Write, c: &Collected, url_of: UrlOf, stop: &dyn Fn() -> bool) -> io::Result<()> {
    let sampling = c.options.mode == super::Mode::Sampling;
    let threads = threads(&c.codeunits);
    let order: Vec<&CodeunitProfile> = threads
        .iter()
        .flat_map(|t| c.codeunits.iter().filter(move |p| p.thread == *t))
        .collect();
    let mut root = Node {
        function: "al-runner run".into(),
        object_name: "al-runner run".into(),
        object_type: "System",
        keyword: "Object",
        ..Node::default()
    };
    root.app = std::sync::Arc::new(super::AppRef {
        name: "System".into(),
        ..super::AppRef::default()
    });
    let mut own = vec![root];
    let mut own_children: Vec<Vec<At>> = vec![Vec::new()];
    let mut k = 0;
    for t in &threads {
        let ti = own.len();
        let mut tn = Node {
            children: Vec::new(),
            total_ns: 0,
            child_ns: 0,
            ..own[0].clone()
        };
        tn.function = t.label.as_str().into();
        tn.object_name = t.label.as_str().into();
        tn.parent = Some(0);
        own.push(tn);
        own_children.push(Vec::new());
        own_children[0].push(At::Own(ti));
        while k < order.len() && order[k].thread == *t {
            let p = order[k];
            let ci = own.len();
            let r = &p.nodes[0];
            own.push(Node {
                parent: Some(ti),
                function: format!("Codeunit {} \"{}\"", p.codeunit, p.name).into(),
                object_type: "CodeUnit",
                keyword: "Codeunit",
                object_name: p.name.as_str().into(),
                object_id: p.codeunit,
                app: std::sync::Arc::new(p.app.clone()),
                calls: 1,
                total_ns: r.total_ns,
                child_ns: r.child_ns,
                samples: r.samples,
                sample_ns: r.sample_ns,
                ..Node::default()
            });
            own_children.push(r.children.iter().map(|&n| At::Of(k, n)).collect());
            own_children[ti].push(At::Own(ci));
            own[ti].total_ns += r.total_ns;
            own[ti].child_ns += r.total_ns;
            k += 1;
        }
        own[0].total_ns += own[ti].total_ns;
        own[0].child_ns += own[ti].total_ns;
    }
    let mut t = Tree {
        codeunits: &order,
        own,
        own_children,
        own_size: Vec::new(),
        size: Vec::new(),
    };
    t.sizes();
    let start = c.codeunits.iter().map(|p| p.start_unix_us).min().unwrap_or(0);
    let end = c
        .codeunits
        .iter()
        .map(|p| p.start_unix_us + p.wall_ns / 1000)
        .max()
        .unwrap_or(start);
    let activity = format!(
        "al-runner: {} test codeunits on {} threads; thread totals are busy time and together exceed the wall time",
        c.codeunits.len(),
        threads.len()
    );
    let runner = runner_block(&c.options, &c.codeunits);
    document(
        w,
        &t,
        Some(At::Own(0)),
        sampling,
        start,
        end - start,
        &activity,
        &runner,
        url_of,
        stop,
    )
}
