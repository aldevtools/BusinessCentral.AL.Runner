//! The install code of an app (ADR-046 step A): its `Subtype = Install` codeunits' `OnInstallAppPerCompany` and
//! `OnInstallAppPerDatabase` triggers, and its subscribers to the same two events of `Extension Triggers` (decision 1;
//! events/platform-triggers/PeVFixtures.al:395-407, PeVTests.al:284). [`plan`] lists them per app and part,
//! [`facts`] works out which tables a test may not read because the runtime cannot know their install rows, and
//! [`run_apps_under_test`] runs the apps under test one after the other.
//!
//! What BC does (amendment 1, A1): one app's whole install ends before the next app's begins
//! (record/BfInstallTests.al:239); each app installs in a session of its own, not the test
//! session (`:142-144`, `:168`); per company the install session raises Company Triggers OnCompanyOpen and
//! OnCompanyOpenCompleted (to every installed app's subscribers, see [`company_open`]), then runs the routines (`:125-140`,
//! `company open calls`); SingleInstance state starts again in
//! each company and in the per-database part, and carries between the routines of one company (`counter`); the
//! per-database part runs last, with `CompanyName()` empty (`:138-140`). Inside one app and part the routines run in
//! ascending codeunit id, across the 65536 id boundary (amendment 3, [`order_of`]); only the order of two subscribers of
//! one codeunit is not pinned, and every test that can see it is refused (A2).

use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::lower::Lowered;
use al_vm::{Engine, InstallFacts, Session};
use std::collections::{BTreeMap, BTreeSet};
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

/// The install routines of one app, per part, in ascending object id (ADR-046 amendment 3).
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

fn is_upgrade(o: &al_ir::ObjectHead) -> bool {
    o.is_kind(al_ir::ObjKind::Codeunit)
        && o.properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Upgrade"))
}

/// The tables (object indexes) the install and upgrade code of the apps under test may write: every routine of their
/// Install and Upgrade codeunits, their install subscribers and Company Triggers subscribers, what those call and the
/// subscribers they reach. A subscriber whose install order is not known and that stops unless it finds a row of an empty
/// table of its own app outside this set does nothing in either order (al-vm empty_reads.rs; al-runner issue 6924).
/// `None` when that code holds a RecordRef or a DataTransfer, which may write any table.
pub(crate) fn tables_install_code_may_write(lowered: &Lowered, engine: &Engine) -> Option<BTreeSet<u32>> {
    tables_install_code_of_may_write(lowered, engine, |o| !lowered.dependency[o])
}

/// [`tables_install_code_may_write`] for the install code of the objects `of` accepts: the partner dependency apps whose
/// install order is open in the install baseline (al-runner issue 6972, al-vm order_trial.rs).
pub(crate) fn tables_install_code_of_may_write(
    lowered: &Lowered,
    engine: &Engine,
    of: impl Fn(usize) -> bool + Copy,
) -> Option<BTreeSet<u32>> {
    use al_builtins::library_kind::LibraryKind;
    use al_builtins::value::RtType;
    let objs = &lowered.program.objects;
    let all = install_roots_and_subscribers(lowered, engine, of);
    // Every routine those reach through calls, for the slots that may write any table.
    let mut seen = BTreeSet::new();
    let mut stack: Vec<(usize, usize)> = all.iter().copied().collect();
    while let Some((o, r)) = stack.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        let obj = &objs[o];
        let dynamic = |t: &RtType| matches!(t, RtType::Library(LibraryKind::RecordRef | LibraryKind::DataTransfer));
        if obj
            .routines
            .get(r)
            .is_none_or(|rt| rt.locals.iter().chain(&obj.globals).any(|s| dynamic(&s.ty)))
        {
            return None;
        }
        lowered.calls_into(o, r, &mut stack);
    }
    let mut out = BTreeSet::new();
    for &(o, r) in &all {
        out.extend(crate::baseline::tables_written(lowered, o, r));
    }
    Some(out)
}

/// The Install and Upgrade codeunit routines of the objects `of` accepts, their install and Company Triggers subscribers,
/// and the subscribers those reach.
fn install_roots_and_subscribers(
    lowered: &Lowered,
    engine: &Engine,
    of: impl Fn(usize) -> bool + Copy,
) -> BTreeSet<(usize, usize)> {
    let objs = &lowered.program.objects;
    let mut roots: Vec<(usize, usize)> = Vec::new();
    for (oi, o) in objs.heads().iter().enumerate() {
        if of(oi) && (is_install(o) || is_upgrade(o)) {
            roots.extend((0..objs[oi].routines.len()).map(|ri| (oi, ri)));
        }
    }
    roots.extend(
        engine
            .events
            .install_subscribers
            .iter()
            .filter(|(so, _, _)| of(*so))
            .map(|&(so, sr, _)| (so, sr)),
    );
    roots.extend(company_open_subscribers(lowered, &engine.events, of));
    let mut all: BTreeSet<(usize, usize)> = roots.iter().copied().collect();
    all.extend(crate::runtime::subscribers_reached_from(engine, lowered, &roots));
    all
}

/// Every routine the install code of the objects `of` accepts can run (`install_roots_and_subscribers` and what those call;
/// install_rows.rs).
pub(crate) fn install_code_of(
    lowered: &Lowered,
    engine: &Engine,
    of: impl Fn(usize) -> bool + Copy,
) -> BTreeSet<(usize, usize)> {
    let mut seen = BTreeSet::new();
    let mut stack: Vec<(usize, usize)> = install_roots_and_subscribers(lowered, engine, of).into_iter().collect();
    while let Some((o, r)) = stack.pop() {
        if seen.insert((o, r)) {
            lowered.calls_into(o, r, &mut stack);
        }
    }
    seen
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
    // Amendment 3: ascending codeunit id across the 65536 boundary ([`order_of`]).
    routines.sort_by_key(|&(o, r, _)| (object_id(lowered, o), o, r));
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

/// The object id of object `o` (0 for an object without one).
fn object_id(lowered: &Lowered, o: usize) -> u32 {
    lowered.program.objects.heads()[o].id.unwrap_or(0)
}

/// The highest codeunit id the order probes could publish: the sandbox refused 120000 (corpus PR 810, writer UR,
/// install-upgrade/install-order/UrBlockOrderTests.al:9-11).
const HIGHEST_PROBED_ID: u32 = 99_999;

/// What is known of BC's order of the install routines `a` and `b` of one app and part (ADR-046 amendment 3).
///
/// Install codeunit triggers and Extension Triggers subscribers of one app run as one sequence in ascending codeunit id,
/// in each company and in the per-database part, across the 65536 id boundary (install-upgrade/install-order/
/// UrBlockOrderTests.al:59-81, 50300 50301 50302 before 90511 90512 90515, corpus PR 810, writer UR, branch
/// al-runtime-rs-staging-ur 1ac158e1; inside one block install-upgrade/install-order/UmInstallOrder.al:120-125, corpus PR
/// 807). Both are in the pinned corpus. Ids above 99999 could not be published there: the same integer comparison is an
/// assumption. Two subscribers of one codeunit are not
/// pinned (every pin has one routine per codeunit and event). The earlier contradiction,
/// record/BfInstallTests.al:239, is SystemRowVersion of buffered plain Inserts (UrBlockOrderTests.al:99-115),
/// not the run order.
fn order_of(lowered: &Lowered, a: (usize, usize), b: (usize, usize)) -> Order {
    if a.0 == b.0 {
        return Order::Unknown;
    }
    if object_id(lowered, a.0).max(object_id(lowered, b.0)) > HIGHEST_PROBED_ID {
        Order::Assumed(al_builtins::assumptions::Assumption::InstallOrderIdsAboveProbedRange)
    } else {
        Order::Pinned
    }
}

/// What is known of the order of two install routines.
#[derive(Clone, Copy)]
enum Order {
    /// The corpus pins it.
    Pinned,
    /// It rests on an unprobed assumption.
    Assumed(al_builtins::assumptions::Assumption),
    /// Not pinned: what depends on it is refused.
    Unknown,
}

/// The tables whose install rows a test may not read (ADR-046 amendment 1, A2 items a to d), from the
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
    if !dependency {
        // Decision 5: the tables of the apps under test.
        out.under_test_tables = objs
            .heads()
            .iter()
            .enumerate()
            .filter(|(oi, o)| !lowered.dependency[*oi] && o.table.is_some())
            .map(|(oi, _)| oi)
            .collect();
    }
    let add = |map: &mut std::collections::HashMap<usize, String>, t: u32, why: String| {
        map.entry(t as usize).or_insert(why);
    };
    {
        for app in plan(lowered, events, dependency) {
            let name = app_name(app.app);
            for part in [Part::Company, Part::Database] {
                let routines = app.part(part);
                let closures: Vec<Closure> = routines.iter().map(|&(o, r)| closure(lowered, o, r)).collect();
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
                // Amendment 3: the routines run in ascending codeunit id ([`order_of`]); only two subscribers of one
                // codeunit have no known order. A table the order shows is refused only through such a pair; through
                // any other pair it is a flagged pass.
                let order = |i: usize, j: usize| {
                    format!(
                        "{} and {} of {name} are install subscribers of one codeunit in the {} part, and the order BC runs them in is not pinned (ADR-046 amendment 3)",
                        label(lowered, routines[i].0, routines[i].1),
                        label(lowered, routines[j].0, routines[j].1),
                        part.label()
                    )
                };
                // A2 d, kept by amendment 3: BC gives a row a plain Insert wrote its SystemRowVersion only when it writes
                // its insert buffer, later than rows of routines that ran after it
                // (install-upgrade/install-order/UrBlockOrderTests.al:99-115, the cause of BfInstallTests.al:239); this
                // runtime stamps at the Insert. So the row versions of install rows do not follow the run order.
                let rows = format!(
                    "{name} has {} install routines in the {} part, and BC gives a row a plain Insert wrote its SystemRowVersion only when it writes its insert buffer, which this runtime does not model, so the row versions of their rows are not pinned (ADR-046 amendment 3)",
                    routines.len(),
                    part.label()
                );
                for c in &closures {
                    for &t in &c.written {
                        add(&mut out.row_order, t, rows.clone());
                    }
                }
                for (i, c) in closures.iter().enumerate() {
                    for (j, d) in closures.iter().enumerate() {
                        if i == j {
                            continue;
                        }
                        let known = order_of(lowered, routines[i], routines[j]);
                        let hit = |map: &mut InstallFacts, t: u32, why: String| match known {
                            Order::Pinned => {}
                            Order::Assumed(assumption) => {
                                map.assumed.entry(t as usize).or_insert((app.app, assumption));
                            }
                            Order::Unknown => {
                                map.unknown.entry(t as usize).or_insert(why);
                            }
                        };
                        // A2 a: two routines write one table.
                        for t in c.written.intersection(&d.written) {
                            hit(
                                &mut out,
                                *t,
                                format!("two of its install routines write this table: {}", order(i, j)),
                            );
                        }
                        // A2 b: one reads what another writes; what the reader writes depends on the order.
                        if c.read.intersection(&d.written).next().is_some() {
                            for &t in &c.written {
                                hit(
                                    &mut out,
                                    t,
                                    format!("it reads a table another install routine writes: {}", order(i, j)),
                                );
                            }
                        }
                        // A2 c: a SingleInstance codeunit carries state from one routine to the next.
                        if let Some(&shared) = c.singles.intersection(&d.singles).next() {
                            for &t in c.written.iter().chain(&d.written) {
                                hit(
                                    &mut out,
                                    t,
                                    format!(
                                        "two of its install routines share the SingleInstance codeunit \"{}\": {}",
                                        objs[shared].name,
                                        order(i, j)
                                    ),
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
    let view = InstallView::new(lowered, crate::host::publish_order_known());
    let under_test = apps_under_test(lowered);
    if under_test.len() > 1 {
        let written = tables_install_code_may_write(lowered, &engine);
        // A table only its own app's install code writes, with the rows constants show (al-runner issue 6971).
        if written.is_some() {
            session.set_install_rows(Some(crate::install_rows::install_rows(lowered, &engine)));
        }
        session.set_install_written_tables(written);
    }
    let plans: Vec<AppInstall> = plan(lowered, &engine.events, false);
    // Microsoft's apps under test install before the partner apps under test (ADR-046 amendment 7, `install_orders.rs`).
    let (microsoft, partner) = crate::install_orders::split_microsoft(lowered, &view, &under_test);
    let mut inst = UnderTestInstalls {
        lowered,
        plans: &plans,
        company: &company,
        skipped: &mut skipped,
    };
    for app in plans.iter().filter(|a| microsoft.contains(&(a.app as u32))) {
        if app.company.is_empty() && app.database.is_empty() {
            continue;
        }
        // One app at a time: the apps under test after it are not installed yet (`InstallView`).
        view.apply(session, app.app as u32, &microsoft, &partner);
        crate::install_orders::AppInstalls::install(&mut inst, session, app.app as u32);
    }
    let group = crate::install_orders::PartnerGroup {
        view: &view,
        group: partner.clone(),
        held: BTreeSet::new(),
        apps: plans
            .iter()
            .filter(|a| partner.contains(&(a.app as u32)) && !(a.company.is_empty() && a.database.is_empty()))
            .map(|a| a.app as u32)
            .collect(),
    };
    crate::install_orders::install_partner_apps(lowered, session, &group, &mut inst);
    session.set_install_session_id(None);
    session.set_apps_not_installed(None);
    session.set_apps_install_order_unknown(None);
    session.set_install_written_tables(None);
    session.set_install_rows(None);
    session.set_install_without_client(false);
    // The test session does not see the SingleInstance state of the install run (PeVTests.al:297, fact 6).
    session.reset_single_instances();
    session.commit_transaction();
    skipped
}

/// The apps under test, for `install_orders::install_partner_apps` and the Microsoft apps before them.
struct UnderTestInstalls<'a> {
    lowered: &'a Lowered,
    plans: &'a [AppInstall],
    company: &'a str,
    skipped: &'a mut Vec<(BTreeSet<u32>, String)>,
}

impl crate::install_orders::AppInstalls for UnderTestInstalls<'_> {
    type Mark = usize;

    fn mark(&self) -> usize {
        self.skipped.len()
    }

    fn rewind(&mut self, mark: &usize) {
        self.skipped.truncate(*mark);
    }

    fn install(&mut self, session: &mut Session, app: u32) {
        if let Some(k) = self.plans.iter().position(|a| a.app as u32 == app) {
            install_app_under_test(self.lowered, session, &self.plans[k], k, self.company, self.skipped);
        }
    }

    fn refuse(&mut self, tables: BTreeSet<u32>, why: String) {
        self.skipped.push((tables, why));
    }

    fn note(&mut self, _line: String) {}
}

/// Runs the install code of app under test `app` (`k`: its place in the plan), the session's install view set.
fn install_app_under_test(
    lowered: &Lowered,
    session: &mut Session,
    app: &AppInstall,
    k: usize,
    company: &str,
    skipped: &mut Vec<(BTreeSet<u32>, String)>,
) {
    let engine = session.engine.clone();
    let company = company.to_string();
    // A session of its own (A4 item 1): any id that is positive and not the test session's 1.
    session.set_install_session_id(Some(2 + k as i32));
    for part in [Part::Company, Part::Database] {
        let routines = app.part(part);
        if routines.is_empty() {
            continue;
        }
        // Decision 3, step 4a: the per-company part once in each declared company, in list order (the session
        // company first: record/BfInstallTests.al:125-137, rows #1-#6, and
        // install-upgrade/install-order/AvInstallOrder.al:56-67, `first per-company run`); one company completes
        // before the next starts (BfInstallTests.al:239). Without `--companies`, the session company only.
        let companies: Vec<String> = match part {
            Part::Database => vec![String::new()],
            Part::Company if session.companies().is_empty() => vec![company.clone()],
            Part::Company => session.companies().to_vec(),
        };
        for in_company in companies {
            // A4 item 2: SingleInstance state starts again in each company and in the per-database part.
            session.reset_single_instances();
            session.set_company(&in_company);
            // A4 item 4: Company Triggers in each company, before the routines. Under `--companies` the dependency
            // apps stay in the session company (decision 5): in another company only the subscribers of the apps
            // under test run (decided without owner review, 2026-10-29, ADR-046 amendment 2).
            let with_dependencies = in_company == company;
            let open = if part == Part::Company {
                company_open(lowered, session, with_dependencies)
            } else {
                Ok(())
            };
            match open {
                Err(why) => {
                    session.rollback_transaction();
                    // What the subscribers wrote before the one that could not run is rolled back, and BC would have
                    // kept it: a test that touches what any of them can write is not run either.
                    for (so, sr) in company_open_subscribers(lowered, &engine.events, |o| {
                        installed(lowered, o, app.app) && (with_dependencies || !lowered.dependency[o])
                    }) {
                        refuse(
                            lowered,
                            session,
                            skipped,
                            &in_company,
                            &company,
                            (
                                (so, sr),
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
                            ),
                        );
                    }
                    for &(oi, ri) in routines {
                        refuse(
                            lowered,
                            session,
                            skipped,
                            &in_company,
                            &company,
                            (
                                (oi, ri),
                                format!(
                                    "install routine {} could not run: Company Triggers before it: {why}",
                                    label(lowered, oi, ri)
                                ),
                            ),
                        );
                    }
                }
                Ok(()) => {
                    session.commit_transaction();
                    for &(oi, ri) in routines {
                        if let Some(why) = run_routine(lowered, session, oi, ri) {
                            session.rollback_transaction();
                            refuse(
                                lowered,
                                session,
                                skipped,
                                &in_company,
                                &company,
                                (
                                    (oi, ri),
                                    format!("install routine {} could not run: {why}", label(lowered, oi, ri)),
                                ),
                            );
                        } else {
                            // A routine that ran is durable before the next one starts: a later one that cannot run
                            // must not take its writes with it.
                            al_builtins::native::Ctx::commit(session).expect("commit");
                        }
                    }
                }
            }
        }
        session.set_company(&company);
    }
}

/// Whether the app of object `o` is installed when app `app` (an app under test) installs: a dependency app (the install
/// baseline installs them first), an app under test before it in dependency order, or the app itself.
/// Records that install routine `what.0` could not run in company `in_company` (`what.1` says why). In the session
/// company, or in the per-database part, a test that touches a table the routine touches is not run. In another declared
/// company (`--companies`, ADR-046 decision 6) the tables the routine would have written there are not known: a shared one
/// everywhere (a test that touches it is not run), a per-company one only in that company, refused at run time
/// (`Session::mark_company_unknown`).
fn refuse(
    lowered: &Lowered,
    session: &mut Session,
    skipped: &mut Vec<(BTreeSet<u32>, String)>,
    in_company: &str,
    session_company: &str,
    what: ((usize, usize), String),
) {
    let ((oi, ri), why) = what;
    let engine = session.engine.clone();
    // A routine the compile for a selection left out: the run is redone on the full run's compile, whatever the tests
    // touch (al-runner issue 6924).
    crate::runtime::narrowing::note_install(&engine, &why);
    if in_company.is_empty() || in_company == session_company {
        skipped.push((touched(&engine, lowered, oi, ri), why));
        return;
    }
    let mut written = crate::baseline::tables_written(lowered, oi, ri);
    for (so, sr) in crate::runtime::subscribers_reached(&engine, lowered, oi, ri) {
        written.extend(crate::baseline::tables_written(lowered, so, sr));
    }
    written.retain(|&x| !crate::runtime::listed_from_objects(lowered, x));
    let heads = lowered.program.objects.heads();
    let (per, shared): (BTreeSet<u32>, BTreeSet<u32>) = written
        .into_iter()
        .partition(|&t| heads.get(t as usize).is_some_and(per_company));
    let why = format!("{why} (in company {in_company}, ADR-046 decision 6)");
    for t in per {
        session.mark_company_unknown(in_company, t, why.clone());
    }
    if !shared.is_empty() {
        skipped.push((shared, why));
    }
}

fn installed(lowered: &Lowered, o: usize, app: usize) -> bool {
    lowered.dependency[o] || lowered.program.objects[o].app as usize <= app
}

/// The apps (indexes in `Program::apps`) each app depends on, directly or through other apps.
fn depends_on(lowered: &Lowered) -> Vec<BTreeSet<u32>> {
    let apps = &lowered.program.apps;
    let norm = |s: &str| {
        s.trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .to_ascii_lowercase()
    };
    let direct: Vec<Vec<u32>> = apps
        .iter()
        .map(|a| {
            a.dependencies
                .iter()
                .filter_map(|d| apps.iter().position(|b| norm(&b.id) == norm(d)).map(|i| i as u32))
                .collect()
        })
        .collect();
    (0..apps.len())
        .map(|i| {
            let mut seen = BTreeSet::new();
            let mut stack = direct[i].clone();
            while let Some(d) = stack.pop() {
                if seen.insert(d) {
                    stack.extend(direct[d as usize].iter().copied());
                }
            }
            seen
        })
        .collect()
}

/// What the session sees while app `installing` installs, one of the apps of `group` that install one after the other in
/// dependency order: the apps of `group` that depend on it are not installed yet, nor are the apps of
/// `held`; the apps of `group` with no dependency path to it in either direction have no known order to it. BC installs one
/// app at a time in dependency order and an app's install code sees only the apps installed before it and itself: during
/// XApp A's install no subscriber of XApp B (which depends on A) or of the main app ran and `NavApp.GetModuleInfo` of B
/// was false; during B's install A's and B's subscribers ran, the main app's did not (corpus branch
/// al-runtime-rs-staging-y3: install-upgrade/install-order/InstallOrderSubscriberTests.al:66-79 (A's install), :90-91
/// (GetModuleInfo of B), :104-117 (B's install), :139-144 (test time); fixtures tests/al-language-xapp-a/Y3XaInstallEvents.al,
/// tests/al-language-xapp-b/Y3XbInstallEvents.al). Nothing pins the order of two apps with no
/// dependency path between them, so what one of them could see of the other is refused (`Session::set_apps_install_order_unknown`).
///
/// `listed_order`: the run publishes the apps of the group in the order `Program::apps` lists them
/// (`host::publish_order_known`), so two of them with no dependency path between them install in that order.
pub(crate) struct InstallView {
    deps: Vec<BTreeSet<u32>>,
    listed_order: bool,
}

impl InstallView {
    pub(crate) fn new(lowered: &Lowered, listed_order: bool) -> Self {
        InstallView {
            deps: depends_on(lowered),
            listed_order,
        }
    }

    /// (not installed, order unknown) while `installing` installs, with `fixed` an install order the runner chose for some apps of `group` with no dependency
    /// path between them (ADR-046 amendment 7, `install_orders.rs`): between two apps of `fixed`, the one listed first is
    /// installed while the other installs.
    pub(crate) fn of_ordered(
        &self,
        installing: u32,
        group: &BTreeSet<u32>,
        held: &BTreeSet<u32>,
        fixed: &[u32],
    ) -> (BTreeSet<u32>, BTreeSet<u32>) {
        let mut not_installed = held.clone();
        let mut unknown = BTreeSet::new();
        let at = |a: u32| fixed.iter().position(|&f| f == a);
        for &g in group {
            if g == installing || self.needs(installing, g) {
                continue;
            }
            if self.needs(g, installing) {
                not_installed.insert(g);
            } else if let (Some(i), Some(j)) = (at(installing), at(g)) {
                if j < i {
                    not_installed.remove(&g);
                } else {
                    not_installed.insert(g);
                }
            } else if self.listed_order && g > installing {
                not_installed.insert(g);
            } else if self.listed_order {
                not_installed.remove(&g);
            } else {
                not_installed.remove(&g);
                unknown.insert(g);
            }
        }
        (not_installed, unknown)
    }

    /// Whether the apps of a group install in the order the run lists them (`host::publish_order_known`).
    pub(crate) fn listed(&self) -> bool {
        self.listed_order
    }

    /// Whether app `a` depends on app `b`, directly or through other apps.
    pub(crate) fn needs(&self, a: u32, b: u32) -> bool {
        self.deps.get(a as usize).is_some_and(|d| d.contains(&b))
    }

    /// Sets what the session sees while `installing` installs.
    pub(crate) fn apply(&self, session: &mut Session, installing: u32, group: &BTreeSet<u32>, held: &BTreeSet<u32>) {
        self.apply_ordered(session, installing, group, held, &[]);
    }

    /// [`InstallView::apply`] with an install order chosen for the apps of `fixed` ([`InstallView::of_ordered`]).
    pub(crate) fn apply_ordered(
        &self,
        session: &mut Session,
        installing: u32,
        group: &BTreeSet<u32>,
        held: &BTreeSet<u32>,
        fixed: &[u32],
    ) {
        let (not_installed, unknown) = self.of_ordered(installing, group, held, fixed);
        session.set_apps_not_installed(Some(not_installed));
        session.set_apps_install_order_unknown(Some(unknown));
        session.set_installing_app(Some(installing));
    }
}

/// The app ids of Microsoft's test-toolkit apps: what BcContainerHelper's `Import-TestToolkitToBcContainer` publishes and
/// installs into a container whose application apps are installed already (BcContainerHelper 6.1.12 HelperFunctions.ps1:381-407,
/// `GetTestToolkitApps`: Permissions Mock, Test Runner, Any, Library Assert, Library Variable Storage, System Application Test
/// Library, Business Foundation Test Libraries, Application Test Library, Tests-TestLibraries, AI Test Toolkit). BCApps' CI
/// builds its test containers the same way. The ids are the apps' manifests' (the same in the three supported majors).
/// Owner decision 2026-10-30 (ADR-046 amendment 6): the application apps install before these.
pub(crate) const TEST_TOOLKIT_APP_IDS: &[&str] = &[
    "40860557-a18d-42ad-aecb-22b7dd80dc80", // Permissions Mock
    "23de40a6-dfe8-4f80-80db-d70f83ce8caf", // Test Runner
    "e7320ebb-08b3-4406-b1ec-b4927d3e280b", // Any
    "dd0be2ea-f733-4d65-bb34-a28f4624fb14", // Library Assert
    "5095f467-0a01-4b99-99d1-9ff1237d286f", // Library Variable Storage
    "9856ae4f-d1a7-46ef-89bb-6ef056398228", // System Application Test Library
    "bee8cf2f-494a-42f4-aabd-650e87934d39", // Business Foundation Test Libraries
    "d852d5d2-a39d-4179-baeb-f99a19e32510", // Application Test Library
    "5d86850b-0d76-4eca-bd7b-951ad998e997", // Tests-TestLibraries
    "2156302a-872f-4568-be0b-60968696f0d5", // AI Test Toolkit
];

/// The dependency apps (indexes in `Program::apps`) whose id is in [`TEST_TOOLKIT_APP_IDS`].
pub(crate) fn test_toolkit_apps(lowered: &Lowered) -> BTreeSet<u32> {
    let norm = |s: &str| {
        s.trim()
            .trim_start_matches('{')
            .trim_end_matches('}')
            .to_ascii_lowercase()
    };
    lowered
        .program
        .apps
        .iter()
        .enumerate()
        .filter(|(_, a)| a.dependency && TEST_TOOLKIT_APP_IDS.contains(&norm(&a.id).as_str()))
        .map(|(i, _)| i as u32)
        .collect()
}

/// The apps (indexes in `Program::apps`) that have an object the run treats as under test.
pub(crate) fn apps_under_test(lowered: &Lowered) -> BTreeSet<u32> {
    let objs = &lowered.program.objects;
    (0..objs.len())
        .filter(|&oi| !lowered.dependency[oi])
        .map(|oi| objs[oi].app)
        .collect()
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
/// BC does in each company before the per-company install routines (record/BfInstallTests.al:125-140,
/// subscribers at events/platform-triggers/BfCompanyOpenFixtures.al:51-71). Pinned for the app's own subscribers; the owner
/// decided that every installed app's subscribers get them (ADR-046 amendment 1, A4 item 4, 2026-10-19), an assumption until
/// probe Q3: the tables that relies on mark the tests that touch them (`facts`, `InstallFacts::assumed`). A subscriber of
/// any app that cannot run here stops the app's per-company install routines, and what it and they touch is refused.
///
/// `with_dependencies` false (a company other than the session company under `--companies`): the dependency apps' subscribers
/// do not run there, since the dependency apps stay in the session company (ADR-046 decision 5, amendment 2).
fn company_open(lowered: &Lowered, session: &mut Session, with_dependencies: bool) -> Result<(), String> {
    let engine = session.engine.clone();
    // What the session sees while the app installs (`InstallView::apply_ordered`, set by the caller).
    let unknown = session.apps_install_order_unknown().cloned().unwrap_or_default();
    let not_installed = session.apps_not_installed().cloned().unwrap_or_default();
    let objs = &lowered.program.objects;
    if let Some(&(o, r)) = company_open_subscribers(lowered, &engine.events, |o| {
        !lowered.dependency[o] && unknown.contains(&objs[o].app)
    })
    .first()
    {
        session.note_order_unknown_hit(objs[o].app);
        return Err(format!(
            "Company Triggers subscriber {} is in an app with no dependency path to the app that installs; the install \
             order of the two is not known",
            label(lowered, o, r)
        ));
    }
    let all = company_open_subscribers(lowered, &engine.events, |o| {
        (lowered.dependency[o] || !not_installed.contains(&objs[o].app))
            && (with_dependencies || !lowered.dependency[o])
            && !unknown.contains(&objs[o].app)
    });
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
            Err(Signal::Unsupported(r)) => return Err(r.into_reason()),
            Err(Signal::Error(e)) => return Err(format!("failed (behaviour not pinned): {}", e.message)),
            Err(_) => return Ok(()),
        }
    }
    Ok(())
}

/// The tables the Company Triggers subscribers of the test session wrote ([`test_session_company_open`]).
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct CompanyOpenWrites {
    /// The first subscriber that wrote, for the reason text.
    pub who: Option<String>,
    pub tables: BTreeSet<u32>,
    /// The first subscriber of the login (a dependency app's, `login.rs`) that wrote: its writes stay in the session.
    pub login: Option<String>,
    /// The apps under test's own login subscribers ran where what BC's login does is not known (`login::Login::Own`):
    /// the tables they wrote, and why.
    pub watched_login: Option<(BTreeSet<u32>, &'static str)>,
    /// The tables the apps under test's own login subscribers wrote in the session the pin measured
    /// (`login::Login::Own`): a test that touches one relies on `login-own-registration-reads-alone`.
    pub own_login: BTreeSet<u32>,
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
    /// written tables watched, and [`CompanyOpenWrites::touched`] decides after it ran. The tables of `watched_login` are
    /// only watched: in Microsoft's apps the static reach of nearly every write reaches them (`login.rs`).
    pub fn refuses(&self, facts: &crate::runtime::written::ReachFacts, lowered: &Lowered) -> Option<String> {
        self.who.as_ref()?;
        if facts.unknown.is_some() {
            return None;
        }
        let t = self.tables.intersection(&facts.touched).next()?;
        Some(self.reason(&format!("this test can read table {}", Self::table_name(lowered, *t))))
    }

    /// The tables whose reads and writes a test is watched for: those a subscriber wrote, those of `watched_login`, and
    /// `own_login`.
    pub fn watched(&self) -> BTreeSet<u32> {
        let mut out = self.tables.clone();
        out.extend(self.watched_login.iter().flat_map(|(t, _)| t.iter().copied()));
        out.extend(self.own_login.iter().copied());
        out
    }

    /// After a test that `began`, of the watched tables it touched (`hits`): the one that decides its outcome
    /// ([`CompanyOpenWrites::touched`]), if any. A test that touched a table of `own_login` marks
    /// `login-own-registration-reads-alone`. The other watched tables (an earlier Disabled codeunit's, `run.rs`) are the
    /// caller's to judge on the whole of `hits`: the first table touched can be one of `own_login`.
    pub fn watch_hit(&self, hits: &BTreeSet<u32>, began: bool) -> Option<u32> {
        if !began {
            return None;
        }
        if !self.own_login.is_disjoint(hits) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::LoginOwnRegistrationReadsAlone);
        }
        (hits.iter())
            .find(|t| self.tables.contains(t) || self.watched_login.as_ref().is_some_and(|(w, _)| w.contains(t)))
            .copied()
    }

    /// Why a test that read or wrote table `t` while it ran is not run: `None` when `t` is not watched.
    pub fn touched(&self, t: u32, lowered: &Lowered) -> Option<String> {
        let name = Self::table_name(lowered, t);
        if self.tables.contains(&t) {
            return Some(self.reason(&format!("this test read or wrote table {name} while it ran")));
        }
        let (_, why) = self.watched_login.as_ref().filter(|(tables, _)| tables.contains(&t))?;
        Some(format!(
            "{why}: the login subscribers of the apps under test ran before the first test and wrote table {name}, which this test read or wrote"
        ))
    }
}

/// How the test session of the codeunit models the login ([`test_session_company_open`]; `disabled`: under
/// RequiredTestIsolation = Disabled). The login (`login.rs`, al-runner issue 7343): raised through the dependency apps'
/// OnCompanyOpenCompleted subscribers, in codeunit id order with the apps under test's own, for a test codeunit that can
/// see what the login leaves; else the apps under test's own login subscribers run where the login is raised, watched in
/// a session the pin does not cover (`login::decide`). It reads the program and the test codeunit only, so it is part of
/// the key of the login start (`login_start.rs`).
pub(crate) fn login_decision(lowered: &Lowered, session: &Session, disabled: bool) -> crate::login::Login {
    let watched = if session.is_clientless_session() {
        Some(crate::login::NO_CLIENT)
    } else if disabled {
        Some(crate::login::DISABLED)
    } else {
        None
    };
    session.test_object.map_or(crate::login::Login::None, |t| {
        crate::login::decide(&session.engine, lowered, t, watched)
    })
}

/// The test session raises Company Triggers OnCompanyOpen, then OnCompanyOpenCompleted, once, before the first test of a
/// codeunit, and not again for later tests (ADR-046 amendment 1, A4 item 10; events/platform-triggers/BfCompanyOpenTests.al:19-38,
/// :40-50; events/platform-triggers/PeVTests.al:297). Here the subscribers of the apps under test run. What the runner
/// does with the database writes of a subscriber is not pinned: the tables they wrote while they ran are returned, and a
/// test that can read one is not run (`CompanyOpenWrites::refuses` before it starts, `CompanyOpenWrites::touched` after it
/// ran; al-runner issues 5606, 5641). A subscriber that did not run to its end refuses the codeunit. The subscribers of
/// dependency apps do not run here (A4 item 10 names the apps under test only), except the OnCompanyOpenCompleted ones
/// when the session raises the login; otherwise the apps under test's own login subscribers run among them, watched
/// under RequiredTestIsolation = Disabled (`disabled`) and in a session without a client (`login.rs`, al-runner issues
/// 7343, 7717).
pub(crate) fn test_session_company_open(
    lowered: &Lowered,
    session: &mut Session,
    mut decision: crate::login::Login,
) -> Result<CompanyOpenWrites, String> {
    let engine = session.engine.clone();
    let objs = &lowered.program.objects;
    let mut wrote = CompanyOpenWrites::default();
    // Microsoft's code raises OnRegisterAssistedSetup only while the checklist is not set up (the pin measured a session
    // where it was not, LgnLoginChecklistTests.al:10, :139): when it is, Microsoft's code decides (review of al-runner
    // issue 7717, round three, probe p6c).
    if let crate::login::Login::Own { events, .. } = &decision {
        if events.iter().any(|e| e.event == "onregisterassistedsetup")
            && crate::login::checklist_set_up(lowered, session)
        {
            decision = crate::login::Login::Raise;
        }
    }
    let raise = matches!(decision, crate::login::Login::Raise);
    let (own, watched): (&[crate::login::OwnEvent], _) = match &decision {
        crate::login::Login::Own { events, watched } => (events, *watched),
        crate::login::Login::Refused(why) => return Err(why.clone()),
        _ => (&[], None),
    };
    // The subscribers to run, (object, routine); `None` for the login raised to the apps' own subscribers.
    let mut subs: Vec<Option<(usize, usize)>> = Vec::new();
    for event in ["oncompanyopen", "oncompanyopencompleted"] {
        let mut of_event: Vec<(u32, Option<(usize, usize)>)> = engine
            .events
            .subscribers
            .get(&(al_vm::events::COMPANY_TRIGGERS, event.to_string()))
            .into_iter()
            .flatten()
            .filter(|s| !s.manual && (!lowered.dependency[s.object] || (raise && event == "oncompanyopencompleted")))
            .map(|s| (objs[s.object].id.unwrap_or(0), Some((s.object, s.routine))))
            .collect();
        if event == "oncompanyopencompleted" && !own.is_empty() {
            of_event.push((crate::login::LOGIN_RAISER, None));
        }
        of_event.sort_by_key(|&(id, _)| id);
        subs.extend(of_event.into_iter().map(|(_, s)| s));
    }
    let mut own_wrote = BTreeSet::new();
    for step in subs {
        let Some((o, r)) = step else {
            own_wrote = own_login(lowered, session, own, watched)?;
            continue;
        };
        let name = label(lowered, o, r);
        let who = format!("Company Triggers OnCompanyOpen(Completed) subscriber {name}");
        if let Some(reason) = crate::runtime::blocked_reason(lowered, o, r) {
            return Err(format!("{who} before the first test: {reason}"));
        }
        // The subscriber runs here, so what it wrote is what it wrote while it ran, not what its static reach could write:
        // code it did not run (a routine refused at lowering, a run by an id known only at run time, behind a branch that
        // is not taken) wrote nothing (al-runner issue 5641). Code it ran that is not supported raises `Unsupported`.
        let since = session.store_mut().change_counter();
        match session.call_parameterless_subscriber(o, r) {
            Ok(()) => {}
            Err(Signal::Unsupported(why)) => return Err(format!("{who} before the first test: {why}")),
            Err(Signal::Error(e)) => {
                return Err(format!(
                    "{who} failed before the first test (behaviour not pinned): {}",
                    e.message
                ));
            }
            Err(_) => return Err(format!("{who} stopped before the first test (not pinned)")),
        }
        let store = session.store_mut();
        // A change to a computed (platform) table is not a stored row the watch can follow.
        if store.virtual_write_pending() {
            return Err(format!(
                "{who} changed a platform table in the test session before the first test: what the test runner does with the change is not pinned (ADR-046 amendment 1, A4 item 10)"
            ));
        }
        let changed = store.tables_changed_since(since);
        if changed.is_empty() {
            continue;
        }
        // What the login wrote is in the session for every test of the codeunit (LgnLoginChecklistTests.al:137-138,
        // :161): it is not refused, only noted for the carry rule of Disabled isolation (`run.rs`).
        if lowered.dependency[o] {
            wrote.login.get_or_insert(name);
        } else {
            wrote.tables.extend(changed);
            wrote.who.get_or_insert(name);
        }
    }
    match watched {
        Some(why) => wrote.watched_login = Some((own_wrote, why)),
        None => wrote.own_login = own_wrote,
    }
    Ok(wrote)
}

/// The login events raised to the apps under test's own subscribers only (`login::Login::Own`), through the event code
/// (`Session::raise_to`), so `OnAfterLogin` stays an isolated event (codeunits/TestIsolatedEventSubscriberError.al). What
/// they wrote is the login's, there for every test of the codeunit (LgnLoginChecklistTests.al:137-138, :161); the tables
/// are answered. Where what BC's login does is not known (`watched`), a change of the session's state other than rows
/// refuses the codeunit.
fn own_login(
    lowered: &Lowered,
    session: &mut Session,
    events: &[crate::login::OwnEvent],
    watched: Option<&'static str>,
) -> Result<BTreeSet<u32>, String> {
    let before = watched.map(|_| session.state_stamp());
    let since = session.store_mut().change_counter();
    for e in events {
        for &(o, r) in &e.subscribers {
            if let Some(reason) = crate::runtime::blocked_reason(lowered, o, r) {
                return Err(format!(
                    "login subscriber of an app under test {} before the first test: {reason}",
                    label(lowered, o, r)
                ));
            }
        }
        let (po, pr) = e.publisher;
        let what = format!(
            "{} raised to the login subscribers of the apps under test",
            lowered.program.objects[po].routines[pr].name
        );
        match session.raise_to(po, pr, &e.subscribers) {
            Ok(()) => {}
            Err(Signal::Unsupported(why)) => return Err(format!("{what} before the first test: {why}")),
            Err(Signal::Error(err)) => {
                return Err(format!(
                    "{what} failed before the first test (behaviour not pinned): {}",
                    err.message
                ));
            }
            Err(_) => return Err(format!("{what} stopped before the first test (not pinned)")),
        }
        // On BC the registration events are raised inside Base Application's "Checklist Events" subscriber of the
        // isolated OnAfterLogin, whose own transaction commits before the app's OnAfterLogin subscribers run; an isolated
        // event isolates a subscriber only when nothing is pending (codeunits/TestIsolatedEventSubscriberError.al:5-11,
        // :59-72, :74-88).
        if e.event != "onafterlogin" {
            session.commit_transaction();
        }
    }
    let store = session.store_mut();
    if store.virtual_write_pending() {
        return Err("the login subscribers of the apps under test changed a platform table in the test session before the first test: what the test runner does with the change is not pinned (ADR-046 amendment 1, A4 item 10)".into());
    }
    let wrote = store.tables_changed_since(since);
    if let (Some(why), Some(b)) = (watched, &before) {
        if let Some(what) = b.changed(&session.state_stamp()) {
            return Err(format!(
                "{why}: the login subscribers of the apps under test ran before the first test and changed {what}"
            ));
        }
    }
    Ok(wrote)
}

/// What the login code does to WorkDate when the test session opens the company: Base Application's `LogInManagement`
/// (codeunit 40) sets it to `GetDefaultWorkDate()`, the date of the last G/L entry in a demo company, else the WorkDate it
/// had. Pinned: WorkDate in the test codeunit's OnRun and in its first test is that date in a demo company with G/L
/// entries (test-framework/RlWorkDateAtStart.al:47-52, al-runner issue 5743); install code still sees today
/// (errors/RlInstallUi.al:212, :240). The rest of the login code (language, user personalization,
/// application areas) does not run here, as before. Without the codeunit, or in a session without a client, nothing changes.
pub(crate) fn login_work_date(engine: &Engine, lowered: &Lowered, session: &mut Session) -> Result<(), String> {
    // `--no-client`: the login code is skipped as in every session without a client, but that the session answers
    // GuiAllowed with false is assumed (`no-client-session`): the session marks its WorkDate reads when there is login
    // code that a session with a client would have run.
    let clientless = session.is_clientless_session();
    if !clientless && !session.has_interactive_client() {
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
    if clientless {
        session.note_clientless_login_skipped();
        return Ok(());
    }
    let why =
        |what: String| format!("the login code that sets WorkDate when the test session opens the company: {what}");
    if let Some(reason) = crate::runtime::blocked_reason(lowered, object, routine) {
        return Err(why(reason));
    }
    // Recorded and replayed once per engine (ADR-073 phase 1, `login_start.rs`). A run kept in the baseline's file
    // ([`keep_login_records`]) is not taken by a session that notes the code it runs.
    let may_take_record = !session.notes_code_run();
    let company = session.access_company().to_string();
    let work_date = session.work_date();
    let origin = session.baseline_origin();
    crate::login_start::work_date_step(
        engine,
        session,
        |session| work_date_code(session, object, routine, may_take_record, why),
        |session, run| {
            let Some(baseline) = origin else { return };
            let Some((day, globals)) = replayable(engine, lowered, object, routine, session, run) else {
                return;
            };
            let record = crate::baseline::LoginRecord {
                company,
                work_date,
                day,
                globals,
                hits: run.hits(),
            };
            note_login_record(baseline, record);
        },
    )
}

/// `GetDefaultWorkDate` on the session's LogInManagement, a SingleInstance codeunit whose one instance keeps what the
/// call set: from a run kept in the baseline's file when one fits (LogInManagement is new, no table changed since the
/// baseline, [`keep_login_records`]), else the code.
fn work_date_code(
    session: &mut Session,
    object: usize,
    routine: usize,
    may_take_record: bool,
    why: impl Fn(String) -> String,
) -> Result<(), String> {
    let fresh = !session.has_single_instance(object);
    let instance = session.codeunit_instance(object);
    if let Some(record) = (fresh && may_take_record).then(|| kept_login_record(session)).flatten() {
        {
            let mut inst = instance.borrow_mut();
            for &(g, day, flag) in &record.globals {
                inst.globals[g] = Value::Date(day, flag);
            }
        }
        al_builtins::assumptions::add_hit_bits(record.hits);
        session.set_work_date(record.day);
        return Ok(());
    }
    let called = session.call(object, routine, instance, Vec::new());
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
        Err(Signal::Unsupported(r)) => Err(why(r.into_reason())),
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

/// Keeps a recorded run of the login code that sets WorkDate for the file of the baseline of origin `baseline`, in
/// place of one recorded with the same company and WorkDate.
fn note_login_record(baseline: u64, record: crate::baseline::LoginRecord) {
    with_origin(baseline, |kept| {
        kept.retain(|r| !(r.company == record.company && r.work_date == record.work_date));
        kept.push(record);
        if kept.len() > KEPT_RECORDS {
            kept.remove(0);
        }
    });
}

/// The run kept in the file of the session's baseline for its company and WorkDate: `None` when a table changed since
/// the baseline, the session has none, or who offers the rows of its untouched tables is not known.
fn kept_login_record(session: &mut Session) -> Option<crate::baseline::LoginRecord> {
    session.store_mut().backing_identity()?;
    let baseline = session.baseline_origin()?;
    let (company, work_date) = (session.access_company(), session.work_date());
    let all = LOGIN_RECORDS.lock().unwrap_or_else(PoisonError::into_inner);
    let (_, records) = all.iter().find(|(o, _)| *o == baseline)?;
    records
        .iter()
        .find(|r| r.company == company && r.work_date == work_date)
        .cloned()
}

/// How many runs of the login code the file of one baseline keeps (the oldest goes first).
const KEPT_RECORDS: usize = 8;

/// What a recorded run of the login code that sets WorkDate keeps in the file of its baseline (the WorkDate it set
/// and the Date globals of LogInManagement it may have changed), when another process may take it without comparing
/// the session image: the code ran (not a kept run), wrote no row, raised no event, used no page, report or .NET, read
/// only stored tables, and the routines it called read no session state but WorkDate, change none, are all known, and
/// name no global of another codeunit; the globals they may change hold Dates.
fn replayable(
    engine: &Engine,
    lowered: &Lowered,
    object: usize,
    routine: usize,
    session: &Session,
    run: &al_vm::RecordedRun,
) -> Option<(i32, Vec<(usize, i32, bool)>)> {
    use crate::runtime::written::state;
    let touches = run.touches();
    let quiet = touches.routines.contains(&(object, routine))
        && !run.wrote_rows()
        && touches.events.is_empty()
        && touches.writes.is_empty()
        && touches.pages.is_empty()
        && touches.reports.is_empty()
        && !touches.dotnet
        && touches.tables.iter().all(|&t| session.table_stamp(t).is_some());
    if !quiet {
        return None;
    }
    let day = run.work_date_after()?;
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
    // values. A record or another handle among them keeps the run out of the file.
    let values = run.single_globals(object)?;
    let mut written: Vec<usize> = facts.globals_written.iter().map(|&(_, g)| g).collect();
    written.sort_unstable();
    let globals = written
        .into_iter()
        .map(|g| match values.iter().find(|(slot, _)| *slot == g) {
            Some((_, Some(Value::Date(d, f)))) => Some((g, *d, *f)),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    Some((day, globals))
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
        Err(Signal::Unsupported(r)) => Some(r.into_reason()),
        Err(Signal::Error(e)) => Some(format!("failed (behaviour not pinned): {}", e.message)),
        Err(_) => None,
    }
}

/// The tables a routine that could not run touches, with those of the event subscribers it would have set off.
fn touched(engine: &Engine, lowered: &Lowered, oi: usize, ri: usize) -> BTreeSet<u32> {
    let mut t = crate::runtime::tables_touched(lowered, oi, ri);
    // The tables it writes include those of a `NavApp.LoadPackageData` with a constant table id, which names no record
    // variable (al-runner issue 7359: a test that reads the rows the refused load would have put there).
    t.extend(crate::baseline::tables_written_or_loaded(lowered, oi, ri));
    for (so, sr) in crate::runtime::subscribers_reached(engine, lowered, oi, ri) {
        t.extend(crate::runtime::tables_touched(lowered, so, sr));
    }
    // AllObj and AllObjWithCaption are listed from the compiled objects alone and cannot be written: nothing an install
    // routine does shows in them, so a test that reads them is not held back.
    t.retain(|&x| !crate::runtime::listed_from_objects(lowered, x));
    t
}
