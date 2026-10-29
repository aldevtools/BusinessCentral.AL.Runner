//! Static selection for an impact run (ADR-043 section 2.3): which tests can reach an impact point of an extension,
//! found from the compiled program instead of from a run.
//!
//! The graph's nodes are the routines of the program, one node per object ("the object is used": a record operation
//! on a table, a page opened, a report run, a codeunit run, an enum's values read), one "written" node per table, one
//! per event, the `Global Triggers` write node, and six nodes for what the program does not name: a codeunit, table,
//! page or report whose id is held in a variable, a RecordRef write whose table the code does not name, and a routine
//! that was not compiled. Edges:
//!
//! - a routine to what it calls (`Lowered::calls_of`: procedures, interface implementers, what page and report
//!   variables drive);
//! - a publisher routine to its event; an event to its subscribers;
//! - a routine to the object of each record, page, test page and enum variable it holds (its locals and its object's
//!   globals), and to the codeunit, page, report or table a `Codeunit.Run`, `Page.Run`, `Report.Run` or
//!   `RecordRef.Open` names with a constant id (else to the matching "id in a variable" node). A codeunit variable
//!   alone runs nothing: its procedures are calls, and `Codeunit.Run` on it is the codeunit's `OnRun`;
//! - a routine that writes or validates a record to the table's written node (`Insert`, `Modify`, `Delete`,
//!   `DeleteAll`, `ModifyAll`, `Rename`, `Truncate`, `Validate` on a record variable, the implicit `Rec` included); one
//!   that writes through a RecordRef or validates a FieldRef to the written node of each table the routine opens by
//!   constant or takes with `GetTable`, and to the "RecordRef write" node (the ref may come from anywhere);
//! - a routine that writes a record to the `Global Triggers` write events;
//! - a table to the tables its fields relate to and the enums of its fields; a table's written node to the table, its
//!   triggers, its extensions' triggers, its events and the `Global Triggers` write node; a page to its routines, its
//!   extensions' routines, its events, its record variables and the written node of its source table (a page inserts,
//!   modifies and deletes rows); a report to its routines, its extensions' routines, its events and the tables of its
//!   records; an xmlport to its routines and the written nodes of its tables (an import writes them); a query to its
//!   routines and its tables; a codeunit to its `OnRun`.
//!
//! A test *reaches* an impact point when a path from one of its roots (the test, its codeunit's `OnRun`, its handler
//! functions) leads to the point without passing a node for what the program does not name, and *cannot rule it
//! out* when every such path passes one. The graph over-selects (a variable of a table counts as using the table, a
//! validation counts as a write, an interface call reaches every implementer, a branch that never runs counts); it
//! misses only what no node stands for: events the platform raises with no static raiser, and setup rows that name an
//! object to run.

use crate::impact_trace::{ImpactPoint, Matcher, Site};
use al_builtins::value::RtType;
use al_ir::{ObjKind, Program};
use al_sema::lower::Lowered;
use std::collections::HashMap;

/// The `Global Triggers` events the runtime raises for table writes (`al_vm` globaltriggers.rs).
pub const GLOBAL_WRITE_EVENTS: [&str; 10] = [
    "getglobaltabletriggermask",
    "getdatabasetabletriggersetup",
    "onglobalinsert",
    "onglobalmodify",
    "onglobaldelete",
    "onglobalrename",
    "ondatabaseinsert",
    "ondatabasemodify",
    "ondatabasedelete",
    "ondatabaserename",
];

/// The record methods that change the database (`al_builtins::record_op::RecordOp::writes`).
const WRITES: [&str; 7] = [
    "Insert",
    "Modify",
    "Delete",
    "DeleteAll",
    "ModifyAll",
    "Rename",
    "Truncate",
];

/// The nodes for what the program does not name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unnamed {
    Codeunit,
    Table,
    TableWrite,
    Page,
    Report,
    NotCompiled,
}

const UNNAMED: [Unnamed; 6] = [
    Unnamed::Codeunit,
    Unnamed::Table,
    Unnamed::TableWrite,
    Unnamed::Page,
    Unnamed::Report,
    Unnamed::NotCompiled,
];

impl Unnamed {
    fn label(self) -> &'static str {
        match self {
            Unnamed::Codeunit => "Codeunit.Run with an id from a variable",
            Unnamed::Table => "RecordRef.Open with a table number from a variable",
            Unnamed::TableWrite => "a RecordRef write to a table the code does not name",
            Unnamed::Page => "Page.Run with an id from a variable",
            Unnamed::Report => "Report.Run with an id from a variable",
            Unnamed::NotCompiled => "a routine that was not compiled",
        }
    }
}

/// The static graph of a program (see the module documentation).
pub struct Graph {
    /// First node of each object's routines.
    base: Vec<usize>,
    /// The number of routine nodes; object `o` is node `routines + o`.
    routines: usize,
    /// (publisher object as `al_vm::events::Registry` keys it, lower-case event) -> node.
    events: HashMap<(usize, String), usize>,
    /// The first of the `UNNAMED` nodes.
    unnamed: usize,
    /// The written node of object `o` is `written + o` (it has edges only for tables).
    written: usize,
    edges: Vec<Vec<u32>>,
    rev: Vec<Vec<u32>>,
    /// Objects by kind and id.
    by_id: HashMap<(ObjKind, u32), usize>,
    /// The object `Global Triggers` subscribers are registered under (`Registry::global_key`).
    global_key: usize,
}

/// The nodes a search found: `precise` without passing an unnamed node, `possible` with or without.
pub struct Reach {
    pub precise: Vec<bool>,
    pub possible: Vec<bool>,
}

impl Graph {
    pub fn build(lowered: &Lowered) -> Graph {
        let program = &lowered.program;
        let registry = al_vm::events::Registry::build(program);
        let objs = &program.objects;
        let mut base = Vec::with_capacity(objs.len());
        let mut routines = 0;
        for o in objs {
            base.push(routines);
            routines += o.routines.len();
        }
        let objects = objs.len();
        let mut events: HashMap<(usize, String), usize> = HashMap::new();
        let mut next = routines + objects;
        let mut event_keys: Vec<(usize, String)> = registry.subscribers.keys().cloned().collect();
        event_keys.extend(registry.publishers.values().map(|p| (p.object, p.event.clone())));
        event_keys.sort();
        event_keys.dedup();
        for k in event_keys {
            events.insert(k, next);
            next += 1;
        }
        let global_write = next;
        let unnamed = next + 1;
        let written = unnamed + UNNAMED.len();
        let n = written + objects;
        let mut g = Graph {
            base,
            routines,
            events,
            unnamed,
            written,
            edges: vec![Vec::new(); n],
            rev: Vec::new(),
            by_id: HashMap::new(),
            global_key: registry.global_key,
        };
        // Objects by kind and id, for the ids `Codeunit.Run`, `Page.Run`, `Report.Run` and `RecordRef.Open` name.
        let mut by_id: HashMap<(ObjKind, u32), usize> = HashMap::new();
        let mut by_name: HashMap<(ObjKind, String), usize> = HashMap::new();
        // The object a page or report extension extends: its routines' `BaseGlobal` places are that object's globals.
        let mut base_of: HashMap<usize, usize> = HashMap::new();
        for (oi, o) in objs.iter().enumerate() {
            if let Some(id) = o.id {
                by_id.entry((o.kind, id)).or_insert(oi);
            }
            by_name.entry((o.kind, o.name.to_lowercase())).or_insert(oi);
            for &e in o.page.iter().flat_map(|p| p.extensions.iter()) {
                base_of.insert(e, oi);
            }
            if let Some(x) = &o.report_ext {
                base_of.insert(oi, x.base);
            }
        }
        let mut edges = std::mem::take(&mut g.edges);
        // The events the platform raises to resolve a caption class.
        let caption_class: Vec<usize> = g
            .events
            .iter()
            .filter(|((o, _), _)| *o == al_vm::events::CAPTION_CLASS)
            .map(|(_, &n)| n)
            .collect();
        let table_write = g.unnamed_node(Unnamed::TableWrite) as u32;
        for (oi, o) in objs.iter().enumerate() {
            for (ri, r) in o.routines.iter().enumerate() {
                let from = g.base[oi] + ri;
                let out = &mut edges[from];
                for (co, cr) in lowered.calls_of(oi, ri) {
                    if co < objs.len() && cr < objs[co].routines.len() {
                        out.push((g.base[co] + cr) as u32);
                    }
                }
                if let Some(p) = registry.publishers.get(&(oi, ri))
                    && let Some(&e) = g.events.get(&(p.object, p.event.clone()))
                {
                    out.push(e as u32);
                }
                if lowered.unsupported[oi][ri]
                    .iter()
                    .any(|s| s.starts_with("not compiled"))
                {
                    out.push(g.unnamed_node(Unnamed::NotCompiled) as u32);
                }
                for s in r.locals.iter().chain(&o.globals) {
                    if let Some(t) = slot_object(objs, &s.ty) {
                        out.push((routines + t) as u32);
                    }
                }
                let ty_of = |p: &al_ir::Place| -> Option<&RtType> {
                    let mut p = p;
                    while let al_ir::Place::Index(b, _) = p {
                        p = b;
                    }
                    let mut ty = match p {
                        al_ir::Place::Local(i) => &r.locals.get(*i)?.ty,
                        al_ir::Place::Global(i) => &o.globals.get(*i)?.ty,
                        al_ir::Place::BaseGlobal(i) => &objs[*base_of.get(&oi)?].globals.get(*i)?.ty,
                        al_ir::Place::ExtGlobal(e, i) => &objs.get(*e)?.globals.get(*i)?.ty,
                        _ => return None,
                    };
                    while let RtType::Array(_, inner) = ty {
                        ty = inner;
                    }
                    Some(ty)
                };
                let type_of = |a: &al_ir::Arg| match a {
                    al_ir::Arg::Ref(p) => ty_of(p),
                    al_ir::Arg::Value(al_ir::Expr::Load(p)) => ty_of(p),
                    _ => None,
                };
                // The tables a RecordRef of the routine is opened on by constant or takes with `GetTable`, and
                // whether the routine writes through a RecordRef.
                let mut ref_tables: Vec<usize> = Vec::new();
                let mut ref_writes = false;
                al_ir::types::for_each_expr_post(&r.body, &mut |e| {
                    // A constant of an enum reads its values (an interface cast of it, a format of it).
                    if let al_ir::Expr::Const(al_ir::Const::Enum(en, _)) = e {
                        out.push((routines + *en as usize) as u32);
                        return;
                    }
                    let al_ir::Expr::Builtin { id, receiver, args, .. } = e else {
                        return;
                    };
                    let name = id.name();
                    if name.starts_with("System.CaptionClassTranslate") {
                        out.extend(caption_class.iter().map(|&n| n as u32));
                    }
                    let method = id.method();
                    let (class, _) = name.split_once('.').unwrap_or((name, ""));
                    let constant = |i: usize| match args.get(i) {
                        Some(al_ir::Arg::Value(al_ir::Expr::Const(al_ir::Const::Integer(n)))) => Some(*n as u32),
                        _ => None,
                    };
                    let receiver_ty = receiver.as_deref().and_then(type_of);
                    let named = |kind: ObjKind, unnamed: Unnamed, out: &mut Vec<u32>| match constant(0) {
                        Some(id) => {
                            if let Some(&t) = by_id.get(&(kind, id)) {
                                out.push((routines + t) as u32);
                                return Some(t);
                            }
                            None
                        }
                        None => {
                            out.push(g.unnamed_node(unnamed) as u32);
                            None
                        }
                    };
                    match (class, method) {
                        ("Codeunit", "Run") => match receiver_ty {
                            Some(RtType::Codeunit(c)) => out.push((routines + *c as usize) as u32),
                            _ if receiver.is_none() => {
                                named(ObjKind::Codeunit, Unnamed::Codeunit, out);
                            }
                            _ => out.push(g.unnamed_node(Unnamed::Codeunit) as u32),
                        },
                        ("Page", "Run" | "RunModal") if receiver.is_none() => {
                            named(ObjKind::Page, Unnamed::Page, out);
                        }
                        ("Report", m)
                            if receiver.is_none()
                                && (m.starts_with("Run")
                                    || m.starts_with("SaveAs")
                                    || matches!(m, "Execute" | "Print" | "RdlcLayout" | "WordLayout")) =>
                        {
                            named(ObjKind::Report, Unnamed::Report, out);
                        }
                        ("RecordRef", "Open") => ref_tables.extend(named(ObjKind::Table, Unnamed::Table, out)),
                        ("RecordRef", "GetTable") => {
                            if let Some(RtType::Record(t, _)) = args.first().and_then(type_of) {
                                ref_tables.push(*t as usize);
                            }
                        }
                        ("Record", m) if WRITES.contains(&m) || m == "Validate" => {
                            if WRITES.contains(&m) {
                                out.push(global_write as u32);
                            }
                            match receiver_ty {
                                Some(RtType::Record(t, _)) => out.push((g.written + *t as usize) as u32),
                                _ => out.push(table_write),
                            }
                        }
                        ("RecordRef", m) if WRITES.contains(&m) => {
                            out.push(global_write as u32);
                            ref_writes = true;
                        }
                        ("FieldRef", "Validate") => ref_writes = true,
                        _ => {}
                    }
                });
                if ref_writes {
                    out.push(table_write);
                    out.extend(ref_tables.iter().map(|&t| (g.written + t) as u32));
                }
            }
        }
        // Objects: what using them runs.
        for (oi, o) in objs.iter().enumerate() {
            let from = routines + oi;
            let mut out: Vec<u32> = Vec::new();
            let all = |q: usize, out: &mut Vec<u32>| {
                out.extend((0..objs[q].routines.len()).map(|r| (g.base[q] + r) as u32));
            };
            let triggers = |q: usize, out: &mut Vec<u32>| {
                out.extend(
                    objs[q]
                        .routines
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| r.is_trigger)
                        .map(|(r, _)| (g.base[q] + r) as u32),
                );
            };
            // The tables of the object's record variables: their object nodes, or with `write` their written nodes.
            let record_globals = |q: usize, write: bool, out: &mut Vec<u32>| {
                for s in &objs[q].globals {
                    if let Some(t) = slot_object(objs, &s.ty)
                        && objs[t].kind == ObjKind::Table
                    {
                        out.push((if write { g.written } else { routines } + t) as u32);
                    }
                }
            };
            match o.kind {
                ObjKind::Table => {
                    let mut w = vec![from as u32, global_write as u32];
                    triggers(oi, &mut w);
                    if let Some(t) = &o.table {
                        for &e in &t.extensions {
                            triggers(e, &mut w);
                        }
                        for f in &t.fields {
                            for rel in f.relation_table.iter().chain(&f.relation_tables) {
                                if let Some(&r) = by_name.get(&(ObjKind::Table, rel.to_lowercase())) {
                                    out.push((routines + r) as u32);
                                }
                            }
                            if let Some(e) = slot_object(objs, &f.ty) {
                                out.push((routines + e) as u32);
                            }
                        }
                    }
                    edges[g.written + oi] = w;
                }
                ObjKind::Page => {
                    all(oi, &mut out);
                    // A page resolves the CaptionClass of its controls (`pages/rows.rs`).
                    out.extend(caption_class.iter().map(|&n| n as u32));
                    for &e in o.page.iter().flat_map(|p| p.extensions.iter()) {
                        all(e, &mut out);
                    }
                    record_globals(oi, false, &mut out);
                    // A page inserts, modifies and deletes rows of its source table (its `Rec`).
                    if let Some(RtType::Record(t, _)) = o.rec_slot.and_then(|s| o.globals.get(s)).map(|s| &s.ty) {
                        out.push((g.written + *t as usize) as u32);
                    }
                }
                ObjKind::Report => {
                    all(oi, &mut out);
                    for (e, x) in objs.iter().enumerate() {
                        if x.report_ext.as_ref().is_some_and(|x| x.base == oi) {
                            all(e, &mut out);
                        }
                    }
                    record_globals(oi, false, &mut out);
                }
                // An import inserts and modifies the rows of the xmlport's tables.
                ObjKind::XmlPort => {
                    all(oi, &mut out);
                    record_globals(oi, true, &mut out);
                }
                ObjKind::Query => {
                    all(oi, &mut out);
                    record_globals(oi, false, &mut out);
                }
                ObjKind::Codeunit => {
                    out.extend(
                        o.routines
                            .iter()
                            .enumerate()
                            .filter(|(_, r)| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"))
                            .map(|(r, _)| (g.base[oi] + r) as u32),
                    );
                }
                _ => {}
            }
            edges[from] = out;
        }
        // Events: their subscribers. An event of a table is raised when the table is written, one of a page or report
        // when it is used (the platform raises them).
        for ((object, event), &node) in &g.events {
            for s in registry
                .subscribers
                .get(&(*object, event.clone()))
                .into_iter()
                .flatten()
            {
                if s.object < objs.len() && s.routine < objs[s.object].routines.len() {
                    edges[node].push((g.base[s.object] + s.routine) as u32);
                }
            }
            if *object < objs.len()
                && let Some(raiser) = g.raiser(objs[*object].kind, *object)
                && !registry
                    .publishers
                    .values()
                    .any(|p| p.object == *object && &p.event == event)
            {
                edges[raiser].push(node as u32);
            }
            if *object == registry.global_key && GLOBAL_WRITE_EVENTS.contains(&event.as_str()) {
                edges[global_write].push(node as u32);
            }
        }
        // What the program does not name: any object of the kind.
        for u in UNNAMED {
            let (kind, first) = match u {
                Unnamed::Codeunit => (ObjKind::Codeunit, routines),
                Unnamed::Table => (ObjKind::Table, routines),
                Unnamed::TableWrite => (ObjKind::Table, g.written),
                Unnamed::Page => (ObjKind::Page, routines),
                Unnamed::Report => (ObjKind::Report, routines),
                Unnamed::NotCompiled => continue,
            };
            let node = g.unnamed_node(u);
            edges[node] = objs
                .heads()
                .iter()
                .enumerate()
                .filter(|(_, o)| o.kind == kind)
                .map(|(oi, _)| (first + oi) as u32)
                .collect();
        }
        for e in &mut edges {
            e.sort_unstable();
            e.dedup();
        }
        let mut rev = vec![Vec::new(); n];
        for (from, out) in edges.iter().enumerate() {
            for &to in out {
                rev[to as usize].push(from as u32);
            }
        }
        g.edges = edges;
        g.rev = rev;
        g.by_id = by_id;
        g
    }

    /// The node that raises the platform events of an object: a table's written node, a page's or report's object
    /// node; `None` for other kinds.
    fn raiser(&self, kind: ObjKind, object: usize) -> Option<usize> {
        match kind {
            ObjKind::Table => Some(self.written_node(object)),
            ObjKind::Page | ObjKind::Report => Some(self.object_node(object)),
            _ => None,
        }
    }

    /// The object of this kind and id.
    pub fn object_of(&self, kind: ObjKind, id: u32) -> Option<usize> {
        self.by_id.get(&(kind, id)).copied()
    }

    fn unnamed_node(&self, u: Unnamed) -> usize {
        self.unnamed + u as usize
    }

    fn is_unnamed(&self, n: usize) -> bool {
        (self.unnamed..self.written).contains(&n)
    }

    pub fn routine_node(&self, object: usize, routine: usize) -> usize {
        self.base[object] + routine
    }

    pub fn object_node(&self, object: usize) -> usize {
        self.routines + object
    }

    /// The node for writes of a table (and validations of its fields).
    pub fn written_node(&self, object: usize) -> usize {
        self.written + object
    }

    pub fn event_node(&self, object: usize, event: &str) -> Option<usize> {
        self.events.get(&(object, event.to_lowercase())).copied()
    }

    pub fn len(&self) -> usize {
        self.edges.len()
    }

    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// The nodes from which one of `targets` can be reached.
    pub fn reaching(&self, targets: &[usize]) -> Reach {
        Reach {
            precise: self.search(targets, &self.rev, false),
            possible: self.search(targets, &self.rev, true),
        }
    }

    /// The nodes `roots` can reach.
    pub fn reached_from(&self, roots: &[usize]) -> Reach {
        Reach {
            precise: self.search(roots, &self.edges, false),
            possible: self.search(roots, &self.edges, true),
        }
    }

    fn search(&self, from: &[usize], adj: &[Vec<u32>], through_unnamed: bool) -> Vec<bool> {
        let mut seen = vec![false; adj.len()];
        let mut stack: Vec<usize> = Vec::new();
        for &f in from {
            if !seen[f] {
                seen[f] = true;
                stack.push(f);
            }
        }
        while let Some(n) = stack.pop() {
            for &m in &adj[n] {
                let m = m as usize;
                if seen[m] || (!through_unnamed && self.is_unnamed(m)) {
                    continue;
                }
                seen[m] = true;
                stack.push(m);
            }
        }
        seen
    }

    /// A shortest path from one of `from` to `to`, with `through_unnamed` also through the nodes for what the program
    /// does not name (a diagnostic: why static reach selects).
    pub fn path(&self, from: &[usize], to: usize, through_unnamed: bool) -> Option<Vec<usize>> {
        let mut parent = vec![usize::MAX; self.edges.len()];
        let mut queue = std::collections::VecDeque::new();
        for &f in from {
            if parent[f] == usize::MAX {
                parent[f] = f;
                queue.push_back(f);
            }
        }
        while let Some(n) = queue.pop_front() {
            if n == to {
                let mut out = vec![n];
                let mut m = n;
                while parent[m] != m {
                    m = parent[m];
                    out.push(m);
                }
                out.reverse();
                return Some(out);
            }
            for &m in &self.edges[n] {
                let m = m as usize;
                if parent[m] == usize::MAX && (through_unnamed || !self.is_unnamed(m)) {
                    parent[m] = n;
                    queue.push_back(m);
                }
            }
        }
        None
    }

    /// A node as text, for `path`.
    pub fn describe(&self, program: &Program, node: usize) -> String {
        let object = |o: usize| {
            let x = &program.objects[o];
            format!("{} {} \"{}\"", x.kind.label(), x.id.unwrap_or_default(), x.name)
        };
        if node < self.routines {
            let o = self.base.partition_point(|&b| b <= node) - 1;
            let x = &program.objects[o];
            return format!("{}.{}", object(o), x.routines[node - self.base[o]].name);
        }
        if node < self.routines + program.objects.len() {
            return object(node - self.routines);
        }
        if node >= self.written {
            return format!("writes of {}", object(node - self.written));
        }
        if self.is_unnamed(node) {
            return UNNAMED[node - self.unnamed].label().to_string();
        }
        if let Some(((o, e), _)) = self.events.iter().find(|(_, n)| **n == node) {
            let publisher = if *o < program.objects.len() {
                object(*o)
            } else {
                format!("platform publisher {o}")
            };
            return format!("event {publisher} {e}");
        }
        "Global Triggers write events".into()
    }

    /// The unnamed nodes `reach` holds, as text for a verdict.
    pub fn unnamed_in(&self, reach: &[bool]) -> Vec<&'static str> {
        UNNAMED
            .iter()
            .filter(|u| reach[self.unnamed_node(**u)])
            .map(|u| u.label())
            .collect()
    }

    /// The node an impact point's site stands for, or `None` when nothing in the graph stands for it.
    pub fn site_nodes(&self, site: &Site) -> Option<Vec<usize>> {
        match site {
            Site::Event(object, event) => self.event_node(*object, event).map(|n| vec![n]),
            Site::Object(o) => Some(vec![self.object_node(*o)]),
            Site::Tables(tables) => Some(tables.iter().map(|&t| self.object_node(t)).collect()),
            Site::None(_) => None,
        }
    }
}

/// The node a key of the impact index (`impact_trace::keys`) stands for: a `table` key the table, a `write` key its
/// written node, `page`, `report` and `enum` keys the object, an event its event node, or for a table, page or report
/// event no one subscribes to the node that raises it (a table's written node, a page or report). `None`: the graph has
/// no node for it.
pub fn key_node(graph: &Graph, program: &Program, key: &str) -> Option<usize> {
    let (what, rest) = key.split_once(' ')?;
    let object = |kind: ObjKind, id: &str| graph.object_of(kind, id.parse().ok()?);
    let simple = |kind: ObjKind| object(kind, rest).map(|o| graph.object_node(o));
    match what {
        "table" => simple(ObjKind::Table),
        "write" => object(ObjKind::Table, rest).map(|o| graph.written_node(o)),
        "page" => simple(ObjKind::Page),
        "report" => simple(ObjKind::Report),
        "enum" => simple(ObjKind::Enum),
        "event" => {
            let mut p = rest.splitn(3, ' ');
            let (kind, id, event) = (p.next()?, p.next()?, p.next()?);
            let event = event.split('/').next()?;
            let o = match (kind, id) {
                ("platform", "captionclass") => al_vm::events::CAPTION_CLASS,
                ("platform", "reportingtriggers") => al_vm::events::REPORTING_TRIGGERS,
                ("codeunit", "2000000002") => graph.global_key,
                _ => object(kind.parse().ok()?, id)?,
            };
            graph
                .event_node(o, event)
                .or_else(|| graph.raiser(program.objects.get(o)?.kind, o))
        }
        _ => None,
    }
}

/// The object a variable of this type uses: a table, page, test page, enum, query, report or xmlport (report and
/// xmlport variables are typed `RtType::Codeunit`). A codeunit variable runs nothing by itself: its procedures are
/// calls (`Lowered::calls_of`) and `Codeunit.Run` on it is handled where it is written.
fn slot_object(objs: &al_ir::Objects, ty: &RtType) -> Option<usize> {
    let mut ty = ty;
    while let RtType::Array(_, inner) = ty {
        ty = inner;
    }
    match ty {
        RtType::Record(t, _) | RtType::Page(t) | RtType::TestPage(t) | RtType::Enum(t) | RtType::Query(t) => {
            Some(*t as usize)
        }
        RtType::Codeunit(t) => objs
            .get(*t as usize)
            .is_some_and(|o| o.kind != ObjKind::Codeunit)
            .then_some(*t as usize),
        _ => None,
    }
}

/// Whether the object is a test codeunit (`Subtype = Test`).
pub fn is_test_codeunit(o: &al_ir::Object) -> bool {
    o.kind == ObjKind::Codeunit
        && o.properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Test"))
}

/// The roots of a test: the test routine, its codeunit's `OnRun` and the handler functions its `[HandlerFunctions]`
/// names.
pub fn test_roots(program: &Program, object: usize, routine: usize) -> Vec<(usize, usize)> {
    let o = &program.objects[object];
    let mut out = vec![(object, routine)];
    out.extend(
        o.routines
            .iter()
            .enumerate()
            .filter(|(_, r)| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"))
            .map(|(r, _)| (object, r)),
    );
    for a in o.routines[routine]
        .attributes
        .iter()
        .filter(|a| a.name.eq_ignore_ascii_case("HandlerFunctions"))
    {
        for arg in &a.args {
            if let al_ir::Const::Text(list) = arg {
                for name in list.split(',').map(str::trim) {
                    out.extend(
                        o.routines
                            .iter()
                            .enumerate()
                            .filter(|(_, r)| r.name.eq_ignore_ascii_case(name))
                            .map(|(r, _)| (object, r)),
                    );
                }
            }
        }
    }
    out
}

/// What static reach says of one test and one impact point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// A path from the test to the point passes only named code.
    Reaches,
    /// Only paths through what the program does not name lead to the point (the text names what).
    CannotRuleOut(String),
}

/// Static selection over a set of tests.
pub struct Selection {
    /// Per impact point: why static selection cannot select for it (every test codeunit runs), else `None`.
    pub everything: Vec<Option<String>>,
    /// Per test (object, routine) that reaches or cannot rule out a point: (point index, verdict).
    pub verdicts: HashMap<(usize, usize), Vec<(usize, Verdict)>>,
}

impl Selection {
    /// Whether the selection must take every test codeunit.
    pub fn takes_everything(&self) -> bool {
        self.everything.iter().any(Option::is_some)
    }
}

/// What the install baseline of the run did, for the impact points that run code: one it reached changes the data
/// every test starts from, so every test runs.
pub enum Install<'a> {
    /// It ran with the trace on: what it raised and touched (`impact_trace::keys`). A point the trace can answer for
    /// (its matcher is a key) is decided by it; another by static reach from `roots` (`baseline_roots`).
    Traced {
        keys: &'a [String],
        roots: &'a [(usize, usize)],
    },
    /// It did not run: no test starts from data it wrote.
    NotRun,
}

/// Static selection (ADR-043 section 2.3): per impact point one search backwards from what stands for it, then each
/// test's roots looked up. `tests` are (object, routine) of test routines.
pub fn select(
    lowered: &Lowered,
    graph: &Graph,
    points: &[ImpactPoint],
    tests: &[(usize, usize)],
    install: Install,
) -> Selection {
    let program = &lowered.program;
    let mut sel = Selection {
        everything: vec![None; points.len()],
        verdicts: HashMap::new(),
    };
    let roots: Vec<(usize, Vec<usize>)> = tests
        .iter()
        .enumerate()
        .map(|(i, &(o, r))| {
            (
                i,
                test_roots(program, o, r)
                    .into_iter()
                    .map(|(o, r)| graph.routine_node(o, r))
                    .collect(),
            )
        })
        .collect();
    let (trace, baseline): (Option<std::collections::HashSet<&str>>, Vec<usize>) = match &install {
        Install::Traced { keys, roots } => (
            Some(keys.iter().map(String::as_str).collect()),
            roots.iter().map(|&(o, r)| graph.routine_node(o, r)).collect(),
        ),
        Install::NotRun => (None, Vec::new()),
    };
    let traced_keys: &[String] = match &install {
        Install::Traced { keys, .. } => keys,
        Install::NotRun => &[],
    };
    // Per unnamed node: the nodes that reach it through named code (to say why a test cannot rule a point out).
    let to_unnamed: Vec<Vec<bool>> = UNNAMED
        .iter()
        .map(|u| graph.search(&[graph.unnamed_node(*u)], &graph.rev, false))
        .collect();
    for (pi, p) in points.iter().enumerate() {
        let Some(targets) = graph.site_nodes(&p.site) else {
            let why = match &p.site {
                Site::None(why) => why.clone(),
                _ => "nothing in the program stands for it".into(),
            };
            sel.everything[pi] = Some(format!("{}: {why}", p.what));
            continue;
        };
        let reach = graph.reaching(&targets);
        // Still decide per test: the report names what each test reaches, and a changed test that reaches nothing is
        // a selection miss even when every codeunit ran.
        if p.runs_code
            && let Some(trace) = &trace
        {
            // The trace answers for a point the index can select for; static reach from the install code for the
            // rest. Static reach alone would decide for nearly every point: it passes code that data decides on.
            let reached = match &p.matcher {
                Matcher::Unsupported(_) => baseline.iter().any(|&b| reach.precise[b]),
                _ => p.reached_by(traced_keys, |k| trace.contains(k)),
            };
            if reached {
                sel.everything[pi] = Some(format!(
                    "{}: the install code reached it, so it changes the data every test starts from",
                    p.what
                ));
            }
        }
        for (i, nodes) in &roots {
            let verdict = if nodes.iter().any(|&n| reach.precise[n]) {
                Verdict::Reaches
            } else if nodes.iter().any(|&n| reach.possible[n]) {
                // Name what the test reaches that can lead to the point.
                let mut via: Vec<&str> = UNNAMED
                    .iter()
                    .zip(&to_unnamed)
                    .filter(|(u, from)| reach.possible[graph.unnamed_node(**u)] && nodes.iter().any(|&n| from[n]))
                    .map(|(u, _)| u.label())
                    .collect();
                if via.is_empty() {
                    via = graph.unnamed_in(&reach.possible);
                }
                Verdict::CannotRuleOut(via.join(", "))
            } else {
                continue;
            };
            sel.verdicts.entry(tests[*i]).or_default().push((pi, verdict));
        }
    }
    sel
}

/// The routines the install baseline runs: Base Application's `Company-Initialize` (its `OnRun`), the install
/// triggers of every install codeunit of an app `is_dependency` accepts, and that app's subscribers to the install events
/// of `Extension Triggers` (ADR-046 decision 1; the extension's own install code is an impact point of its own).
pub fn baseline_roots(program: &Program, is_dependency: impl Fn(usize) -> bool) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = al_vm::events::Registry::build(program)
        .install_subscribers
        .iter()
        .filter(|(so, _, _)| is_dependency(*so))
        .map(|(so, sr, _)| (*so, *sr))
        .collect();
    for (oi, o) in program.objects.iter().enumerate() {
        if o.kind != ObjKind::Codeunit || !is_dependency(oi) {
            continue;
        }
        let install = o
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Install"));
        let company_initialize = o.id == Some(2) && o.name.eq_ignore_ascii_case("Company-Initialize");
        for (ri, r) in o.routines.iter().enumerate() {
            let root = r.is_trigger
                && if install {
                    r.name.eq_ignore_ascii_case("OnInstallAppPerCompany")
                        || r.name.eq_ignore_ascii_case("OnInstallAppPerDatabase")
                } else {
                    company_initialize && r.name.eq_ignore_ascii_case("OnRun")
                };
            if root {
                out.push((oi, ri));
            }
        }
    }
    out.sort_unstable();
    out
}
