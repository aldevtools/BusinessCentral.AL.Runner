//! The setup-ask caches of the partner login (al-runner issue 8945): SingleInstance codeunits of a partner app (a
//! dependency app or an app under test, its publisher known and not Microsoft) that a `Global Triggers`
//! `GetDatabaseTableTriggerSetup` subscriber of the codeunit's own app changes when a partner login subscriber's write
//! makes the test session ask the setup of a table. The test session asks again for a table the install session asked
//! (Y69InstallTriggerSetup.al:134, `d_setup_61180`), so with an install codeunit this ask runs in the login too. That the
//! test session keeps what the ask left, for every later read, is assumption `partner-login-setup-ask-cache` (corpus probe
//! of handoff fix-9028-ask-cells, pull request pending), judged by `partner_login::kept`.
//!
//! The raise watch of al-vm (`raise_watch.rs`) stamps the codeunits of [`ask_watched`] and names the setup-ask
//! subscriber that changed each global ([`changed_in_own_ask`]). Entry points: [`ask_watched`], [`changed_in_own_ask`].

use crate::partner_login::By;
use crate::runtime::written::{ReachIndex, call_reach_facts};
use al_sema::lower::Lowered;
use al_vm::events::Registry;
use std::collections::{BTreeMap, BTreeSet};

/// The event of the setup ask, lower case, as the registry keys it and the watch names its raise.
const ASK: &str = "getdatabasetabletriggersetup";

/// Base Application's codeunit that passes the setup ask on, (id, name, event): its subscriber of [`ASK`] raises this
/// event inside the ask, and the partner apps subscribe to it (GlobalTriggerManagement.Codeunit.al of Base Application).
const PASSED_ON: (u32, &str, &str) = (49, "GlobalTriggerManagement", "onaftergetdatabasetabletriggersetup");

/// [`ask_watched`] of the engine's program, made once per engine.
struct AskWatched(BTreeMap<usize, BTreeSet<usize>>);

/// The codeunits whose globals the raise watch stamps for [`changed_in_own_ask`], each with all of its globals: every
/// SingleInstance codeunit of a partner app whose globals the calls of a setup-ask subscriber ([`setup_subscribers`]) of the
/// same app write
/// (`written::call_reach_facts`; the subscriber's own codeunit included). A cache another code fills is not stamped, so
/// [`changed_in_own_ask`] does not hold for it.
pub(crate) fn ask_watched(lowered: &Lowered, engine: &al_vm::Engine) -> BTreeMap<usize, BTreeSet<usize>> {
    let index = engine.derived(|| ReachIndex::new(&engine.events, lowered));
    let made = engine.derived(|| {
        let objs = &lowered.program.objects;
        let mut out = BTreeMap::new();
        for (o, r) in setup_subscribers(lowered, &engine.events) {
            if !crate::install_orders::is_partner(lowered, objs[o].app) {
                continue;
            }
            let written = call_reach_facts(&engine.events, lowered, &index, o, r).globals_written;
            for (c, _) in written {
                let cache = objs[c].is_kind(al_ir::ObjKind::Codeunit)
                    && al_ir::ObjectFacts::of_properties(&objs[c].properties).single_instance
                    && objs[c].app == objs[o].app;
                if cache {
                    out.insert(c, (0..objs[c].globals.len()).collect());
                }
            }
        }
        AskWatched(out)
    });
    made.0.clone()
}

/// The setup-ask subscribers (object, routine): those of `Global Triggers`.`GetDatabaseTableTriggerSetup`, and those of
/// the event [`PASSED_ON`] of a dependency app published by Microsoft, which runs inside the ask.
fn setup_subscribers(lowered: &Lowered, events: &Registry) -> Vec<(usize, usize)> {
    let objs = &lowered.program.objects;
    let (id, name, event) = PASSED_ON;
    let passed_on = (0..objs.len()).filter(|&o| {
        crate::login_memo::microsoft_dependency(lowered, o)
            && objs[o].is_kind(al_ir::ObjKind::Codeunit)
            && objs[o].id == Some(id)
            && objs[o].name.eq_ignore_ascii_case(name)
    });
    let keys: Vec<(usize, String)> = std::iter::once((events.global_key, ASK.to_string()))
        .chain(passed_on.map(|o| (o, event.to_string())))
        .collect();
    (keys
        .iter()
        .flat_map(|k| events.subscribers.get(k).into_iter().flatten()))
    .map(|s| (s.object, s.routine))
    .collect()
}

/// Whether `by`, a partner login subscriber's change of SingleInstance codeunit `memo`, is a fill of a setup-ask cache
/// only. The facts, by equality: `memo` is a codeunit of a partner app (`install_orders::is_partner`); each global it
/// changed was noted by the watch in exactly one place, the setup ask that runs in no other `Global Triggers` raise
/// ([`ASK`]); and each subscriber routine the watch names for that change (the innermost subscriber call running) is a
/// setup-ask subscriber of `memo`'s app ([`setup_subscribers`]). A change outside that ask as well, in an ask nested in another raise, or by code of another app,
/// does not hold. The caller requires that every earlier change of `memo` in this login was such a fill.
pub(crate) fn changed_in_own_ask(lowered: &Lowered, events: &Registry, memo: usize, by: &By) -> bool {
    let objs = &lowered.program.objects;
    let app = objs[memo].app;
    let setup = setup_subscribers(lowered, events);
    let own_setup = |s: &(usize, usize)| objs[s.0].app == app && setup.contains(s);
    crate::install_orders::is_partner(lowered, app)
        && objs[memo].is_kind(al_ir::ObjKind::Codeunit)
        && !by.globals.is_empty()
        && by.globals.iter().all(|&g| {
            let askers = by.asked_by(g);
            by.raised(g) == [ASK] && !askers.is_empty() && askers.iter().all(own_setup)
        })
}
