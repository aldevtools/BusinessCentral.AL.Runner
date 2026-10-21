//! The install baseline (ADR-013 decision 5), mirroring AL Runner (`InstallTriggerRunner`,
//! `CompanyInitializer`, `TestExecutor`): once per run, before the first test,
//!
//! 1. every dependency app's install code (install codeunit triggers and `Extension Triggers` subscribers, ADR-046
//!    decision 1), in dependency order; per app every `OnInstallAppPerCompany`, then every `OnInstallAppPerDatabase`;
//! 2. Base Application's `Company-Initialize` (codeunit 2), once, after the dependency installs;
//! 3. the install code of the apps under test, one app after the other (`install.rs`, ADR-046 step A);
//! 4. a capture of the data ([`Baseline`], ADR-004), put back at the start of every test codeunit.
//!
//! Only install triggers run, never upgrade triggers. A trigger that cannot run stops where it
//! stops: `Company-Initialize` keeps the rows it wrote before (as AL Runner's partial company
//! initialization does), an install trigger is rolled back. What did not run is recorded in the
//! engine's `unseeded` set, so that a `Get` that finds nothing in dependency code is refused for
//! exactly those tables instead of answered.

use al_builtins::native::Signal;
use al_sema::lower::Lowered;
use al_vm::{Baseline, Engine, Session};
use std::collections::BTreeSet;
use std::sync::Arc;

/// What the baseline run leaves.
pub struct BaselineRun {
    pub baseline: Baseline,
    /// Tables the roots that did not run (or stopped early) would have written rows to (a record
    /// variable's Insert, Modify, Rename or ModifyAll in the root's own call closure), and that hold no row in the baseline: object
    /// indexes.
    pub unseeded: BTreeSet<u32>,
    /// How many tables hold a row in the baseline.
    pub filled: usize,
    /// One line per root that could not run to its end.
    pub notes: Vec<String>,
    /// Roots that ran to their end, and all roots.
    pub ran: usize,
    pub total: usize,
    /// Changes the roots made to rows the restored company holds (allowed, ADR-020 amendment).
    pub restored: Vec<al_data::memory::RestoredChange>,
    /// With [`run_traced`]: what the install code raised and touched (`impact_trace::keys`).
    pub trace: Option<Vec<String>>,
}

impl BaselineRun {
    /// One line per table the roots changed restored rows of: how many by kind, and how many
    /// Modify wrote the values the row already had.
    pub fn restored_lines(&self) -> Vec<String> {
        let mut by_table: std::collections::BTreeMap<&str, (usize, usize, usize, usize)> = Default::default();
        let mut fields: std::collections::BTreeMap<&str, std::collections::BTreeSet<&str>> = Default::default();
        for c in &self.restored {
            fields
                .entry(&c.table)
                .or_default()
                .extend(c.fields.iter().map(String::as_str));
            let e = by_table.entry(&c.table).or_default();
            match (c.op, c.same) {
                ("Modify", true) => e.0 += 1,
                ("Modify", false) => e.1 += 1,
                ("Rename", _) => e.2 += 1,
                _ => e.3 += 1,
            }
        }
        by_table
            .into_iter()
            .map(|(t, (same, other, rename, delete))| {
                let names = fields[t].iter().copied().collect::<Vec<_>>().join(", ");
                format!(
                    "install code changed restored rows of {t}: {same} Modify with the values the row has, {other} Modify with other values{}, {rename} Rename, {delete} Delete",
                    if names.is_empty() { String::new() } else { format!(" (fields: {names})") }
                )
            })
            .collect()
    }
}

fn is_install(o: &al_ir::Object) -> bool {
    o.is_kind(al_ir::ObjKind::Codeunit)
        && o.properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Install"))
}

pub(crate) fn trigger(o: &al_ir::Object, name: &str) -> Option<usize> {
    o.routines
        .iter()
        .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case(name))
}

const ASSUMPTION_NOTE: &str = "the baseline relied on the unprobed assumption ";

/// The assumption ids named by the notes of a baseline run (they survive the baseline cache as text).
pub fn assumption_ids(notes: &[String]) -> Vec<String> {
    notes
        .iter()
        .filter_map(|n| n.strip_prefix(ASSUMPTION_NOTE))
        .filter_map(|r| r.strip_prefix('`')?.split('`').next().map(String::from))
        .collect()
}

/// Runs the baseline on a fresh session of `engine`. `None`: no dependency app has anything to run.
pub fn run(engine: &Arc<Engine>, lowered: &Lowered) -> Option<BaselineRun> {
    run_traced(engine, lowered, false)
}

/// [`run`], and with `trace` what the install code raised and touched (`BaselineRun::trace`, ADR-043 step 2).
pub fn run_traced(engine: &Arc<Engine>, lowered: &Lowered, trace: bool) -> Option<BaselineRun> {
    let objs = &lowered.program.objects;
    // (object, routine, keep what it wrote when it stops early, no company chosen)
    //
    // Per dependency app, in dependency order: its install code (install codeunit triggers and "Extension Triggers"
    // subscribers, ADR-046 decision 1), every per-company routine, then every per-database routine, which runs last and
    // with CompanyName() empty. Measured on real BC 27.5 (a fresh install with two companies, a scratch app;
    // docs/probes-needed.md) and pinned by install/lt-bf-install-probes/BfInstallTests.al:138-140, :239.
    let mut roots: Vec<(usize, usize, bool, bool)> = Vec::new();
    for app in crate::install::plan(lowered, &engine.events, true) {
        for &(oi, ri) in &app.company {
            roots.push((oi, ri, false, false));
        }
        for &(oi, ri) in &app.database {
            roots.push((oi, ri, false, true));
        }
    }
    let company_initialize = (0..objs.len()).find(|&oi| {
        lowered.dependency[oi]
            && objs[oi].is_kind(al_ir::ObjKind::Codeunit)
            && objs[oi].id == Some(2)
            && objs[oi].name.eq_ignore_ascii_case("Company-Initialize")
    });
    if let Some(oi) = company_initialize {
        if let Some(ri) = trigger(&objs[oi], "OnRun") {
            roots.push((oi, ri, true, false));
        }
    }
    if roots.is_empty() {
        return None;
    }

    crate::install::ensure_facts(engine, lowered);
    let mut session = Session::with_host(engine.clone(), crate::host::system_host());
    session.set_install_scope(true);
    if trace {
        session.enable_impact_trace();
    }
    // So that a root that stops can say where (fault sites are noted only on an error or a refusal).
    session.trace_faults(true);
    let mut notes = Vec::new();
    let mut touched = BTreeSet::new();
    let mut ran = 0;
    al_builtins::assumptions::clear_hits();
    let company = session.set_company("");
    session.set_company(&company);
    for &(oi, ri, keep_prefix, no_company) in &roots {
        let label = format!("{}.{}", objs[oi].name, objs[oi].routines[ri].name);
        if no_company {
            session.set_company("");
        }
        session.clear_fault_site();
        let called = if is_install(&objs[oi]) || keep_prefix {
            let inst = session.instance(oi);
            session.call(oi, ri, inst, Vec::new()).map(|_| ())
        } else {
            session.call_parameterless_subscriber(oi, ri)
        };
        let why = match called {
            Ok(_) => None,
            // Where it stopped: the innermost routine and its source position.
            Err(Signal::Unsupported(r)) => Some(match session.fault_site().last() {
                Some(f) => {
                    let at = crate::explain::describe(&lowered.program, f.object, f.routine, f.pos);
                    format!("{r} [in {} at {}]", at.who, at.at)
                }
                None => r,
            }),
            Err(Signal::Error(e)) => Some(format!("failed with an error: {}", e.message)),
            Err(_) => None,
        };
        session.set_company(&company);
        match why {
            None => {
                ran += 1;
                session.commit_transaction();
            }
            Some(why) => {
                if keep_prefix {
                    session.commit_transaction();
                } else {
                    session.rollback_transaction();
                }
                notes.push(format!(
                    "{label} did not run to its end{}: {why}",
                    if keep_prefix {
                        " (the rows before it are kept)"
                    } else {
                        ""
                    }
                ));
                touched.extend(tables_written(lowered, oi, ri));
            }
        }
    }
    session.trace_faults(false);
    // The baseline is not a test, so no test carries its marks; the run says what the shared start relied on.
    for a in al_builtins::assumptions::take_hits() {
        notes.push(format!(
            "{ASSUMPTION_NOTE}`{}` (--list-assumptions describes it)",
            a.info().id
        ));
    }
    // The workspace app's own install codeunits run here, once, so that the data they write is part of what every
    // test codeunit starts from (publishing the app runs them once; they are not re-run for each codeunit).
    let workspace_installs = crate::runtime::run_install_codeunits(lowered, &mut session);
    let restored = session.restored_changes();
    let trace = trace.then(|| crate::impact_trace::keys(&lowered.program, &session.take_impact_trace()));
    session.set_install_scope(false);
    let mut baseline = session.capture_baseline();
    baseline.workspace_installs = Some(workspace_installs);
    let filled = baseline.nonempty_tables();
    // A table the restored company holds rows for is seeded, whether or not a root could write it.
    let unseeded = touched
        .difference(&filled)
        .copied()
        .filter(|&t| !session.backs_rows(t))
        .collect();
    Some(BaselineRun {
        baseline,
        unseeded,
        filled: filled.len(),
        notes,
        ran,
        total: roots.len(),
        restored,
        trace,
    })
}

/// The tables that code reachable from `routine` of `object` writes rows to: a record variable's
/// `Insert`, `Modify`, `Rename`, `ModifyAll` or `Validate`-free equivalents, found in the lowered
/// bodies of everything it calls (not of event subscribers). Temporary
/// records are left out. Writes through a `RecordRef` have no static table and are not counted.
/// The Base Application tables whose rows the corpus database's company holds and the runtime does not: Accounting Period
/// (36 rows, 2025-01-01 to 2027-12-01) and Company Information (one row) (session/lt-company-information/
/// CompanyInformationValues.al:45 QG_CompanyInformation_CorpusRow `count` 1, :89 QG_AccountingPeriod_CorpusRows `count` 36).
/// ADR-047 (seed them) is rejected, so without test data a `Count` or `IsEmpty` of one of them before the test's own code
/// wrote to it is refused, the shape those two tests read (install code may have written a row with other values). Other reads stay answered: refusing every read of those tables lost 116 BC 29
/// baseline cases (reports and pages that read Company Information without asserting its values). General Ledger Setup holds
/// one demo row there too (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:804 QF_GLSetup_CorpusRow `count` 1, :815).
/// Object indexes of the dependency tables 50, 79 and 98.
pub fn demo_row_tables(lowered: &Lowered) -> std::collections::HashSet<usize> {
    let objs = &lowered.program.objects;
    (0..objs.len())
        .filter(|&oi| {
            lowered.dependency[oi]
                && objs[oi].table.is_some()
                && matches!(
                    (objs[oi].id, objs[oi].name.as_str()),
                    (Some(50), "Accounting Period")
                        | (Some(79), "Company Information")
                        | (Some(98), "General Ledger Setup")
                )
        })
        .collect()
}

pub(crate) fn tables_written(lowered: &Lowered, object: usize, routine: usize) -> BTreeSet<u32> {
    use al_builtins::value::RtType;
    use al_ir::{Arg, Expr, Place, Stmt};

    struct W<'a> {
        locals: &'a [al_ir::Slot],
        globals: &'a [al_ir::Slot],
        out: &'a mut BTreeSet<u32>,
    }
    impl W<'_> {
        fn root(p: &Place) -> Option<&Place> {
            match p {
                Place::Local(_) | Place::Global(_) => Some(p),
                Place::Index(inner, _) => Self::root(inner),
                // The base object's variable (ADR-023): page and report extension code, which no
                // install routine runs.
                Place::Field(..) | Place::BaseGlobal(_) => None,
            }
        }
        fn record_of(&self, p: &Place) -> Option<u32> {
            let ty = match Self::root(p)? {
                Place::Local(i) => &self.locals.get(*i)?.ty,
                Place::Global(i) => &self.globals.get(*i)?.ty,
                _ => return None,
            };
            match ty {
                RtType::Record(t, false) => Some(*t),
                _ => None,
            }
        }
        fn arg(&mut self, a: &Arg) {
            match a {
                Arg::Value(e) => self.expr(e),
                Arg::Ref(p) => self.place(p),
            }
        }
        fn place(&mut self, p: &Place) {
            match p {
                Place::Index(inner, idx) => {
                    self.place(inner);
                    idx.iter().for_each(|e| self.expr(e));
                }
                Place::Field(inner, _) => self.place(inner),
                _ => {}
            }
        }
        fn expr(&mut self, e: &Expr) {
            match e {
                Expr::Const(_) | Expr::BaseInstance | Expr::ThisInstance | Expr::Refused(_) | Expr::Fail { .. } => {}
                Expr::TddStub { args, .. } => args.iter().for_each(|a| self.expr(a)),
                Expr::Load(p) => self.place(p),
                Expr::Unary(_, a) | Expr::Convert(_, a) | Expr::DotNetCoerce(_, a) | Expr::DotNetOperand(a, _) => {
                    self.expr(a)
                }
                Expr::Binary(_, a, b) | Expr::CharAt(a, b) => {
                    self.expr(a);
                    self.expr(b);
                }
                Expr::In(a, set) => {
                    self.expr(a);
                    for s in set {
                        match s {
                            al_ir::SetElem::One(e) => self.expr(e),
                            al_ir::SetElem::Range(a, b) => {
                                a.iter().chain(b).for_each(|e| self.expr(e));
                            }
                        }
                    }
                }
                Expr::Ternary(a, b, c) => {
                    self.expr(a);
                    self.expr(b);
                    self.expr(c);
                }
                Expr::Call { instance, args, .. } => {
                    instance.iter().for_each(|e| self.expr(e));
                    args.iter().for_each(|a| self.arg(a));
                }
                Expr::InterfaceCall { receiver, args, .. } => {
                    self.expr(receiver);
                    args.iter().for_each(|a| self.arg(a));
                }
                Expr::InterfaceTest { value, .. } | Expr::EnumImpl { value, .. } => self.expr(value),
                Expr::Control { page, .. } => self.expr(page),
                Expr::QueryColumn { query, .. } => self.expr(query),
                Expr::FieldOf(base, _) => self.expr(base),
                Expr::DotNet { receiver, args, .. } => {
                    receiver.iter().for_each(|e| self.expr(e));
                    args.iter().for_each(|a| self.arg(a));
                }
                Expr::Builtin { id, receiver, args, .. } => {
                    if let Some(r) = receiver {
                        if let Some(rest) = id.name().strip_prefix("Record.") {
                            let method = rest.split('#').next().unwrap_or("");
                            if matches!(method, "Insert" | "Modify" | "Rename" | "ModifyAll") {
                                let place = match &**r {
                                    Arg::Ref(p) => Some(p),
                                    Arg::Value(Expr::Load(p)) => Some(p),
                                    Arg::Value(_) => None,
                                };
                                if let Some(t) = place.and_then(|p| self.record_of(p)) {
                                    self.out.insert(t);
                                }
                            }
                        }
                        self.arg(r);
                    }
                    args.iter().for_each(|a| self.arg(a));
                }
            }
        }
        fn stmts(&mut self, body: &[Stmt]) {
            for s in body {
                match s {
                    Stmt::Assign(p, e) => {
                        self.place(p);
                        self.expr(e);
                    }
                    Stmt::Expr(e) => self.expr(e),
                    Stmt::If(c, a, b) => {
                        self.expr(c);
                        self.stmts(a);
                        self.stmts(b);
                    }
                    Stmt::While(c, b) | Stmt::Repeat(b, c) => {
                        self.expr(c);
                        self.stmts(b);
                    }
                    Stmt::For {
                        var, from, to, body, ..
                    } => {
                        self.place(var);
                        self.expr(from);
                        self.expr(to);
                        self.stmts(body);
                    }
                    Stmt::Foreach { var, collection, body } => {
                        self.place(var);
                        self.expr(collection);
                        self.stmts(body);
                    }
                    Stmt::Case {
                        scrutinee,
                        branches,
                        otherwise,
                    } => {
                        self.expr(scrutinee);
                        for (labels, b) in branches {
                            for l in labels {
                                match l {
                                    al_ir::SetElem::One(e) => self.expr(e),
                                    al_ir::SetElem::Range(a, b) => {
                                        a.iter().chain(b).for_each(|e| self.expr(e));
                                    }
                                }
                            }
                            self.stmts(b);
                        }
                        self.stmts(otherwise);
                    }
                    Stmt::Exit(e) => e.iter().for_each(|e| self.expr(e)),
                    Stmt::AssertError(b) => self.stmts(b),
                    Stmt::Break | Stmt::Continue | Stmt::Pos(_) | Stmt::Assume(_) => {}
                }
            }
        }
    }

    let mut out = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut stack = vec![(object, routine)];
    while let Some((o, r)) = stack.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        let obj = &lowered.program.objects[o];
        let rt = &obj.routines[r];
        W {
            locals: &rt.locals,
            globals: &obj.globals,
            out: &mut out,
        }
        .stmts(&rt.body);
        lowered.calls_into(o, r, &mut stack);
    }
    // Rows that event subscribers of those writes add are not counted: which of them run depends on
    // data (a subscriber that writes a table when something else is configured), and counting every
    // subscriber the tables could reach marks tables BC does not fill (measured: Dimension
    // Correction). A table only a subscriber fills stays unmarked (docs/probes-needed.md).
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn assumption_ids_are_read_back_from_the_notes() {
        let notes = vec![
            "Plan Installer.OnInstallAppPerDatabase did not run to its end: x".to_string(),
            "the baseline relied on the unprobed assumption `geturl-web-base-address` (--list-assumptions describes it)".into(),
            "the baseline relied on the unprobed assumption `a-b` (--list-assumptions describes it)".into(),
        ];
        assert_eq!(super::assumption_ids(&notes), vec!["geturl-web-base-address", "a-b"]);
    }
}
