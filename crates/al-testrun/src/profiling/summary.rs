//! The text summary of a profile (`summary.txt` beside the `.alcpuprofile` files, and the file of the deprecated
//! `AL_PROFILE` variable): a header that names the mode and the unit, the time per test codeunit and test, then the
//! routines by self and inclusive time, the statements, and the record operations per routine.

use super::{Collected, Node, self_ns};
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

const TOP: usize = 60;

/// The first line: the mode, the unit, the threads.
pub fn header(c: &Collected) -> String {
    let threads: BTreeSet<usize> = c.codeunits.iter().map(|p| p.thread.id).collect();
    match c.options.mode {
        super::Mode::Sampling => {
            let us = c.options.interval_us;
            let every = if us % 1000 == 0 {
                format!("{} ms", us / 1000)
            } else {
                format!("{us} us")
            };
            let samples: u64 = c.codeunits.iter().map(|p| p.samples).sum();
            format!(
                "samples {samples} of {every} on {} threads (each thread sampled every tick); times in ms",
                threads.len()
            )
        }
        super::Mode::Instrumentation => format!(
            "instrumentation on {} threads: exact entry and exit times, call counts; times in ms",
            threads.len()
        ),
    }
}

fn ms(ns: u64) -> String {
    format!("{:.3}", ns as f64 / 1e6)
}

fn top<K: Ord + Clone>(m: &HashMap<K, (u64, u64)>) -> Vec<(K, (u64, u64))> {
    let mut v: Vec<(K, (u64, u64))> = m
        .iter()
        .filter(|(_, x)| x.0 > 0 || x.1 > 0)
        .map(|(k, x)| (k.clone(), *x))
        .collect();
    v.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(&b.0)));
    v.truncate(TOP);
    v
}

/// The inclusive time of a node: instrumentation its total, sampling its samples and those below it.
fn inclusive(nodes: &[Node], i: usize) -> u64 {
    let n = &nodes[i];
    if n.total_ns > 0 {
        return n.total_ns;
    }
    n.sample_ns + n.db.iter().map(|d| d.ns).sum::<u64>() + n.children.iter().map(|&c| inclusive(nodes, c)).sum::<u64>()
}

/// The whole summary.
pub fn render(c: &Collected) -> String {
    let mut out = header(c);
    out.push_str("\n\ntest codeunits (wall ms per codeunit and test, thread)\n");
    for p in &c.codeunits {
        let _ = writeln!(
            out,
            "{:>10} codeunit {} \"{}\"  [{}]",
            ms(p.wall_ns),
            p.codeunit,
            p.name,
            p.thread.label
        );
        for (t, d) in &p.tests {
            let _ = writeln!(out, "{:>10}   {t}", ms(d.as_nanos() as u64));
        }
    }
    // (time ns, calls or samples) per frame name.
    let mut own: HashMap<String, (u64, u64)> = HashMap::new();
    let mut incl: HashMap<String, (u64, u64)> = HashMap::new();
    let mut stmts: HashMap<String, (u64, u64)> = HashMap::new();
    let mut db: HashMap<(String, String), (u64, u64)> = HashMap::new();
    for p in &c.codeunits {
        for (i, n) in p.nodes.iter().enumerate().skip(1) {
            let hits = if p.sampling { n.samples } else { n.calls };
            let e = own.entry(n.frame_name.clone()).or_default();
            e.0 += self_ns(n);
            e.1 += hits;
            // Inclusive once per path: a recursive frame counts at its outermost node only.
            let mut up = n.parent;
            let mut nested = false;
            while let Some(q) = up {
                nested |= p.nodes[q].frame_name == n.frame_name;
                up = p.nodes[q].parent;
            }
            if !nested {
                let e = incl.entry(n.frame_name.clone()).or_default();
                e.0 += inclusive(&p.nodes, i);
                e.1 += n.calls.max(n.samples);
            }
            let file = n
                .file
                .as_deref()
                .map_or("?", |f| f.rsplit(['/', '\\']).next().unwrap_or(f));
            for s in &n.statements {
                let e = stmts
                    .entry(format!("{}  {file}:{}", n.frame_name, s.line + 1))
                    .or_default();
                e.0 += s.ns;
                e.1 += s.hits;
            }
            for d in &n.db {
                let e = db
                    .entry((n.frame_name.clone(), super::alcpuprofile::db_label(d)))
                    .or_default();
                e.0 += d.ns;
                e.1 += d.count;
            }
        }
    }
    let count_word = if c.options.mode == super::Mode::Sampling {
        "samples"
    } else {
        "calls"
    };
    let _ = writeln!(out, "\nself (ms, {count_word})");
    for (k, (t, n)) in top(&own) {
        let _ = writeln!(out, "{:>10} {n:>8} {k}", ms(t));
    }
    let _ = writeln!(out, "\ninclusive (ms, {count_word})");
    for (k, (t, n)) in top(&incl) {
        let _ = writeln!(out, "{:>10} {n:>8} {k}", ms(t));
    }
    let what = if c.options.mode == super::Mode::Sampling {
        "samples"
    } else {
        "runs; time includes the calls the statement made"
    };
    let _ = writeln!(out, "\nby statement (ms, {what})");
    for (k, (t, n)) in top(&stmts) {
        let _ = writeln!(out, "{:>10} {n:>8} {k}", ms(t));
    }
    out.push_str(
        "\nrecord operations per routine (ms in memory, count): these are SQL calls on BC; their time here is in memory, \
         far below BC's\n",
    );
    let mut rows: Vec<((String, String), (u64, u64))> = db.into_iter().collect();
    rows.sort_by(|a, b| b.1.1.cmp(&a.1.1).then(a.0.cmp(&b.0)));
    rows.truncate(TOP);
    for ((frame, op), (t, n)) in rows {
        let _ = writeln!(out, "{:>10} {n:>8} {frame}  {op}", ms(t));
    }
    out
}
