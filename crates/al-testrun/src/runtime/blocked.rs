//! What a test cannot run for: install codeunits, tables a test touches, blocked routines.
use super::written::{ReachIndex, report_that_cannot_run};
use super::*;

/// Runs the install code of the apps under test (`install.rs`, ADR-046 step A): per app, in dependency order, its
/// per-company routines in the session company, then its per-database routines with no company chosen. A routine that
/// cannot run is skipped; returned are the tables it touches and why, so tests that touch them can be reported not run.
///
/// A routine a lean compile left out (`Lowered::left_out`) does not run: the compile is lean only for tests that
/// read no data (`observe.rs`), and BC runs install code in a session of its own, so nothing else of it reaches a test's
/// session (the install baseline, which runs it once in a session of its own, gives the same).
pub(crate) fn run_install_codeunits(lowered: &Lowered, session: &mut Session) -> Vec<al_vm::SkippedInstall> {
    crate::install::run_apps_under_test(lowered, session)
}

/// The tables of `skipped` (what [`run_install_codeunits`] returns): the install code that could not run touches them, so
/// the rows they hold are not BC's. Code that reads or writes one, through any path, is not run. The readers, each
/// watched: a test and the test codeunit's OnRun (`run.rs`, the store's watch next to the static check of their record
/// variables); the install code that runs after it (`install::watching_skipped`), with the Company Triggers subscribers of
/// the install session; and the code that runs when the test session opens the company, its Company Triggers subscribers
/// and the login subscribers of the apps under test (`install::test_session_company_open`).
///
/// The tables a skipped install holds on assumption (`SkippedInstall::assumed`) are watched too; what their readers get
/// is [`Assumed`].
pub(crate) fn skipped_install_tables(skipped: &[al_vm::SkippedInstall]) -> BTreeSet<u32> {
    (skipped.iter())
        .flat_map(|s| s.tables.iter().chain(&s.assumed).copied())
        .collect()
}

/// What code that read or wrote a table only `SkippedInstall::assumed` holds gets: the tables that only the subscribers of
/// table events of a table the install code only reads would add (`install.rs` `touched`, assumption
/// `table-reads-raise-no-table-event`, corpus PR 1431, open).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Assumed {
    /// It runs, marked with the assumption: a test method (its own marks), and the code that runs when the test session
    /// opens the company (its marks count for every test of the session, `run.rs` `open_company`).
    Marked,
    /// It is not run, as for a table of `SkippedInstall::tables` (main's result): the test codeunit's OnRun and the install
    /// code that runs after it, whose marks count for no test.
    Refused,
}

/// Runs `run` with the tables of `skipped` watched ([`skipped_install_tables`]), and clears the watch after it. Returns
/// what `run` returned and, when it read or wrote one of them, why that table's rows are not known
/// ([`skipped_install_reason`]). Nothing is watched when `skipped` holds no table.
pub(crate) fn watching_skipped_installs<R>(
    lowered: &Lowered,
    session: &mut Session,
    skipped: &[al_vm::SkippedInstall],
    assumed: Assumed,
    run: impl FnOnce(&mut Session) -> R,
) -> (R, Option<String>) {
    let tables = skipped_install_tables(skipped);
    if tables.is_empty() {
        return (run(session), None);
    }
    session.store_mut().watch(tables);
    let ran = run(session);
    let hits = session.store_mut().watch_hits();
    session.store_mut().watch(Default::default());
    let hit = skipped_install_hit(skipped, &hits, assumed).map(|(t, why)| skipped_install_reason(lowered, t, why));
    (ran, hit)
}

/// Why code that read or wrote table `t` of a skipped install (`why`: why that install did not run) is not run.
pub(crate) fn skipped_install_reason(lowered: &Lowered, t: u32, why: &str) -> String {
    format!(
        "it read or wrote table {}, whose rows install code that did not run to its end leaves unknown: {why}",
        lowered.program.objects.get(t as usize).map_or("?", |o| o.name.as_str())
    )
}

/// Of the tables `hits` that code read or wrote, the first of `skipped` that refuses it: the table and why its rows are
/// not known. A table of `SkippedInstall::assumed` refuses as one of `SkippedInstall::tables` under `Assumed::Refused`;
/// under `Assumed::Marked`, when nothing refuses, a hit on one marks `table-reads-raise-no-table-event`.
pub(crate) fn skipped_install_hit<'a>(
    skipped: &'a [al_vm::SkippedInstall],
    hits: &BTreeSet<u32>,
    assumed: Assumed,
) -> Option<(u32, &'a String)> {
    let refuses = |s: &al_vm::SkippedInstall, t: &u32| {
        s.tables.contains(t) || (assumed == Assumed::Refused && s.assumed.contains(t))
    };
    let hit = (hits.iter()).find_map(|t| skipped.iter().find(|s| refuses(s, t)).map(|s| (*t, &s.why)));
    if hit.is_none() && skipped.iter().any(|s| !s.assumed.is_disjoint(hits)) {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TableReadsRaiseNoTableEvent);
    }
    hit
}

/// Whether the table at object index `table` is a platform table whose rows are computed from the compiled objects
/// only (AllObj, AllObjWithCaption): no database write, and so no install code, changes them.
pub(crate) fn listed_from_objects(lowered: &Lowered, table: u32) -> bool {
    use al_builtins::platform::{Provider, REGISTRY, Virtual};
    let Some(id) = lowered.program.objects.get(table as usize).and_then(|o| o.id) else {
        return false;
    };
    REGISTRY
        .iter()
        .any(|(rid, _, p)| *rid == id && matches!(p, Provider::Virtual(Virtual::AllObj | Virtual::AllObjWithCaption)))
}

/// Tables whose records appear in the slots of anything `routine` can reach.
pub(crate) fn tables_touched(lowered: &Lowered, object: usize, routine: usize) -> BTreeSet<u32> {
    record_tables_reached(lowered, object, routine, true)
}

/// [`tables_touched`] without the records declared temporary: a record declared temporary reads only its own rows, and its
/// Insert and Modify leave the stored table as it was (record/temporary/QdStoredToTempParam.al:123, :127-135). A record
/// variable not declared temporary counts, also when it is handed a temporary record. For install code that did not run to
/// its end (`install::touched`).
pub(crate) fn tables_touched_not_temporary(lowered: &Lowered, object: usize, routine: usize) -> BTreeSet<u32> {
    record_tables_reached(lowered, object, routine, false)
}

/// The tables of the record slots of anything `routine` of `object` can reach; those declared temporary only with
/// `temporary`.
fn record_tables_reached(lowered: &Lowered, object: usize, routine: usize, temporary: bool) -> BTreeSet<u32> {
    let record_tables = |t: &al_builtins::value::RtType, out: &mut BTreeSet<u32>| {
        if let al_builtins::value::RtType::Record(id, temp) = t {
            if temporary || !temp {
                out.insert(*id);
            }
        }
    };
    let mut out = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut stack = vec![(object, routine)];
    while let Some((o, r)) = stack.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        let obj = &lowered.program.objects[o];
        for s in obj.routines[r].locals.iter().chain(&obj.globals) {
            record_tables(&s.ty, &mut out);
        }
        lowered.calls_into(o, r, &mut stack);
    }
    out
}

/// Subscriber routines (object, routine) that `routine` of `object` can set off
/// through static calls: those of the events it publishes, and those of the table
/// events of every table whose records it uses -- followed through the routines
/// those subscribers call in turn.
pub(crate) fn subscribers_reached(
    engine: &Engine,
    lowered: &Lowered,
    object: usize,
    routine: usize,
) -> Vec<(usize, usize)> {
    subscribers_reached_from(engine, lowered, &[(object, routine)])
        .into_iter()
        .collect()
}

/// [`subscribers_reached`] of each of the routines `roots`, together: one walk of the union of their reaches. The walk
/// finds a subscriber from the routine it visits alone, and it visits the routines the walk of some root visits, so it
/// finds the subscribers the walks of the roots find (the install code of an app shares most of its reach).
pub(crate) fn subscribers_reached_from(
    engine: &Engine,
    lowered: &Lowered,
    roots: &[(usize, usize)],
) -> BTreeSet<(usize, usize)> {
    subscribers_reached_with(engine, lowered, roots, false)
}

/// [`subscribers_reached_from`] without the subscribers of the ten table events of a table that every routine of the walk
/// only reads, in a shape corpus PR 1431 measured (`table_reads::ReadOnlyTables`): the reach `install.rs` `touched`
/// narrows on assumption `table-reads-raise-no-table-event`.
pub(crate) fn subscribers_reached_reading(
    engine: &Engine,
    lowered: &Lowered,
    roots: &[(usize, usize)],
) -> BTreeSet<(usize, usize)> {
    subscribers_reached_with(engine, lowered, roots, true)
}

/// The walk of [`subscribers_reached_from`] (`reading` false) and [`subscribers_reached_reading`] (`reading` true).
fn subscribers_reached_with(
    engine: &Engine,
    lowered: &Lowered,
    roots: &[(usize, usize)],
    reading: bool,
) -> BTreeSet<(usize, usize)> {
    let events = &engine.events;
    let objs = &lowered.program.objects;
    let mut seen = BTreeSet::new();
    let mut reached = BTreeSet::new();
    let mut stack = roots.to_vec();
    let mut read_only = reading.then(super::table_reads::ReadOnlyTables::default);
    while let Some((o, r)) = stack.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        lowered.calls_into(o, r, &mut stack);
        let mut reach = |sub: (usize, usize)| {
            if reached.insert(sub) {
                stack.push(sub);
            }
        };
        if let Some(p) = events.publishers.get(&(o, r)) {
            for sub in events
                .subscribers
                .get(&(p.object, p.event.clone()))
                .into_iter()
                .flatten()
            {
                reach((sub.object, sub.routine));
            }
        }
        // Table events fire for any table whose record the routine uses: every event of the table object. The result is
        // a set, so the order the subscribers are found in does not matter.
        let mut tables = BTreeSet::new();
        for s in objs[o].routines[r].locals.iter().chain(&objs[o].globals) {
            if let al_builtins::value::RtType::Record(t, _) = &s.ty {
                tables.insert(*t as usize);
            }
        }
        match &mut read_only {
            Some(walk) => walk.visit(engine, lowered, (o, r), &tables, &mut reach),
            None => {
                for t in tables {
                    events.subscribers_of(t).iter().copied().for_each(&mut reach);
                }
            }
        }
    }
    reached
}

/// Why routine `routine` of `object` cannot run: the first reason found in
/// anything it can reach through static calls (hard rule 4: decided before running).
pub fn blocked_reason(lowered: &Lowered, object: usize, routine: usize) -> Option<String> {
    blocked_site(lowered, object, routine).map(|b| b.text)
}

/// A routine that cannot run, found from a root by [`blocked_site`].
pub struct Blocked {
    /// The routine that carries the lowering reason.
    pub object: usize,
    pub routine: usize,
    /// `Object.Routine: reason`.
    pub text: String,
}

/// [`blocked_reason`] with the routine the reason was found in.
pub fn blocked_site(lowered: &Lowered, object: usize, routine: usize) -> Option<Blocked> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![(object, routine)];
    // `AL_BLOCK_PATH=1` prints the chain of calls from the root to the routine with the reason.
    let trace = std::env::var_os("AL_BLOCK_PATH").is_some();
    let mut parent = std::collections::BTreeMap::new();
    while let Some((o, r)) = stack.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        // The routine that writes an expression-bound page control (`pages::expand`) is optional: when it cannot be
        // lowered the control stays read-only, and a test that never sets the control is not refused for it.
        if lowered.program.objects[o].routines[r].name.starts_with("__sourceset_") {
            continue;
        }
        // A page action's OnAction, a report's trigger and a TryFunction run only on a path the run may never take (al-runner
        // issues 6014, 6112): what they reach is refused when it runs (`Session::call` checks `Engine::blocked` on every
        // call), not before. A routine the root reaches another way is still found on that way. The root itself is checked.
        if (o, r) != (object, routine) && refused_when_run(lowered, o, r) {
            continue;
        }
        if let Some(reason) = lowered.unsupported[o][r].first() {
            let name = &lowered.program.objects[o].routines[r].name;
            let obj = &lowered.program.objects[o].name;
            if trace {
                let mut at = (o, r);
                let mut chain = vec![at];
                while let Some(&p) = parent.get(&at) {
                    chain.push(p);
                    at = p;
                }
                for (co, cr) in chain.iter().rev() {
                    let ob = &lowered.program.objects[*co];
                    eprintln!("  via {} {} .{}", ob.kind, ob.name, ob.routines[*cr].name);
                }
            }
            return Some(Blocked {
                object: o,
                routine: r,
                text: format!("{obj}.{name}: {reason}"),
            });
        }
        let before = stack.len();
        calls_before_run(lowered, o, r, &mut stack);
        if trace {
            for &next in &stack[before..] {
                parent.entry(next).or_insert((o, r));
            }
        }
    }
    None
}

/// Whether routine `r` of object `o` holds a `Permissions Mock` codeunit variable, itself or as a global of the object:
/// the test can lower its permissions (ADR-062).
pub fn holds_permissions_mock(lowered: &Lowered, o: usize, r: usize) -> bool {
    let objs = &lowered.program.objects;
    let obj = &objs[o];
    obj.routines[r].locals.iter().chain(&obj.globals).any(|s| {
        matches!(&s.ty, al_builtins::value::RtType::Codeunit(c)
            if objs[*c as usize].name.eq_ignore_ascii_case("Permissions Mock"))
    })
}

/// ADR-062 stage 2: for a test that can lower its permissions, the first part of its reach that the analysis cannot follow (a
/// report the runtime refuses). A denied read is then refused at the read, not later, after the test changed state a later
/// test of the codeunit reads.
pub(crate) fn refusal_after_denial(
    engine: &Engine,
    lowered: &Lowered,
    reach_index: &mut Option<Arc<ReachIndex>>,
    object: usize,
    r: usize,
) -> Option<String> {
    if !holds_permissions_mock(lowered, object, r) {
        return None;
    }
    let index = reach_index.get_or_insert_with(|| engine.derived(|| ReachIndex::new(&engine.events, lowered)));
    // Only a report the runtime refuses counts. Any other unknown part of the reach (a routine of a dependency app that no code
    // reached while compiling) does not say that a report is coming: refusing the first denied read for it left the shared state
    // of Y24PermReads.al half written and made its later tests untrusted.
    report_that_cannot_run(&engine.events, lowered, index, object, r)
}

/// Whether routine `r` of object `o` is the OnAction trigger of an action of a page or page extension.
pub fn is_action_trigger(lowered: &Lowered, o: usize, r: usize) -> bool {
    let obj = &lowered.program.objects[o];
    let rt = &obj.routines[r];
    (obj.is_kind(al_ir::ObjKind::Page) || obj.is_kind(al_ir::ObjKind::PageExtension))
        && rt.is_trigger
        && rt.name.eq_ignore_ascii_case("onaction")
        && rt.container.as_ref().is_some_and(|(k, _)| k == "action")
}

/// Whether the refusal before the run stops at routine `r` of object `o` (`blocked_site`, the reach report): what it
/// reaches is refused when it runs, by `Engine::blocked`. A page action's OnAction (al-runner issue 6014); a trigger of a
/// report or report extension, which runs only when the report runs (issue 6112: an install routine that holds a report
/// whose OnInitReport reaches `Image.Resize`); and a TryFunction (issue 6112: `TryResizeImage`). An Unsupported raised
/// inside a TryFunction is not caught by it: only `Signal::Error` is (`interp.rs`, the call of a used TryFunction).
pub fn refused_when_run(lowered: &Lowered, o: usize, r: usize) -> bool {
    let obj = &lowered.program.objects[o];
    let Some(rt) = obj.routines.get(r) else {
        return false;
    };
    is_action_trigger(lowered, o, r)
        || (rt.is_trigger && (obj.is_kind(al_ir::ObjKind::Report) || obj.is_kind(al_ir::ObjKind::ReportExtension)))
        || rt.attributes.iter().any(|a| a.name.eq_ignore_ascii_case("TryFunction"))
}

/// What routine `r` of object `o` can set running, as the refusal before the run follows it (`blocked_site`, the reach
/// report), appended to `out`: the routines its code calls, and what a page, report or xmlport variable it holds drives,
/// except the procedures of a driven page. Opening a page runs its triggers, not its procedures: a procedure runs only
/// when a trigger or other code calls it, and that call is an edge of its own (al-runner issue 6031). So a procedure that
/// only an action's OnAction calls is refused when the action runs (`Session::call` checks `Engine::blocked`), like the
/// OnAction itself, and one that a trigger running with the page or the test's own code calls is still found that way.
pub fn calls_before_run(lowered: &Lowered, o: usize, r: usize, out: &mut Vec<(usize, usize)>) {
    lowered.direct_calls_into(o, r, out);
    let start = out.len();
    lowered.driven_into(o, r, out);
    let driven = out.split_off(start);
    out.extend(driven.into_iter().filter(|&(q, ri)| !is_page_procedure(lowered, q, ri)));
}

/// Whether routine `r` of object `o` is a procedure of a page, page extension, report or report extension: not a trigger,
/// and not a routine made for a control property (`pages::expand`, its name starts `__`), which the page runs itself. A
/// report variable drives the report's triggers; its procedures run only when a trigger or other code calls them (issue
/// 6112: OnInitReport calls `ReportsForNavInit`).
fn is_page_procedure(lowered: &Lowered, o: usize, r: usize) -> bool {
    let obj = &lowered.program.objects[o];
    let Some(rt) = obj.routines.get(r) else {
        return false;
    };
    (obj.is_kind(al_ir::ObjKind::Page)
        || obj.is_kind(al_ir::ObjKind::PageExtension)
        || obj.is_kind(al_ir::ObjKind::Report)
        || obj.is_kind(al_ir::ObjKind::ReportExtension))
        && !rt.is_trigger
        && !rt.name.starts_with("__")
}
