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
//! The runtime raises the login by running the dependency apps' Company Triggers `OnCompanyOpenCompleted` subscribers
//! with the apps under test's own (`install::test_session_company_open`), the real code of Microsoft's apps. It does so
//! for a test codeunit that can see what an app under test's subscriber to one of the two events leaves
//! ([`observed_by`]); otherwise those subscribers stay out of the test session, as before. What the login writes stays
//! in the session, committed before the first test. Microsoft's code that cannot run here refuses the codeunit.

use crate::runtime::written::{
    ReachIndex, call_reach_facts, union_reach_facts_skipping, union_reach_facts_without_database_events,
};
use al_sema::lower::Lowered;
use al_vm::Engine;
use al_vm::events::Registry;
use std::collections::BTreeSet;

/// The publishers of the login events an app under test can subscribe to: (codeunit id, name, event in lower case).
const LOGIN_EVENTS: &[(u32, &str, &str)] = &[
    (150, "System Initialization", "onafterlogin"),
    (1990, "Guided Experience", "onregisterassistedsetup"),
];

/// What the apps under test's login subscribers leave: the tables they write through their own calls
/// ([`call_reach_facts`]), and the codeunits of the apps under test whose state they keep. Made once per engine.
#[derive(Default)]
struct LoginLeaves {
    writes: BTreeSet<u32>,
    state: BTreeSet<usize>,
}

/// The automatic subscribers of the apps under test to the login events (object, routine).
fn workspace_subscribers(lowered: &Lowered, events: &Registry) -> Vec<(usize, usize)> {
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
            let subs = events.subscribers.get(&(i, event.to_string())).into_iter().flatten();
            out.extend(
                subs.filter(|s| !s.manual && !lowered.dependency[s.object])
                    .map(|s| (s.object, s.routine)),
            );
        }
    }
    out
}

/// Whether the test session of test codeunit `test` raises the login: an app under test has an automatic subscriber to
/// "System Initialization".`OnAfterLogin` or "Guided Experience".`OnRegisterAssistedSetup`, and the test codeunit's reach
/// can see what those subscribers leave: it touches a table they write through their own calls and does not write
/// itself (in its whole reach, or in its reach without the subscribers of the database events), or reads a global of
/// the subscriber's codeunit or of an app under test's codeunit the subscriber's codeunit holds in a variable (the
/// state of a SingleInstance codeunit, LgnLoginChecklistTests.al:19-64). The subscribers of the events raised inside
/// the login subscribers' calls, the table triggers and the database events do not count: in Microsoft's apps that
/// fan-out writes most of Base Application (Guided Experience's telemetry reaches every telemetry logger subscriber),
/// and a test codeunit that only reads what it writes is not one the login is modelled for (corpus
/// Y64InstallSubscribers.al reads a table a telemetry logger subscriber of a cross-app fixture writes, guarded so that
/// the login writes nothing). Another test codeunit runs as before, without the login: whatever Microsoft's login code
/// leaves that no app under test subscribes to is not modelled in the test session (Microsoft's login subscribers read
/// the corpus database's demo rows, al-runner issue 7343).
pub(crate) fn observed_by(engine: &Engine, lowered: &Lowered, test: usize) -> bool {
    let events = &engine.events;
    let subs = workspace_subscribers(lowered, events);
    if subs.is_empty() {
        return false;
    }
    let index = engine.derived(|| ReachIndex::new(events, lowered));
    let left = engine.derived(|| {
        let mut left = LoginLeaves::default();
        for &(o, r) in &subs {
            left.writes
                .extend(call_reach_facts(events, lowered, &index, o, r).writes);
            // The codeunits whose state the subscriber keeps: its own, and those its codeunit holds in a variable (the
            // SingleInstance codeunit of LgnLoginChecklistTests.al:87-89), of the apps under test.
            left.state.insert(o);
            left.state.extend(held_codeunits(lowered, o));
        }
        left
    });
    let writes = &left.writes;
    let roots: Vec<(usize, usize)> = (0..lowered.program.objects[test].routines.len())
        .map(|r| (test, r))
        .collect();
    // The subscribers themselves are not followed: a raise of a login event in the test's reach reaches them, but what
    // the test can see is what they left before it.
    let mine = union_reach_facts_skipping(events, lowered, &index, &roots, &subs);
    // A table the test codeunit writes itself is not one whose rows it can only have from the login: an insert reaches
    // the database trigger subscribers of Microsoft's apps, which write the same tables in both reaches.
    if mine
        .touched
        .iter()
        .any(|t| writes.contains(t) && !mine.writes.contains(t))
        || mine.globals_read.iter().any(|g| left.state.contains(&g.0))
    {
        return true;
    }
    // The same question without the subscribers of the `Global Triggers` database events (al-runner issue 7574). A page
    // variable counts as a write (the page client writes the page's source table; a page without one writes a table
    // that is not known), and any write brings those subscribers into the reach above. In Microsoft's apps they write
    // the tables the login subscribers write, so a test that reads the login's row only through the page it opens (the
    // page's OnQueryClosePage, an action) looked like one that writes the table itself. What the test writes through
    // its calls, the objects it runs, the table triggers and the other events still counts as its own. That the row is
    // there before the first test, whoever reads it, is LgnLoginChecklistTests.al:137-138; the read from a page's
    // triggers is the reporter's repro, which passes on BC 29.0.
    let own = union_reach_facts_without_database_events(events, lowered, &index, &roots, &subs);
    own.touched
        .iter()
        .any(|t| writes.contains(t) && !own.writes.contains(t))
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
