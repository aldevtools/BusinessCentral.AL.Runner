//! The login of the test session (al-runner issue 7343). System Application's "System Initialization Impl." subscribes to
//! Company Triggers `OnCompanyOpenCompleted` and raises "System Initialization".`OnAfterLogin` there; Base Application's
//! "Checklist Events" subscribes to that for a Web, Windows or Desktop client and, while the checklist is not set up,
//! calls `Checklist.InitializeGuidedExperienceItems()`, which raises "Guided Experience".`OnRegisterAssistedSetup`.
//!
//! Pinned (corpus master events/platform-triggers/LgnLoginChecklistTests.al, corpus PR 1231, measured on BC 27.5, 28.5
//! and 29.0): the test session is a Web client with GuiAllowed (:135-136, :144-145); before the first test it raised
//! `OnRegisterAssistedSetup`, then `OnAfterLogin`, once each, in the test session (:131-134, :140-143); the row the app's
//! `OnRegisterAssistedSetup` subscriber inserted is there in the first test and still there in the second, without the
//! test registering it (:137-138, :146-147, :161, :164), the checklist counts as set up (:139, :148), and the second test
//! sees no further raise (:159-163).
//!
//! The runtime models the login in one of two ways ([`decide`]), before the first test, and what it writes stays in the
//! session, committed:
//! - **Raise**: the dependency apps' Company Triggers `OnCompanyOpenCompleted` subscribers run with the apps under test's
//!   own (`install::test_session_company_open`): the real code of Microsoft's apps, which raises both events. Microsoft's
//!   code that cannot run here refuses the codeunit. Used for a test codeunit that can see what an app under test's login
//!   subscriber leaves; when those subscribers read what the dependency apps' login code writes ([`reads_microsofts_login`]:
//!   the checklist setup, `SystemInitialization.IsInProgress()`, a Guided Experience row other than through a call keyed
//!   on the app's own object); when an app under test subscribes to `OnRegisterManualSetup` or
//!   `OnRegisterGuidedExperienceItem`, whose raise at login no corpus test pins ([`RAISE_ONLY`]); and when the checklist is
//!   already set up, where Microsoft's code does not raise `OnRegisterAssistedSetup` (the pin's header, :10, does not
//!   measure that case).
//! - **Own**: otherwise, in the session the pin measured (a Web client with GuiAllowed), the two events are raised to the
//!   apps under test's own subscribers only, in the order the login raises them (:140), where "System Initialization
//!   Impl." would raise the login ([`LOGIN_RAISER`]). The event code runs them, so `OnAfterLogin` stays an isolated event:
//!   a subscriber's error is rolled back and the session goes on (codeunits/TestIsolatedEventSubscriberError.al), as on
//!   the Raise path; an error in an `OnRegisterAssistedSetup` subscriber (not isolated) refuses the codeunit.
//!   - **Watched**: in a session the pin does not cover, the same subscribers run, and what BC does there is not known: a
//!     session without a client (`--no-client`), where which login events BC raises is not pinned, and
//!     RequiredTestIsolation = Disabled, where how the login's writes meet what an earlier codeunit left is not pinned (the
//!     reason `run.rs` does not run such a codeunit whose session raised the login). A test that reads or writes a table
//!     they wrote, through any path, is not run (the store's watch, `install::CompanyOpenWrites`); when they changed the
//!     session language, WorkDate, a number sequence or isolated storage, the codeunit is not run, and so it is when their
//!     static reach binds a manual subscriber, reseeds `Random`, or writes a codeunit global the test codeunit reads.
//!
//! What is still not modelled, in one list:
//! - What the dependency apps' login code adds around the app's subscribers (Microsoft's and any other dependency app's
//!   subscribers to the events, the checklist setup) is there only on the Raise path. [`decide`] raises the login for a
//!   test codeunit whose static reach can see the tables or the state the app's subscribers leave; another one that
//!   reads what only the dependency apps' login code leaves runs without it.
//! - Own: the subscribers leave alone what they leave inside the login. Registered as the assumption
//!   `login-own-registration-reads-alone`, marked when a test touches a table they wrote.
//! - Own: whether the checklist is set up is asked before the app's own Company Triggers subscribers run; one that marks
//!   it done there would change what Microsoft's code does at the login.
//! - Own: a subscriber of an app under test that was not compiled is not refused by the raise (`Session::raise_to`
//!   leaves out the not-compiled check, whose entries do not say their app).
//! - Own: an application area a subscriber sets is cleared when the first test starts, as for every test (not pinned
//!   either way).

use crate::runtime::written::{
    JoinedReach, ReachFacts, ReachIndex, global_triggers_reach_facts, login_subscriber_reach_facts,
    login_subscribers_reach_without, state, union_call_reach_facts, union_reach_facts_without_database_events,
};
use al_sema::lower::Lowered;
use al_vm::Engine;
use al_vm::events::Registry;
use std::collections::BTreeSet;
use std::sync::OnceLock;

/// How the test session of a test codeunit models the login ([`decide`]).
pub(crate) enum Login {
    /// No app under test subscribes to a login event: nothing to model.
    None,
    /// The login is raised before the first test, through Microsoft's code.
    Raise,
    /// The login events are raised to the apps under test's own subscribers only, before the first test, in this order:
    /// (publisher object, publisher routine, the subscribers (object, routine)). `watched`: why what BC does in this
    /// session is not known ([`NO_CLIENT`], [`DISABLED`]); a test that touches a table they wrote is then not run.
    Own {
        events: Vec<OwnEvent>,
        watched: Option<&'static str>,
    },
    /// The login is watched in this session and the test codeunit can see what the subscribers leave besides rows: the
    /// codeunit is not run.
    Refused(String),
}

/// One login event raised to the apps under test's own subscribers ([`Login::Own`]).
#[derive(Clone)]
pub(crate) struct OwnEvent {
    /// The publisher routine (object, routine).
    pub publisher: (usize, usize),
    /// Lower-case event name.
    pub event: &'static str,
    /// The subscribers in apps under test (object, routine), in the order the session runs them.
    pub subscribers: Vec<(usize, usize)>,
}

/// The publishers of the login events an app under test can subscribe to: (codeunit id, name, event in lower case), in
/// the order the login raises them: "Guided Experience".`OnRegisterAssistedSetup` before "System Initialization".
/// `OnAfterLogin` (LgnLoginChecklistTests.al:140, the expected entries).
/// `OnRegisterManualSetup` and `OnRegisterGuidedExperienceItem` follow the first in Microsoft's
/// `Checklist.InitializeGuidedExperienceItems`; the pin covers only the first ([`RAISE_ONLY`]).
const LOGIN_EVENTS: &[(u32, &str, &str)] = &[
    (1990, "Guided Experience", "onregisterassistedsetup"),
    (1990, "Guided Experience", "onregistermanualsetup"),
    (1990, "Guided Experience", "onregisterguidedexperienceitem"),
    (150, "System Initialization", "onafterlogin"),
];

/// The login events whose raise at login no corpus test pins (review of al-runner issue 7717, round four, probes p12,
/// p12b): a subscriber of an app under test to one of them gets the login raised through Microsoft's code, which
/// decides. A probe next to LgnLoginChecklistTests.al can let Own raise them (docs/probes-needed.md).
const RAISE_ONLY: &[&str] = &["onregistermanualsetup", "onregisterguidedexperienceitem"];

/// Microsoft's facade codeunits whose procedures read the rows of the object they are given
/// ([`reads_microsofts_login`]): "Guided Experience" and "Checklist" (System Application).
const KEYED_FACADES: &[(u32, &str)] = &[(1990, "Guided Experience"), (1992, "Checklist")];

/// The id of System Application's "System Initialization Impl.", whose Company Triggers `OnCompanyOpenCompleted`
/// subscriber `InitCompany` raises the login: the apps under test's own login subscribers run where it runs, among the
/// `OnCompanyOpenCompleted` subscribers in codeunit id order, the order the Raise path runs them in
/// (`install::test_session_company_open`).
pub(crate) const LOGIN_RAISER: u32 = 151;

/// Why what the login does in a session without a client is not known ([`Login::Own`]).
pub(crate) const NO_CLIENT: &str = "the login was not raised for this test codeunit, and which login events BC raises in a session without a client (--no-client) is not pinned";

/// Why what the login does under RequiredTestIsolation = Disabled is not known ([`Login::Own`]).
pub(crate) const DISABLED: &str = "the login was not raised for this test codeunit, and under RequiredTestIsolation = Disabled how what the login writes meets what an earlier codeunit left is not pinned";

/// The login events the apps under test subscribe to ([`own_events`]), their subscribers, and the reach of the
/// subscribers of the `Global Triggers` database events without them, which the walk of every test codeunit that writes
/// shares ([`JoinedReach`]). Made once per engine.
struct LoginSubscribers {
    own_events: Vec<OwnEvent>,
    subs: Vec<(usize, usize)>,
    database_events: OnceLock<ReachFacts>,
}

/// What the apps under test's login subscribers leave: the facts of their reach ([`login_subscriber_reach_facts`]), and
/// the codeunits of the apps under test whose state they keep; and whether they read what the dependency apps' login code writes
/// before them. Made once per engine.
#[derive(Default)]
struct LoginLeaves {
    facts: ReachFacts,
    state: BTreeSet<usize>,
    /// Why the subscribers read what the dependency apps' login code writes before them ([`reads_microsofts_login`]).
    reads_microsoft: Option<String>,
}

/// The login events an app under test subscribes to automatically, in the order the login raises them, each with its
/// publisher routine and its subscribers in apps under test in the order the session runs them.
fn own_events(lowered: &Lowered, events: &Registry) -> Vec<OwnEvent> {
    let objects = &lowered.program.objects;
    let mut out = Vec::new();
    for &(id, name, event) in LOGIN_EVENTS {
        for (i, o) in objects.heads().iter().enumerate() {
            if !(o.is_kind(al_ir::ObjKind::Codeunit)
                && lowered.dependency.get(i).copied().unwrap_or(false)
                && o.id == Some(id)
                && o.name.eq_ignore_ascii_case(name))
            {
                continue;
            }
            let Some(&(_, r)) = events
                .publishers
                .iter()
                .find(|((po, _), p)| *po == i && p.event == event)
                .map(|(k, _)| k)
            else {
                continue;
            };
            let subs: Vec<(usize, usize)> = events
                .subscribers
                .get(&(i, event.to_string()))
                .into_iter()
                .flatten()
                .filter(|s| !s.manual && !lowered.dependency[s.object])
                .map(|s| (s.object, s.routine))
                .collect();
            if !subs.is_empty() {
                out.push(OwnEvent {
                    publisher: (i, r),
                    event,
                    subscribers: subs,
                });
            }
        }
    }
    out
}

/// Why the apps under test's login subscribers `subs` read what the dependency apps' login code writes before they run,
/// so that alone they would see other state than inside the login (review of al-runner issue 7717, rounds three and
/// four: alone, a subscriber that asks `Checklist.ShouldInitializeChecklist`, `SystemInitialization.IsInProgress()` or
/// whether Microsoft's own assisted setup row exists gets other answers than inside the login). The dependency apps'
/// login code is what [`LOGIN_RAISER`]'s Company Triggers subscriber runs and their subscribers to the login events,
/// through their own code ([`login_subscriber_reach_facts`]). Every table the subscribers touch counts, also one they
/// write themselves, except through a call to a procedure of a [`KEYED_FACADES`] codeunit whose object id argument is a
/// constant naming an object of an app under test: that call reads only the rows of that object (System Application's
/// Guided Experience reads filter on the object's type and id or on the code built from it), and the dependency apps'
/// login code registers none of the apps' objects. That those calls read alone what they read inside the login is the
/// registered assumption `login-own-registration-reads-alone`.
fn reads_microsofts_login(
    events: &Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    subs: &[(usize, usize)],
) -> Option<String> {
    let objects = &lowered.program.objects;
    let dependency = |i: usize| lowered.dependency.get(i).copied().unwrap_or(false);
    let mut ms = ReachFacts::default();
    let raiser = (objects.heads().iter().enumerate())
        .filter(|(i, o)| dependency(*i) && o.id == Some(LOGIN_RAISER))
        .map(|(i, _)| i);
    let roots: Vec<(usize, usize)> = raiser
        .flat_map(|i| {
            events
                .subscribers
                .get(&(al_vm::events::COMPANY_TRIGGERS, "oncompanyopencompleted".to_string()))
                .into_iter()
                .flatten()
                .filter(move |s| s.object == i)
                .map(|s| (s.object, s.routine))
        })
        .chain(own_events_of_microsoft(lowered, events))
        .collect();
    for (o, r) in roots {
        let f = login_subscriber_reach_facts(events, lowered, index, o, r);
        ms.writes.extend(f.writes);
        ms.globals_written.extend(f.globals_written);
    }
    let facades: Vec<usize> = (0..objects.len())
        .filter(|&i| {
            dependency(i)
                && objects[i].is_kind(al_ir::ObjKind::Codeunit)
                && KEYED_FACADES
                    .iter()
                    .any(|&(id, name)| objects[i].id == Some(id) && objects[i].name.eq_ignore_ascii_case(name))
        })
        .collect();
    let (mut own, visited) = login_subscribers_reach_without(events, lowered, index, subs, &facades);
    for (o, r) in facade_calls(lowered, &visited, &facades) {
        let f = login_subscriber_reach_facts(events, lowered, index, o, r);
        own.touched.extend(f.touched);
        own.globals_read.extend(f.globals_read);
    }
    let name = |t: u32| objects.get(t as usize).map_or("?", |o| o.name.as_str()).to_string();
    if let Some(t) = own.touched.iter().find(|t| ms.writes.contains(t)) {
        return Some(format!(
            "they touch table {}, which the dependency apps' login code writes",
            name(*t)
        ));
    }
    let g = (own.globals_read.iter()).find(|g| ms.globals_written.contains(g) && single_instance(lowered, g.0))?;
    Some(format!(
        "they read a global of SingleInstance codeunit {} that the dependency apps' login code sets",
        objects.get(g.0).map_or("?", |o| o.name.as_str())
    ))
}

/// The procedures of the codeunits `facades` that the routines `visited` call other than with a constant object id
/// naming an object of an app under test ([`reads_microsofts_login`]).
fn facade_calls(lowered: &Lowered, visited: &[(usize, usize)], facades: &[usize]) -> BTreeSet<(usize, usize)> {
    let objects = &lowered.program.objects;
    let mut out = BTreeSet::new();
    for &(o, r) in visited {
        let Some(routine) = objects.get(o).and_then(|x| x.routines.get(r)) else {
            continue;
        };
        al_ir::types::for_each_expr_post(routine.body.stmts(), &mut |e| {
            if let al_ir::Expr::Call {
                object, routine, args, ..
            } = e
            {
                if facades.contains(object) && !keyed_on_own_object(lowered, *object, *routine, args) {
                    out.insert((*object, *routine));
                }
            }
        });
    }
    out
}

/// Whether a call of routine `routine` of `object` passes, for a parameter whose name holds "ObjectID", a constant
/// naming an object of an app under test. Not `Run`: it runs that object (System Application's
/// `GuidedExperienceImpl.RunObject`), whose reach the walk does not follow, so Raise decides (review of al-runner issue
/// 7717, round five, LOW 1).
fn keyed_on_own_object(lowered: &Lowered, object: usize, routine: usize, args: &[al_ir::Arg]) -> bool {
    let objects = &lowered.program.objects;
    let callee = &objects[object].routines[routine];
    if callee.name.eq_ignore_ascii_case("Run") {
        return false;
    }
    let Some(at) = (callee.locals[..callee.params].iter()).position(|p| p.name.to_lowercase().contains("objectid"))
    else {
        return false;
    };
    let mut arg = match args.get(at) {
        Some(al_ir::Arg::Value(e)) => e,
        _ => return false,
    };
    while let al_ir::Expr::Convert(_, inner) = arg {
        arg = inner;
    }
    let al_ir::Expr::Const(al_ir::Const::Integer(id)) = arg else {
        return false;
    };
    (0..objects.len()).any(|i| {
        !lowered.dependency.get(i).copied().unwrap_or(true) && objects[i].id.is_some_and(|x| i64::from(x) == *id as i64)
    })
}

/// Whether object `o` is a SingleInstance codeunit: only its globals outlive the call that set them.
fn single_instance(lowered: &Lowered, o: usize) -> bool {
    let obj = &lowered.program.objects[o];
    obj.is_kind(al_ir::ObjKind::Codeunit)
        && (obj.properties.iter())
            .any(|(k, v)| k.eq_ignore_ascii_case("SingleInstance") && v.trim().eq_ignore_ascii_case("true"))
}

/// The automatic subscribers of the dependency apps (Microsoft's and any other) to the login events (object, routine).
fn own_events_of_microsoft(lowered: &Lowered, events: &Registry) -> Vec<(usize, usize)> {
    let objects = &lowered.program.objects;
    let mut out = Vec::new();
    for &(id, name, event) in LOGIN_EVENTS {
        for (i, o) in objects.heads().iter().enumerate() {
            if o.is_kind(al_ir::ObjKind::Codeunit)
                && lowered.dependency.get(i).copied().unwrap_or(false)
                && o.id == Some(id)
                && o.name.eq_ignore_ascii_case(name)
            {
                let subs = events.subscribers.get(&(i, event.to_string())).into_iter().flatten();
                out.extend(
                    subs.filter(|s| !s.manual && lowered.dependency[s.object])
                        .map(|s| (s.object, s.routine)),
                );
            }
        }
    }
    out
}

/// How the test session of test codeunit `test` models the login ([`Login`]); `watched` is why what BC does in this
/// session is not known ([`NO_CLIENT`], [`DISABLED`]), `None` in the session the pin measured.
///
/// The login is raised when an app under test has an automatic subscriber to "System Initialization".`OnAfterLogin` or
/// "Guided Experience".`OnRegisterAssistedSetup`, and the test codeunit's reach can see what those subscribers leave: it
/// touches a table they write through their own code ([`login_subscriber_reach_facts`]) and does not write itself (in
/// its whole reach, or in its reach without the subscribers of the database events), or touches through its own calls a
/// table they add rows to and that its reach cannot insert into, or reads a global of the subscriber's codeunit or of an
/// app under test's codeunit the subscriber's codeunit holds in a variable (the state of a SingleInstance codeunit,
/// LgnLoginChecklistTests.al:19-64). The subscribers of the events raised inside the login subscribers' calls in
/// Microsoft's apps and the database events do not count: in Microsoft's apps that fan-out writes most of Base
/// Application (Guided Experience's telemetry reaches every telemetry logger subscriber), and a test codeunit that only
/// reads what it writes is not one the login is modelled for (corpus Y64InstallSubscribers.al reads a table a telemetry
/// logger subscriber of a cross-app fixture writes, guarded so that the login writes nothing). Every test codeunit gets
/// the login raised when the subscribers read what the dependency apps' login code writes before them ([`reads_microsofts_login`],
/// once per engine). Another test codeunit gets the apps' own subscribers (Own); in a watched session it is refused when
/// it can see what they leave besides rows ([`watched_state`]).
pub(crate) fn decide(engine: &Engine, lowered: &Lowered, test: usize, watched: Option<&'static str>) -> Login {
    let events = &engine.events;
    let login = engine.derived(|| {
        let own_events = own_events(lowered, events);
        let subs = own_events.iter().flat_map(|e| e.subscribers.iter().copied()).collect();
        LoginSubscribers {
            own_events,
            subs,
            database_events: OnceLock::new(),
        }
    });
    let subs = &login.subs;
    if subs.is_empty() {
        return Login::None;
    }
    let index = engine.derived(|| ReachIndex::new(events, lowered));
    let left = engine.derived(|| {
        let mut left = LoginLeaves::default();
        for &(o, r) in subs {
            let facts = login_subscriber_reach_facts(events, lowered, &index, o, r);
            let f = &mut left.facts;
            f.touched.extend(facts.touched);
            f.writes.extend(facts.writes);
            f.inserts.extend(facts.inserts);
            f.globals_read.extend(facts.globals_read);
            f.globals_written.extend(facts.globals_written);
            f.state_writes |= facts.state_writes;
            // The codeunits whose state the subscriber keeps: its own, and those its codeunit holds in a variable (the
            // SingleInstance codeunit of LgnLoginChecklistTests.al:87-89), of the apps under test.
            left.state.insert(o);
            left.state.extend(held_codeunits(lowered, o));
        }
        left.reads_microsoft = reads_microsofts_login(events, lowered, &index, subs);
        left
    });
    if left.reads_microsoft.is_some() || login.own_events.iter().any(|e| RAISE_ONLY.contains(&e.event)) {
        return Login::Raise;
    }
    let writes = &left.facts.writes;
    let roots: Vec<(usize, usize)> = (0..lowered.program.objects[test].routines.len())
        .map(|r| (test, r))
        .collect();
    // The subscribers themselves are not followed: a raise of a login event in the test's reach reaches them, but what
    // the test can see is what they left before it. The whole reach is put together from the reach without the
    // subscribers of the `Global Triggers` database events (`own`, asked again below) and theirs, walked once per engine.
    let own = union_reach_facts_without_database_events(events, lowered, &index, &roots, subs);
    // The questions below go through the few tables and codeunits the login subscribers leave, not through the reach.
    let mine = JoinedReach::new(&own, || {
        (login.database_events).get_or_init(|| global_triggers_reach_facts(events, lowered, &index, subs))
    });
    // A table the test codeunit writes itself is not one whose rows it can only have from the login: an insert reaches
    // the database trigger subscribers of Microsoft's apps, which write the same tables in both reaches.
    if writes.iter().any(|&t| mine.touches(t) && !mine.writes_table(t))
        || left.state.iter().any(|&o| mine.reads_a_global_of(o))
    {
        return Login::Raise;
    }
    // The same question without the subscribers of the `Global Triggers` database events (al-runner issue 7574). A page
    // variable counts as a write (the page client writes the page's source table; a page without one writes a table
    // that is not known), and any write brings those subscribers into the reach above. In Microsoft's apps they write
    // the tables the login subscribers write, so a test that reads the login's row only through the page it opens (the
    // page's OnQueryClosePage, an action) looked like one that writes the table itself. What the test writes through
    // its calls, the objects it runs, the table triggers and the other events still counts as its own. That the row is
    // there before the first test, whoever reads it, is LgnLoginChecklistTests.al:137-138; the read from a page's
    // triggers is the reporter's repro, which passes on BC 29.0.
    if writes
        .iter()
        .any(|t| own.touched.contains(t) && !own.writes.contains(t))
    {
        return Login::Raise;
    }
    // A write that cannot add a row does not give the test a row of its own (al-runner issue 7605, whose repro passes on
    // BC 29.0 with a Finish action it never invokes; the review of issue 7574 found the same). The
    // Finish action of a standard wizard calls "Guided Experience".CompleteAssistedSetup, which modifies the row the
    // login registered (System Application: FindSet, then ModifyAll(Completed, true)); the page's OnQueryClosePage still
    // reads that row, and the pin above says it is there. So the login is also raised for a table the login subscribers
    // add rows to, that the test touches through its own calls, and that those calls cannot insert into. "Its own calls"
    // is what it calls, what its page variables drive and the objects it runs by id, without table triggers and event
    // subscribers: both sides are asked on that reach, because a record variable of a Base Application table brings that
    // table's event subscribers into the wider reach, and somewhere in them Microsoft's code inserts into the Guided
    // Experience tables (a wizard whose Finish action reads Customer then looked like one that registers itself). A
    // table the subscribers only modify or delete in holds no row of theirs. A record declared temporary and a page over
    // a temporary source insert nothing.
    let calls = union_call_reach_facts(events, lowered, &index, &roots, subs);
    if (left.facts.inserts.iter()).any(|t| calls.touched.contains(t) && !calls.inserts.contains(t)) {
        return Login::Raise;
    }
    if let Some(why) = watched {
        if let Some(what) = watched_state(lowered, &left.facts, &mine) {
            return Login::Refused(format!(
                "{why}: the login subscribers of the apps under test {what}, which is not a row the watch can follow"
            ));
        }
    }
    Login::Own {
        events: login.own_events.clone(),
        watched,
    }
}

/// What the login subscribers, whose reach is `left`, leave besides rows that a test codeunit whose whole reach is `mine`
/// can see, in a watched session (review of al-runner issue 7717, round three, probe p7): a manual subscriber they bind
/// (any event can reach it), a `Random` sequence they reseed (it goes on past the run seed's restart of each test,
/// ADR-061), code the analysis does not follow (`state::OTHER`), or a global of a SingleInstance codeunit they write that
/// the test codeunit reads (also through a helper codeunit).
fn watched_state(lowered: &Lowered, left: &ReachFacts, mine: &JoinedReach) -> Option<&'static str> {
    if left.state_writes & state::BINDING != 0 {
        Some("bind a manual event subscriber")
    } else if left.state_writes & state::RANDOM_SEED != 0 {
        Some("reseed the Random sequence")
    } else if left.state_writes & state::OTHER != 0 {
        Some(
            "run code the analysis does not follow (a .NET object, a background session or task, an unclassified builtin)",
        )
    } else if (left.globals_written.iter()).any(|&g| mine.reads_global(g) && single_instance(lowered, g.0)) {
        Some("write a codeunit global this test codeunit reads")
    } else {
        None
    }
}

/// Microsoft's table of the checklist's setup (System Application), which `Checklist.MarkChecklistSetupAsDone` fills.
const CHECKLIST_SETUP: &str = "Checklist Setup";

/// Whether the checklist counts as set up in `session`: Microsoft's "Checklist Setup" table holds a row.
pub(crate) fn checklist_set_up(lowered: &Lowered, session: &mut al_vm::Session) -> bool {
    let objects = &lowered.program.objects;
    let Some(t) = (0..objects.len()).find(|&i| {
        lowered.dependency.get(i).copied().unwrap_or(false)
            && objects[i].is_kind(al_ir::ObjKind::Table)
            && objects[i].name.eq_ignore_ascii_case(CHECKLIST_SETUP)
    }) else {
        return false;
    };
    session.store_mut().has_rows(t as u32)
}

/// The codeunits of the apps under test whose state a login subscriber keeps: the codeunit that declares a subscriber to
/// a login event, read from its attributes, and the codeunits of the apps under test it holds in a variable. For the
/// baseline rule (`observe.rs`), which has no event registry.
pub(crate) fn state_objects(lowered: &Lowered) -> BTreeSet<usize> {
    let objects = &lowered.program.objects;
    let mut out = BTreeSet::new();
    for i in 0..objects.len() {
        if lowered.dependency.get(i).copied().unwrap_or(true) || !declares_login_subscriber(&objects[i]) {
            continue;
        }
        out.insert(i);
        out.extend(held_codeunits(lowered, i));
    }
    out
}

/// The codeunits of the apps under test that object `o` holds in a global or local variable.
fn held_codeunits(lowered: &Lowered, o: usize) -> impl Iterator<Item = usize> + '_ {
    let obj = &lowered.program.objects[o];
    obj.globals
        .iter()
        .chain(obj.routines.iter().flat_map(|r| &r.locals))
        .filter_map(|slot| match slot.ty {
            al_builtins::value::RtType::Codeunit(c) if !lowered.dependency.get(c as usize).copied().unwrap_or(true) => {
                Some(c as usize)
            }
            _ => None,
        })
}

/// Whether a routine of `obj` declares a subscriber to one of the login events, read from its attributes.
fn declares_login_subscriber(obj: &al_ir::Object) -> bool {
    obj.routines.iter().any(|r| {
        r.attributes.iter().any(|a| {
            a.name.eq_ignore_ascii_case("EventSubscriber") && {
                let t = a.text.to_lowercase();
                LOGIN_EVENTS.iter().any(|&(id, name, event)| {
                    (t.contains(&format!("\"{}\"", name.to_lowercase())) || t.contains(&format!("::{id},")))
                        && t.contains(event)
                })
            }
        })
    })
}
