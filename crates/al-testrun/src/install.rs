//! The install code of an app (ADR-046 step A): its `Subtype = Install` codeunits' `OnInstallAppPerCompany` and
//! `OnInstallAppPerDatabase` triggers, and its subscribers to the same two events of `Extension Triggers` (decision 1;
//! events/lt-platform-events-v/PeVFixtures.al:395-407, PeVTests.al:284). [`plan`] lists them per app and part,
//! [`facts`] works out which tables a test may not read because the runtime cannot know their install rows, and
//! [`run_apps_under_test`] runs the apps under test one after the other.
//!
//! What BC does (amendment 1, A1): one app's whole install ends before the next app's begins
//! (install/lt-bf-install-probes/BfInstallTests.al:239); each app installs in a session of its own, not the test
//! session (`:142-144`, `:168`); per company the install session raises Company Triggers OnCompanyOpen and
//! OnCompanyOpenCompleted (to every installed app's subscribers, see [`company_open`]), then runs the routines (`:125-140`,
//! `company open calls`); SingleInstance state starts again in
//! each company and in the per-database part, and carries between the routines of one company (`counter`); the
//! per-database part runs last, with `CompanyName()` empty (`:138-140`). The order of the routines inside one app and part
//! is not known (A2): the runtime runs them in program order and refuses every test that can see that order.

use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::lower::Lowered;
use al_vm::{Engine, InstallFacts, Session};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

/// One part of an app's install: the per-company code, or the per-database code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Part {
    Company,
    Database,
}

impl Part {
    fn event(self) -> &'static str {
        match self {
            Part::Company => "OnInstallAppPerCompany",
            Part::Database => "OnInstallAppPerDatabase",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Part::Company => "per-company",
            Part::Database => "per-database",
        }
    }
}

/// The install routines of one app, per part, in program order (the runtime's own order, A2).
#[derive(Debug, Clone, Default)]
pub(crate) struct AppInstall {
    /// The app's index in `Program::apps` (dependency order).
    pub app: usize,
    pub company: Vec<(usize, usize)>,
    pub database: Vec<(usize, usize)>,
}

impl AppInstall {
    pub(crate) fn part(&self, p: Part) -> &[(usize, usize)] {
        match p {
            Part::Company => &self.company,
            Part::Database => &self.database,
        }
    }
}

fn is_install(o: &al_ir::ObjectHead) -> bool {
    o.is_kind(al_ir::ObjKind::Codeunit)
        && o.properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Install"))
}

fn is_single_instance(o: &al_ir::ObjectHead) -> bool {
    o.is_kind(al_ir::ObjKind::Codeunit)
        && o.properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("SingleInstance") && v.trim().eq_ignore_ascii_case("true"))
}

fn per_company(o: &al_ir::ObjectHead) -> bool {
    !o.properties
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("DataPerCompany") && v.trim().eq_ignore_ascii_case("false"))
}

/// The install routines of every app whose objects `dependency` selects (the dependency apps, or the apps under test),
/// in dependency order. A routine of an app under test that a lean compile left out (`Lowered::left_out`) is not listed:
/// that compile is lean only for tests that read no data (`observe.rs`).
pub(crate) fn plan(lowered: &Lowered, events: &al_vm::events::Registry, dependency: bool) -> Vec<AppInstall> {
    let objs = &lowered.program.objects;
    let mut by_app: BTreeMap<usize, AppInstall> = BTreeMap::new();
    // The dependency apps' install code runs in the baseline whatever the lean compile left out (as before ADR-046).
    let usable = |oi: usize| {
        lowered.dependency[oi] == dependency && (dependency || !lowered.left_out.get(oi).copied().unwrap_or(false))
    };
    let mut routines: Vec<(usize, usize, Part)> = Vec::new();
    for (oi, o) in objs.heads().iter().enumerate() {
        if !usable(oi) || !is_install(o) {
            continue;
        }
        for part in [Part::Company, Part::Database] {
            if let Some(ri) = objs[oi]
                .routines
                .iter()
                .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case(part.event()))
            {
                routines.push((oi, ri, part));
            }
        }
    }
    for (so, sr, event) in &events.install_subscribers {
        if !usable(*so) {
            continue;
        }
        let part = if event == "oninstallapppercompany" {
            Part::Company
        } else {
            Part::Database
        };
        routines.push((*so, *sr, part));
    }
    routines.sort_by_key(|&(o, r, _)| (o, r));
    for (o, r, part) in routines {
        let app = objs[o].app as usize;
        let e = by_app.entry(app).or_insert_with(|| AppInstall {
            app,
            ..Default::default()
        });
        match part {
            Part::Company => e.company.push((o, r)),
            Part::Database => e.database.push((o, r)),
        }
    }
    by_app.into_values().collect()
}

/// `codeunit "Name".Routine`, for notes and reasons.
pub(crate) fn label(lowered: &Lowered, object: usize, routine: usize) -> String {
    let o = &lowered.program.objects[object];
    format!("{} \"{}\".{}", o.kind, o.name, o.routines[routine].name)
}

/// What one install routine's static closure touches: the tables it writes (`tables_written`), the tables it has a
/// record of (read), and the SingleInstance codeunits it reaches.
struct Closure {
    written: BTreeSet<u32>,
    read: BTreeSet<u32>,
    singles: BTreeSet<usize>,
}

fn closure(lowered: &Lowered, object: usize, routine: usize) -> Closure {
    let objs = &lowered.program.objects;
    let mut read = BTreeSet::new();
    let mut singles = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut stack = vec![(object, routine)];
    while let Some((o, r)) = stack.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        let obj = &objs[o];
        if is_single_instance(&objs.heads()[o]) {
            singles.insert(o);
        }
        for s in obj.routines[r].locals.iter().chain(&obj.globals) {
            if let al_builtins::value::RtType::Record(t, false) = &s.ty {
                read.insert(*t);
            }
            if let al_builtins::value::RtType::Codeunit(c) = &s.ty {
                if objs.heads().get(*c as usize).is_some_and(is_single_instance) {
                    singles.insert(*c as usize);
                }
            }
        }
        lowered.calls_into(o, r, &mut stack);
    }
    Closure {
        written: crate::baseline::tables_written(lowered, object, routine),
        read,
        singles,
    }
}

/// The tables whose install rows a test may not read (ADR-046 decision 4c and amendment 1, A2 items a to d), from the
/// static closures of the install routines of the apps under test. The answer depends on the sources only.
///
/// The owner decided that the dependency apps get the refusals of A2 too, whatever they cost. Applied, they lose the
/// baseline case 60293 `RetenPolAllowedTables_IsAllowedTable_UnregisteredTable_IsFalse` (System Application's "Email
/// Installer" and "Retention Policy Installer" both write Retention Policy Allowed Table), so they are worked out and
/// counted (`AL_INSTALL_FACTS=1`, [`facts_of`] with `dependency`) but not applied: docs/owner-review.md.
pub fn facts(lowered: &Lowered, events: &al_vm::events::Registry) -> InstallFacts {
    facts_of(lowered, events, false)
}

/// [`facts`] of the dependency apps (`dependency`) or of the apps under test.
pub fn facts_of(lowered: &Lowered, events: &al_vm::events::Registry, dependency: bool) -> InstallFacts {
    let objs = &lowered.program.objects;
    let app_name = |a: usize| lowered.program.apps.get(a).map_or_else(String::new, |x| x.name.clone());
    let mut out = InstallFacts::default();
    let add = |map: &mut std::collections::HashMap<usize, String>, t: u32, why: String| {
        map.entry(t as usize).or_insert(why);
    };
    {
        for app in plan(lowered, events, dependency) {
            let name = app_name(app.app);
            for part in [Part::Company, Part::Database] {
                let routines = app.part(part);
                let closures: Vec<Closure> = routines.iter().map(|&(o, r)| closure(lowered, o, r)).collect();
                // Decision 4c: BC runs the per-company code once in each company of the database, the runtime in one.
                if !dependency && part == Part::Company {
                    for c in &closures {
                        for &t in &c.written {
                            if objs.heads().get(t as usize).is_some_and(|o| !per_company(o)) {
                                add(
                                    &mut out.unknown,
                                    t,
                                    format!(
                                        "per-company install code of {name} writes this DataPerCompany = false table, and BC runs that code once in each company of the database; this runtime runs one company (ADR-046 decision 4c)"
                                    ),
                                );
                            }
                        }
                    }
                }
                // A4 item 4 (owner decision 2026-10-19): the Company Triggers subscribers of the other installed apps run in
                // the install session ([`company_open`]), an assumption until probe Q3. What they write, and what a
                // per-company routine that reads it or shares a SingleInstance codeunit with them writes, relies on it: a
                // test that touches such a table is a flagged pass. When one of them cannot run here, the routines do not
                // run ([`run_apps_under_test`]).
                if !dependency && part == Part::Company && !routines.is_empty() {
                    let others: Vec<Closure> = company_open_subscribers(lowered, events, |o| {
                        installed(lowered, o, app.app) && objs[o].app as usize != app.app
                    })
                    .into_iter()
                    .map(|(o, r)| closure(lowered, o, r))
                    .collect();
                    let written: BTreeSet<u32> = others.iter().flat_map(|c| c.written.iter().copied()).collect();
                    let singles: BTreeSet<usize> = others.iter().flat_map(|c| c.singles.iter().copied()).collect();
                    for &t in &written {
                        out.assumed.entry(t as usize).or_insert((
                            app.app,
                            al_builtins::assumptions::Assumption::InstallCompanyTriggersAllApps,
                        ));
                    }
                    for c in &closures {
                        if c.read.intersection(&written).next().is_some()
                            || c.singles.intersection(&singles).next().is_some()
                        {
                            for &t in &c.written {
                                out.assumed.entry(t as usize).or_insert((
                                    app.app,
                                    al_builtins::assumptions::Assumption::InstallCompanyTriggersAllApps,
                                ));
                            }
                        }
                    }
                }
                if routines.len() < 2 {
                    continue;
                }
                let order = format!(
                    "{name} has {} install routines in the {} part, and the order BC runs them in is not pinned (ADR-046 amendment 1, A2)",
                    routines.len(),
                    part.label()
                );
                // A2 d: the row versions of what any of them wrote order the routines.
                for c in &closures {
                    for &t in &c.written {
                        add(&mut out.row_order, t, order.clone());
                    }
                }
                for (i, c) in closures.iter().enumerate() {
                    for (j, d) in closures.iter().enumerate() {
                        if i == j {
                            continue;
                        }
                        // A2 a: two routines write one table.
                        for t in c.written.intersection(&d.written) {
                            add(
                                &mut out.unknown,
                                *t,
                                format!("two of its install routines write this table: {order}"),
                            );
                        }
                        // A2 b: one reads what another writes; what the reader writes depends on the order.
                        if c.read.intersection(&d.written).next().is_some() {
                            let reader = label(lowered, routines[i].0, routines[i].1);
                            for &t in &c.written {
                                add(
                                    &mut out.unknown,
                                    t,
                                    format!("{reader} reads a table another install routine writes: {order}"),
                                );
                            }
                        }
                        // A2 c: a SingleInstance codeunit carries state from one routine to the next.
                        if c.singles.intersection(&d.singles).next().is_some() {
                            for &t in c.written.iter().chain(&d.written) {
                                add(
                                    &mut out.unknown,
                                    t,
                                    format!("two of its install routines share a SingleInstance codeunit: {order}"),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// The facts of `engine`'s program, worked out at the first ask. `AL_INSTALL_FACTS=1` prints them, and the shared
/// tables the dependency apps' per-company install code writes (ADR-046 decision 4, "Not decided": not refused).
pub(crate) fn ensure_facts<'a>(engine: &'a Engine, lowered: &Lowered) -> &'a InstallFacts {
    engine.install_facts.get_or_init(|| {
        let f = facts(lowered, &engine.events);
        if std::env::var_os("AL_INSTALL_FACTS").is_some() {
            print_facts(lowered, &engine.events, &f);
        }
        f
    })
}

fn print_facts(lowered: &Lowered, events: &al_vm::events::Registry, f: &InstallFacts) {
    let objs = &lowered.program.objects;
    let name = |t: usize| objs.get(t).map_or_else(String::new, |o| o.name.clone());
    let d = facts_of(lowered, events, true);
    for (what, map) in [
        ("unknown", &f.unknown),
        ("row order", &f.row_order),
        ("dependency apps, not applied: unknown", &d.unknown),
        ("dependency apps, not applied: row order", &d.row_order),
    ] {
        let mut rows: Vec<(String, &String)> = map.iter().map(|(t, w)| (name(*t), w)).collect();
        rows.sort();
        eprintln!("install facts: {} tables {what}", rows.len());
        for (t, w) in rows {
            eprintln!("install facts: {what}: {t}: {w}");
        }
    }
    let mut shared = BTreeSet::new();
    for app in plan(lowered, events, true) {
        for &(o, r) in &app.company {
            for t in crate::baseline::tables_written(lowered, o, r) {
                if objs.heads().get(t as usize).is_some_and(|x| !per_company(x)) {
                    shared.insert(name(t as usize));
                }
            }
        }
    }
    eprintln!(
        "install facts: {} shared tables written by dependency per-company install code (not refused): {}",
        shared.len(),
        shared.into_iter().collect::<Vec<_>>().join(", ")
    );
}

/// Runs the install code of the apps under test, one app after the other in dependency order (decision 3, steps 4a and
/// 4b; amendment 1, A3 and A4). Returned, per routine that could not run: the tables it touches and why, so tests that
/// touch them are reported not run. Each routine commits when it ends; one that stops is rolled back.
pub(crate) fn run_apps_under_test(lowered: &Lowered, session: &mut Session) -> Vec<(BTreeSet<u32>, String)> {
    let engine = session.engine.clone();
    ensure_facts(&engine, lowered);
    let mut skipped = Vec::new();
    let company = session.set_company("");
    session.set_company(&company);
    // BC's install session has no client (`Session::set_install_without_client`).
    session.set_install_without_client(true);
    for (k, app) in plan(lowered, &engine.events, false).into_iter().enumerate() {
        if app.company.is_empty() && app.database.is_empty() {
            continue;
        }
        // A session of its own (A4 item 1): any id that is positive and not the test session's 1.
        session.set_install_session_id(Some(2 + k as i32));
        for part in [Part::Company, Part::Database] {
            let routines = app.part(part);
            if routines.is_empty() {
                continue;
            }
            // A4 item 2: SingleInstance state starts again in each company and in the per-database part.
            session.reset_single_instances();
            if part == Part::Database {
                session.set_company("");
            }
            // A4 item 4: Company Triggers in each company, before the routines.
            let open = if part == Part::Company {
                company_open(lowered, session, app.app)
            } else {
                Ok(())
            };
            match open {
                Err(why) => {
                    session.rollback_transaction();
                    // What the subscribers wrote before the one that could not run is rolled back, and BC would have
                    // kept it: a test that touches what any of them can write is not run either.
                    for (so, sr) in
                        company_open_subscribers(lowered, &engine.events, |o| installed(lowered, o, app.app))
                    {
                        skipped.push((
                            touched(&engine, lowered, so, sr),
                            format!(
                                "Company Triggers subscriber {} in the install session of {} could not run: {why}",
                                label(lowered, so, sr),
                                lowered
                                    .program
                                    .apps
                                    .get(app.app)
                                    .map(|a| a.name.as_str())
                                    .unwrap_or("an app under test")
                            ),
                        ));
                    }
                    for &(oi, ri) in routines {
                        skipped.push((
                            touched(&engine, lowered, oi, ri),
                            format!(
                                "install routine {} could not run: Company Triggers before it: {why}",
                                label(lowered, oi, ri)
                            ),
                        ));
                    }
                }
                Ok(()) => {
                    session.commit_transaction();
                    for &(oi, ri) in routines {
                        if let Some(why) = run_routine(lowered, session, oi, ri) {
                            session.rollback_transaction();
                            skipped.push((
                                touched(&engine, lowered, oi, ri),
                                format!("install routine {} could not run: {why}", label(lowered, oi, ri)),
                            ));
                        } else {
                            // A routine that ran is durable before the next one starts: a later one that cannot run
                            // must not take its writes with it.
                            al_builtins::native::Ctx::commit(session).expect("commit");
                        }
                    }
                }
            }
            session.set_company(&company);
        }
    }
    session.set_install_session_id(None);
    session.set_install_without_client(false);
    // The test session does not see the SingleInstance state of the install run (PeVTests.al:297, fact 6).
    session.reset_single_instances();
    session.commit_transaction();
    skipped
}

/// Whether the app of object `o` is installed when app `app` (an app under test) installs: a dependency app (the install
/// baseline installs them first), an app under test before it in dependency order, or the app itself.
fn installed(lowered: &Lowered, o: usize, app: usize) -> bool {
    lowered.dependency[o] || lowered.program.objects[o].app as usize <= app
}

/// The subscribers of `Company Triggers` OnCompanyOpen, then OnCompanyOpenCompleted, that are not manual and whose object
/// `keep` selects, each event in the order BC raises an event to its subscribers (ascending codeunit id: `al-vm`
/// `dispatch.rs` `event_targets`).
fn company_open_subscribers(
    lowered: &Lowered,
    events: &al_vm::events::Registry,
    keep: impl Fn(usize) -> bool,
) -> Vec<(usize, usize)> {
    let objs = &lowered.program.objects;
    let mut out = Vec::new();
    for event in ["oncompanyopen", "oncompanyopencompleted"] {
        let mut subs: Vec<(usize, usize)> = events
            .subscribers
            .get(&(al_vm::events::COMPANY_TRIGGERS, event.to_string()))
            .into_iter()
            .flatten()
            .filter(|s| !s.manual && keep(s.object))
            .map(|s| (s.object, s.routine))
            .collect();
        subs.sort_by_key(|&(o, _)| objs[o].id.unwrap_or(0));
        out.extend(subs);
    }
    out
}

/// Runs the Company Triggers OnCompanyOpen and OnCompanyOpenCompleted subscribers in the install session of app `app`, as
/// BC does in each company before the per-company install routines (install/lt-bf-install-probes/BfInstallTests.al:125-140,
/// subscribers at events/lt-bf-company-open/BfCompanyOpenFixtures.al:51-71). Pinned for the app's own subscribers; the owner
/// decided that every installed app's subscribers get them (ADR-046 amendment 1, A4 item 4, 2026-10-19), an assumption until
/// probe Q3: the tables that relies on mark the tests that touch them (`facts`, `InstallFacts::assumed`). A subscriber of
/// any app that cannot run here stops the app's per-company install routines, and what it and they touch is refused.
fn company_open(lowered: &Lowered, session: &mut Session, app: usize) -> Result<(), String> {
    let engine = session.engine.clone();
    let all = company_open_subscribers(lowered, &engine.events, |o| installed(lowered, o, app));
    run_subscribers(lowered, session, &all)
}

/// Runs `subs` in order; why the first that cannot run cannot.
fn run_subscribers(lowered: &Lowered, session: &mut Session, subs: &[(usize, usize)]) -> Result<(), String> {
    for &(o, r) in subs {
        if let Some(reason) = crate::runtime::blocked_reason(lowered, o, r) {
            return Err(reason);
        }
        match session.call_parameterless_subscriber(o, r) {
            Ok(()) => {}
            Err(Signal::Unsupported(r)) => return Err(r),
            Err(Signal::Error(e)) => return Err(format!("failed (behaviour not pinned): {}", e.message)),
            Err(_) => return Ok(()),
        }
    }
    Ok(())
}

/// The test session raises Company Triggers OnCompanyOpen, then OnCompanyOpenCompleted, once, before the first test of a
/// codeunit, and not again for later tests (ADR-046 amendment 1, A4 item 10; events/lt-bf-company-open/BfCompanyOpenTests.al:19-38,
/// :40-50; events/lt-platform-events-v/PeVTests.al:297). Here the subscribers of the apps under test run. What the runner
/// does with the database writes of a subscriber is not pinned: the tables they wrote while they ran are returned, and a
/// test that can read one is not run (`CompanyOpenWrites::refuses` before it starts, `CompanyOpenWrites::touched` after it
/// ran; al-runner issues 5606, 5641). A subscriber that did not run to its end refuses the codeunit. The subscribers of
/// dependency apps do not run here (A4 item 10 names the apps under test only).
/// The tables the Company Triggers subscribers of the test session wrote ([`test_session_company_open`]).
#[derive(Debug, Default)]
pub(crate) struct CompanyOpenWrites {
    /// The first subscriber that wrote, for the reason text.
    pub who: Option<String>,
    pub tables: BTreeSet<u32>,
}

impl CompanyOpenWrites {
    fn reason(&self, what: &str) -> String {
        format!(
            "Company Triggers OnCompanyOpen(Completed) subscriber {} writes the database in the test session before the first test, and {what}: what the test runner does with the writes is not pinned (ADR-046 amendment 1, A4 item 10)",
            self.who.as_deref().unwrap_or("?")
        )
    }

    fn table_name(lowered: &Lowered, t: u32) -> &str {
        lowered.program.objects.get(t as usize).map_or("?", |o| o.name.as_str())
    }

    /// Why a routine whose static reach is `facts` is not run: its reach can read a table a subscriber wrote. `None` when
    /// no subscriber wrote, the routine cannot see the writes, or its reach is not known: then the routine runs with the
    /// written tables watched, and [`CompanyOpenWrites::touched`] decides after it ran.
    pub fn refuses(&self, facts: &crate::runtime::written::ReachFacts, lowered: &Lowered) -> Option<String> {
        self.who.as_ref()?;
        if facts.unknown.is_some() {
            return None;
        }
        let t = self.tables.intersection(&facts.touched).next()?;
        Some(self.reason(&format!("this test can read table {}", Self::table_name(lowered, *t))))
    }

    /// Why a test that read or wrote table `t`, which a subscriber wrote, while it ran is not run.
    pub fn touched(&self, t: u32, lowered: &Lowered) -> String {
        self.reason(&format!(
            "this test read or wrote table {} while it ran",
            Self::table_name(lowered, t)
        ))
    }
}

pub(crate) fn test_session_company_open(lowered: &Lowered, session: &mut Session) -> Result<CompanyOpenWrites, String> {
    let engine = session.engine.clone();
    let objs = &lowered.program.objects;
    let mut wrote = CompanyOpenWrites::default();
    let mut subs = Vec::new();
    for event in ["oncompanyopen", "oncompanyopencompleted"] {
        let mut of_event: Vec<(usize, usize)> = engine
            .events
            .subscribers
            .get(&(al_vm::events::COMPANY_TRIGGERS, event.to_string()))
            .into_iter()
            .flatten()
            .filter(|s| !s.manual && !lowered.dependency[s.object])
            .map(|s| (s.object, s.routine))
            .collect();
        of_event.sort_by_key(|&(o, _)| objs[o].id.unwrap_or(0));
        subs.extend(of_event);
    }
    for (o, r) in subs {
        let who = label(lowered, o, r);
        if let Some(reason) = crate::runtime::blocked_reason(lowered, o, r) {
            return Err(format!(
                "Company Triggers OnCompanyOpen(Completed) subscriber {who} before the first test: {reason}"
            ));
        }
        // The subscriber runs here, so what it wrote is what it wrote while it ran, not what its static reach could write:
        // code it did not run (a routine refused at lowering, a run by an id known only at run time, behind a branch that
        // is not taken) wrote nothing (al-runner issue 5641). Code it ran that is not supported raises `Unsupported`.
        let since = session.store_mut().change_counter();
        match session.call_parameterless_subscriber(o, r) {
            Ok(()) => {}
            Err(Signal::Unsupported(why)) => {
                return Err(format!(
                    "Company Triggers OnCompanyOpen(Completed) subscriber {who} before the first test: {why}"
                ));
            }
            Err(Signal::Error(e)) => {
                return Err(format!(
                    "Company Triggers OnCompanyOpen(Completed) subscriber {who} failed before the first test (behaviour not pinned): {}",
                    e.message
                ));
            }
            Err(_) => {
                return Err(format!(
                    "Company Triggers OnCompanyOpen(Completed) subscriber {who} stopped before the first test (not pinned)"
                ));
            }
        }
        let store = session.store_mut();
        // A change to a computed (platform) table is not a stored row the watch can follow.
        if store.virtual_write_pending() {
            return Err(format!(
                "Company Triggers OnCompanyOpen(Completed) subscriber {who} changed a platform table in the test session before the first test: what the test runner does with the change is not pinned (ADR-046 amendment 1, A4 item 10)"
            ));
        }
        let changed = store.tables_changed_since(since);
        if !changed.is_empty() {
            wrote.tables.extend(changed);
            wrote.who.get_or_insert(who);
        }
    }
    Ok(wrote)
}

/// What the login code does to WorkDate when the test session opens the company: Base Application's `LogInManagement`
/// (codeunit 40) sets it to `GetDefaultWorkDate()`, the date of the last G/L entry in a demo company, else the WorkDate it
/// had. Pinned: WorkDate in the test codeunit's OnRun and in its first test is that date in a demo company with G/L
/// entries (session/lt-outside-workdate/RlWorkDateAtStart.al:47-52, al-runner issue 5743); install code still sees today
/// (install/lt-outside-install-ui/RlInstallUi.al:212, :240). The rest of the login code (language, user personalization,
/// application areas) does not run here, as before. Without the codeunit, or in a session without a client, nothing changes.
pub(crate) fn login_work_date(engine: &Engine, lowered: &Lowered, session: &mut Session) -> Result<(), String> {
    if !session.has_interactive_client() {
        return Ok(());
    }
    let objs = &lowered.program.objects;
    let Some(object) = objs.heads().iter().position(|o| {
        o.is_kind(al_ir::ObjKind::Codeunit)
            && o.id == Some(al_sema::check::demand::LOGIN_MANAGEMENT)
            && o.name.eq_ignore_ascii_case("LogInManagement")
    }) else {
        return Ok(());
    };
    let Some(routine) = objs[object].routines.iter().position(|r| {
        !r.is_trigger && r.params == 0 && r.name.eq_ignore_ascii_case(al_sema::check::demand::LOGIN_WORK_DATE)
    }) else {
        return Ok(());
    };
    let why =
        |what: String| format!("the login code that sets WorkDate when the test session opens the company: {what}");
    if let Some(reason) = crate::runtime::blocked_reason(lowered, object, routine) {
        return Err(why(reason));
    }
    // LogInManagement is a SingleInstance codeunit: the session's one instance keeps what the call set.
    let fresh = !session.has_single_instance(object);
    let instance = session.codeunit_instance(object);
    let memo = engine.derived(LoginMemo::default);
    let Some(key) = fresh.then(|| memo_key(session)).flatten() else {
        let called = session.call(object, routine, instance, Vec::new());
        return finish(session, called, why);
    };
    // The threads of a run start their first sessions together: one records, the others wait for its run instead of
    // running the same code beside it.
    let recording = memo.recording.lock().unwrap_or_else(PoisonError::into_inner);
    let found = {
        let mut state = memo.state.lock().unwrap_or_else(PoisonError::into_inner);
        // A warm server runs its next request on the same engine, maybe over other test data.
        if state.backing != Some(key.backing) {
            *state = MemoState {
                backing: Some(key.backing),
                ..Default::default()
            };
        }
        if state.gave_up {
            None
        } else {
            Some(state.runs.iter().find(|r| r.key == key).cloned())
        }
    };
    let Some(found) = found else {
        drop(recording);
        let called = session.call(object, routine, instance, Vec::new());
        return finish(session, called, why);
    };
    // A run recorded over the same baseline by an earlier run of the program, kept in the baseline's file
    // ([`keep_login_records`]): it is taken as if this engine had recorded it.
    let found = found.or_else(|| {
        let record = kept_login_record(&key)?;
        let run = MemoRun {
            key: key.clone(),
            day: record.day,
            globals: record.globals,
            hits: record.hits,
        };
        let mut state = memo.state.lock().unwrap_or_else(PoisonError::into_inner);
        if state.runs.len() == MEMO_RUNS {
            state.runs.remove(0);
        }
        state.runs.push(run.clone());
        Some(run)
    });
    if let Some(run) = found {
        drop(recording);
        memo.replays.fetch_add(1, Ordering::Relaxed);
        {
            let mut inst = instance.borrow_mut();
            for &(g, day, flag) in &run.globals {
                inst.globals[g] = Value::Date(day, flag);
            }
        }
        al_builtins::assumptions::add_hit_bits(run.hits);
        session.set_work_date(run.day);
        return Ok(());
    }
    let hits_before = al_builtins::assumptions::take_hit_bits();
    let before = (
        session.store_mut().change_counter(),
        session.work_date(),
        session.clock_offset_ms(),
        session.last_error.clone(),
    );
    session.enable_impact_trace();
    let called = session.call(object, routine, instance.clone(), Vec::new());
    let touches = session.take_impact_trace();
    session.disable_impact_trace();
    let hits = al_builtins::assumptions::take_hit_bits();
    al_builtins::assumptions::add_hit_bits(hits_before | hits);
    let after = (
        session.store_mut().change_counter(),
        session.work_date(),
        session.clock_offset_ms(),
        session.last_error.clone(),
    );
    let recorded = match &called {
        Ok((Value::Date(day, false), _)) if before == after => {
            replayable(engine, lowered, object, session, &touches, &instance).map(|globals| MemoRun {
                key,
                day: *day,
                globals,
                hits,
            })
        }
        _ => None,
    };
    {
        let mut state = memo.state.lock().unwrap_or_else(PoisonError::into_inner);
        match recorded {
            Some(run) => {
                note_login_record(&run);
                if state.runs.len() == MEMO_RUNS {
                    state.runs.remove(0);
                }
                state.runs.push(run);
            }
            // A run whose effects cannot be replayed: later sessions run the code without waiting for each other.
            None => state.gave_up = true,
        }
    }
    drop(recording);
    finish(session, called, why)
}

/// Sets WorkDate to what the login code answered, or says why it cannot run.
fn finish(
    session: &mut Session,
    called: Result<(Value, Vec<Value>), Signal>,
    why: impl Fn(String) -> String,
) -> Result<(), String> {
    match called {
        Ok((Value::Date(day, false), _)) => {
            session.set_work_date(day);
            Ok(())
        }
        Ok((other, _)) => Err(why(format!("GetDefaultWorkDate answered {}", other.describe()))),
        Err(Signal::Unsupported(r)) => Err(why(r)),
        Err(Signal::Error(e)) => Err(why(format!("failed (not pinned): {}", e.message))),
        Err(_) => Err(why("stopped (not pinned)".into())),
    }
}

/// The login code's recorded runs per baseline origin, for the process: those read from a kept baseline's file
/// ([`keep_login_records`]) and those recorded in this process ([`login_records`] gives them to the file). The origin
/// names what the baseline holds; the file's key names the program, the test data and the runtime build, so a record
/// is taken only over the baseline it was recorded on. The newest origins are kept.
static LOGIN_RECORDS: Mutex<Vec<(u64, Vec<crate::baseline::LoginRecord>)>> = Mutex::new(Vec::new());

/// How many baseline origins [`LOGIN_RECORDS`] keeps.
const KEPT_ORIGINS: usize = 8;

fn with_origin<R>(origin: u64, f: impl FnOnce(&mut Vec<crate::baseline::LoginRecord>) -> R) -> R {
    let mut all = LOGIN_RECORDS.lock().unwrap_or_else(PoisonError::into_inner);
    let at = match all.iter().position(|(o, _)| *o == origin) {
        Some(at) => at,
        None => {
            if all.len() == KEPT_ORIGINS {
                all.remove(0);
            }
            all.push((origin, Vec::new()));
            all.len() - 1
        }
    };
    f(&mut all[at].1)
}

/// Makes the login runs read from the file of a kept baseline (origin `origin`) available to its sessions.
pub(crate) fn keep_login_records(origin: u64, records: &[crate::baseline::LoginRecord]) {
    if records.is_empty() {
        return;
    }
    with_origin(origin, |kept| {
        for r in records {
            if !kept.contains(r) {
                kept.push(r.clone());
            }
        }
    });
}

/// The login runs known for the baseline of origin `origin`: read from its file or recorded in this process.
pub(crate) fn login_records(origin: u64) -> Vec<crate::baseline::LoginRecord> {
    let all = LOGIN_RECORDS.lock().unwrap_or_else(PoisonError::into_inner);
    all.iter()
        .find(|(o, _)| *o == origin)
        .map(|(_, r)| r.clone())
        .unwrap_or_default()
}

fn note_login_record(run: &MemoRun) {
    let record = crate::baseline::LoginRecord {
        company: run.key.company.clone(),
        work_date: run.key.work_date,
        day: run.day,
        globals: run.globals.clone(),
        hits: run.hits,
    };
    with_origin(run.key.baseline, |kept| {
        kept.retain(|r| !(r.company == record.company && r.work_date == record.work_date));
        kept.push(record);
        if kept.len() > MEMO_RUNS {
            kept.remove(0);
        }
    });
}

fn kept_login_record(key: &MemoKey) -> Option<crate::baseline::LoginRecord> {
    let all = LOGIN_RECORDS.lock().unwrap_or_else(PoisonError::into_inner);
    let (_, records) = all.iter().find(|(o, _)| *o == key.baseline)?;
    records
        .iter()
        .find(|r| r.company == key.company && r.work_date == key.work_date)
        .cloned()
}

/// How many recorded runs of the login code an engine keeps (the oldest goes first): one per state of the database a
/// session starts from, and the sessions of a run usually share one.
const MEMO_RUNS: usize = 8;

/// The login code's recorded runs, per engine (`Engine::derived`). Without them, each of a run's threads builds the
/// tables `GetDefaultWorkDate` reads from the test data (every G/L entry) only to find the last posting date.
#[derive(Default)]
pub(crate) struct LoginMemo {
    state: Mutex<MemoState>,
    /// Held while a session looks for a recorded run and records one.
    recording: Mutex<()>,
    /// How often a recorded run was replayed (for tests).
    pub(crate) replays: AtomicU64,
}

/// The recorded runs over one test data.
#[derive(Default)]
struct MemoState {
    backing: Option<(u64, u64)>,
    runs: Vec<MemoRun>,
    /// A run could not be replayed: the sessions run the code and no longer wait for each other.
    gave_up: bool,
}

/// What a run of the login code can depend on: the database (the baseline the session started from, while no table
/// changed since, and who offers the rows of the untouched tables), the company and WorkDate. LogInManagement's
/// instance is a new one, as the session made no use of it before. The run reads no other session state
/// ([`replayable`]).
#[derive(Clone, PartialEq)]
struct MemoKey {
    baseline: u64,
    backing: (u64, u64),
    company: String,
    work_date: i32,
}

#[derive(Clone)]
struct MemoRun {
    key: MemoKey,
    day: i32,
    /// The globals of LogInManagement the run may have changed, and their Date values after it.
    globals: Vec<(usize, i32, bool)>,
    /// The assumption marks the run made.
    hits: u128,
}

/// The key of the session's state, whose LogInManagement instance is new; `None` when a run may be neither recorded
/// nor replayed now: the session notes the code it runs (coverage, the impact trace), a table changed since it started
/// from its baseline (install code, what an earlier codeunit left), it has no baseline, or who offers the rows of its
/// untouched tables is not known.
fn memo_key(session: &mut Session) -> Option<MemoKey> {
    if session.notes_code_run() {
        return None;
    }
    let backing = session.store_mut().backing_identity()?;
    let baseline = session.baseline_origin()?;
    Some(MemoKey {
        baseline,
        backing,
        company: session.access_company().to_string(),
        work_date: session.work_date(),
    })
}

/// The globals of LogInManagement a run may have changed and their values after it, when the run can be replayed: it raised no event, wrote nothing,
/// used no page, report or .NET, read only stored tables, and the routines it called read no session state but
/// WorkDate, change none, are all known, and name no global of another codeunit; the globals they may change hold Dates.
fn replayable(
    engine: &Engine,
    lowered: &Lowered,
    object: usize,
    session: &Session,
    touches: &al_vm::impact::Touches,
    instance: &Rc<RefCell<al_builtins::value::Instance>>,
) -> Option<Vec<(usize, i32, bool)>> {
    use crate::runtime::written::state;
    let quiet = touches.events.is_empty()
        && touches.writes.is_empty()
        && touches.pages.is_empty()
        && touches.reports.is_empty()
        && !touches.dotnet
        && touches.tables.iter().all(|&t| session.table_stamp(t).is_some());
    if !quiet {
        return None;
    }
    let mut routines: Vec<(usize, usize)> = touches.routines.iter().copied().collect();
    routines.sort_unstable();
    let index = engine.derived(|| crate::runtime::written::ReachIndex::new(&engine.events, lowered));
    let facts = crate::runtime::written::routines_facts(&engine.events, lowered, &index, &routines);
    let clean = facts.unknown.is_none()
        && facts.writes.is_empty()
        && !facts.writes_any
        && facts.state_writes == 0
        && facts.state_reads & !state::WORK_DATE == 0
        && facts
            .globals_read
            .iter()
            .chain(&facts.globals_written)
            .all(|&(o, _)| o == object);
    if !clean {
        return None;
    }
    // The globals the routines may change (assign, hand to a `var` parameter, call a method on, clear) are taken as
    // they are after the run, and must all hold Dates, which are plain values; the others kept their new instance's
    // values. A record or another handle among them turns the replay off.
    let inst = instance.borrow();
    let mut written: Vec<usize> = facts.globals_written.iter().map(|&(_, g)| g).collect();
    written.sort_unstable();
    written
        .into_iter()
        .map(|g| match inst.globals.get(g) {
            Some(Value::Date(d, f)) => Some((g, *d, *f)),
            _ => None,
        })
        .collect()
}

/// Runs one install routine; why it did not run to its end, if it did not.
fn run_routine(lowered: &Lowered, session: &mut Session, oi: usize, ri: usize) -> Option<String> {
    if let Some(reason) = crate::runtime::blocked_reason(lowered, oi, ri) {
        return Some(reason);
    }
    let called = if is_install(&lowered.program.objects.heads()[oi]) {
        let inst = session.instance(oi);
        session.call(oi, ri, inst, Vec::new()).map(|_| ())
    } else {
        session.call_parameterless_subscriber(oi, ri)
    };
    match called {
        Ok(()) => None,
        Err(Signal::Unsupported(r)) => Some(r),
        Err(Signal::Error(e)) => Some(format!("failed (behaviour not pinned): {}", e.message)),
        Err(_) => None,
    }
}

/// The tables a routine that could not run touches, with those of the event subscribers it would have set off.
fn touched(engine: &Engine, lowered: &Lowered, oi: usize, ri: usize) -> BTreeSet<u32> {
    let mut t = crate::runtime::tables_touched(lowered, oi, ri);
    for (so, sr) in crate::runtime::subscribers_reached(engine, lowered, oi, ri) {
        t.extend(crate::runtime::tables_touched(lowered, so, sr));
    }
    // AllObj and AllObjWithCaption are listed from the compiled objects alone and cannot be written: nothing an install
    // routine does shows in them, so a test that reads them is not held back.
    t.retain(|&x| !crate::runtime::listed_from_objects(lowered, x));
    t
}
