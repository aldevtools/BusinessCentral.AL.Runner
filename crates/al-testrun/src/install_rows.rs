//! The rows install code inserts into its own app's tables, where constants show them (al-runner issue 6971).
//!
//! While an app installs, a subscriber of an app under test with no dependency path to it runs only if BC installed the
//! subscriber's app first (ADR-046 amendment 6, install-upgrade/install-order/InstallOrderSubscriberTests.al:66-79), and
//! then the subscriber's own tables hold what its app's install code wrote. For a table that only its own app's install
//! code writes, this module finds the rows that code may insert: a record local whose fields are assigned constants and
//! that is then inserted. A field assigned anything else, or not assigned, counts as any value. The table is left out (its
//! rows are not known) when any of that code modifies or renames its rows, inserts through a parameter, a global or a
//! `Validate`d record, or when a subscriber of its table events exists (it could change a row while it is inserted).
//! al-vm `empty_reads.rs` answers a subscriber's reads of such a table from these rows.

use al_builtins::value::RtType;
use al_ir::{Arg, Expr, Place, Stmt};
use al_sema::lower::Lowered;
use al_vm::{Engine, InstallRows, KnownValue};
use std::collections::{BTreeSet, HashMap};

type Row = Vec<Option<KnownValue>>;

/// The install rows of the tables of the apps under test that only their own app's install code writes (module comment).
pub(crate) fn install_rows(lowered: &Lowered, engine: &Engine) -> InstallRows {
    let objs = &lowered.program.objects;
    let heads = objs.heads();
    let mut writers: HashMap<u32, BTreeSet<usize>> = HashMap::new();
    let mut rows: HashMap<u32, Vec<Row>> = HashMap::new();
    let mut bad: BTreeSet<u32> = BTreeSet::new();
    for app in crate::install::apps_under_test(lowered) {
        let app = app as usize;
        let mut done = BTreeSet::new();
        for (o, r) in crate::install::install_code_of(lowered, engine, |o| {
            !lowered.dependency[o] && objs[o].app as usize == app
        }) {
            if !done.insert((o, r)) {
                continue;
            }
            let Some(routine) = objs[o].routines.get(r) else {
                continue;
            };
            let mut scan = Scan {
                engine,
                routine,
                globals: &objs[o].globals,
                state: HashMap::new(),
                rows: HashMap::new(),
                written: BTreeSet::new(),
                bad: BTreeSet::new(),
            };
            scan.stmts(routine.body.stmts());
            for t in scan.written {
                writers.entry(t).or_default().insert(app);
            }
            bad.extend(scan.bad);
            for (t, mut found) in scan.rows {
                rows.entry(t).or_default().append(&mut found);
            }
        }
    }
    let mut out = InstallRows::default();
    for (t, owners) in writers {
        let owner = heads.get(t as usize).map(|h| h.app as usize);
        if bad.contains(&t)
            || owners.len() != 1
            || owner.is_none_or(|a| !owners.contains(&a))
            || !engine.events.subscribers_of(t as usize).is_empty()
        {
            continue;
        }
        out.tables.insert(t, rows.remove(&t).unwrap_or_default());
    }
    out
}

/// One routine: the rows it inserts into each table, the tables it writes, and those whose rows it leaves not known.
struct Scan<'a> {
    engine: &'a Engine,
    routine: &'a al_ir::Routine,
    globals: &'a [al_ir::Slot],
    /// Per record local, its field values so far.
    state: HashMap<usize, Row>,
    rows: HashMap<u32, Vec<Row>>,
    written: BTreeSet<u32>,
    bad: BTreeSet<u32>,
}

impl Scan<'_> {
    fn fields(&self, t: u32) -> usize {
        self.engine
            .tables
            .get(t as usize)
            .and_then(|d| d.as_ref())
            .map_or(0, |d| d.fields.len())
    }

    /// The table of a declared record local (not a parameter, not temporary).
    fn local_table(&self, l: usize) -> Option<u32> {
        if l < self.routine.params || Some(l) == self.routine.ret_local {
            return None;
        }
        match self.routine.locals.get(l)?.ty {
            RtType::Record(t, false) => Some(t),
            _ => None,
        }
    }

    /// The table of any record place (local, parameter or global), temporary ones left out.
    fn place_table(&self, p: &Place) -> Option<u32> {
        let ty = match p {
            Place::Local(l) => &self.routine.locals.get(*l)?.ty,
            Place::Global(g) => &self.globals.get(*g)?.ty,
            Place::Index(inner, _) => return self.place_table(inner),
            _ => return None,
        };
        match ty {
            RtType::Record(t, false) => Some(*t),
            _ => None,
        }
    }

    fn forget(&mut self, l: usize) {
        if let Some(t) = self.local_table(l) {
            let n = self.fields(t);
            self.state.insert(l, vec![None; n]);
        }
    }

    fn forget_all(&mut self) {
        let locals: Vec<usize> = self.state.keys().copied().collect();
        for l in locals {
            self.forget(l);
        }
    }

    fn row(&mut self, l: usize, t: u32) -> Row {
        let n = self.fields(t);
        self.state.entry(l).or_insert_with(|| vec![None; n]).clone()
    }

    fn merge(&mut self, others: Vec<HashMap<usize, Row>>) {
        for other in others {
            let keys: BTreeSet<usize> = self.state.keys().chain(other.keys()).copied().collect();
            for l in keys {
                match (self.state.get(&l), other.get(&l)) {
                    (Some(a), Some(b)) if a == b => {}
                    (Some(a), Some(b)) => {
                        let m = a.iter().zip(b).map(|(x, y)| if x == y { *x } else { None }).collect();
                        self.state.insert(l, m);
                    }
                    _ => self.forget(l),
                }
            }
        }
    }

    fn stmts(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            self.stmt(s);
        }
    }

    /// Runs `arms` each from the state before them, then joins what they leave.
    fn arms(&mut self, arms: &[&[Stmt]]) {
        let before = self.state.clone();
        let mut after = Vec::new();
        for a in arms {
            self.state = before.clone();
            self.stmts(a);
            after.push(std::mem::take(&mut self.state));
        }
        self.state = before;
        self.merge(after);
    }

    fn stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Assign(p, e) => {
                self.expr(e);
                match p {
                    Place::Field(inner, pos) => {
                        if let Place::Local(l) = &**inner {
                            if let Some(t) = self.local_table(*l) {
                                let mut row = self.row(*l, t);
                                if let Some(slot) = row.get_mut(*pos) {
                                    *slot = constant(e);
                                }
                                self.state.insert(*l, row);
                            }
                        }
                    }
                    Place::Local(l) => self.forget(*l),
                    _ => {}
                }
            }
            Stmt::Expr(e) => self.expr(e),
            Stmt::If(c, a, b) => {
                self.expr(c);
                self.arms(&[a, b]);
            }
            Stmt::Case {
                scrutinee,
                branches,
                otherwise,
            } => {
                self.expr(scrutinee);
                let mut arms: Vec<&[Stmt]> = branches.iter().map(|(_, b)| b.as_slice()).collect();
                arms.push(otherwise);
                self.arms(&arms);
            }
            // A loop body may run again with what the round before left: field values are not known in it, nor after it.
            Stmt::While(c, b) => {
                self.forget_all();
                self.expr(c);
                self.stmts(b);
                self.forget_all();
            }
            Stmt::Repeat(b, c) => {
                self.forget_all();
                self.stmts(b);
                self.expr(c);
                self.forget_all();
            }
            Stmt::For { from, to, body, .. } => {
                self.expr(from);
                self.expr(to);
                self.forget_all();
                self.stmts(body);
                self.forget_all();
            }
            Stmt::Foreach { collection, body, .. } => {
                self.expr(collection);
                self.forget_all();
                self.stmts(body);
                self.forget_all();
            }
            Stmt::AssertError(b) => self.arms(&[b, &[]]),
            Stmt::Exit(Some(e)) => self.expr(e),
            Stmt::Exit(None) | Stmt::Break | Stmt::Continue | Stmt::Pos(_) | Stmt::Assume(_) | Stmt::TddMark(_) => {}
        }
    }

    fn arg_local(a: &Arg) -> Option<usize> {
        match a {
            Arg::Ref(Place::Local(l)) | Arg::Value(Expr::Load(Place::Local(l))) => Some(*l),
            _ => None,
        }
    }

    fn arg_place(a: &Arg) -> Option<&Place> {
        match a {
            Arg::Ref(p) | Arg::Value(Expr::Load(p)) => Some(p),
            Arg::Value(_) => None,
        }
    }

    fn expr(&mut self, e: &Expr) {
        al_ir::types::children(e, &mut |c| self.expr(c));
        match e {
            Expr::Builtin { id, receiver, args, .. } => {
                // A record handed to a builtin as an argument (`Clear(Rec)`, `Rec.Copy(Other)`): its values are not known.
                for a in args {
                    if let Some(l) = Self::arg_local(a) {
                        self.forget(l);
                    }
                }
                let Some(r) = receiver.as_deref() else { return };
                let Some(method) = id.name().strip_prefix("Record.").and_then(|m| m.split('#').next()) else {
                    return;
                };
                let table = Self::arg_place(r).and_then(|p| self.place_table(p));
                let local = Self::arg_local(r).filter(|&l| self.local_table(l).is_some());
                match (method, table) {
                    ("Insert", Some(t)) => {
                        self.written.insert(t);
                        let Some(l) = local else {
                            self.bad.insert(t);
                            return;
                        };
                        // `Insert(true)` runs the OnInsert trigger, which may change any field.
                        let no_trigger = args
                            .iter()
                            .all(|a| matches!(a, Arg::Value(Expr::Const(al_ir::Const::Boolean(false)))));
                        let row = if no_trigger {
                            self.row(l, t)
                        } else {
                            vec![None; self.fields(t)]
                        };
                        self.rows.entry(t).or_default().push(row);
                    }
                    ("Modify" | "Rename" | "ModifyAll" | "Validate", Some(t)) => {
                        self.written.insert(t);
                        self.bad.insert(t);
                    }
                    (
                        "SetRange" | "SetFilter" | "Reset" | "SetCurrentKey" | "IsEmpty" | "Count" | "Delete"
                        | "DeleteAll" | "TableCaption" | "FieldCaption",
                        _,
                    ) => {}
                    _ => {
                        if let Some(l) = local {
                            self.forget(l);
                        }
                    }
                }
            }
            Expr::Call { args, .. } | Expr::InterfaceCall { args, .. } | Expr::DotNet { args, .. } => {
                for a in args {
                    if let Some(l) = Self::arg_local(a) {
                        self.forget(l);
                    }
                }
            }
            _ => {}
        }
    }
}

/// The value of a constant of a type the reads are answered for.
fn constant(e: &Expr) -> Option<KnownValue> {
    match e {
        Expr::Const(al_ir::Const::Integer(n)) => Some(KnownValue::Int(*n)),
        Expr::Const(al_ir::Const::Boolean(b)) => Some(KnownValue::Bool(*b)),
        _ => None,
    }
}
