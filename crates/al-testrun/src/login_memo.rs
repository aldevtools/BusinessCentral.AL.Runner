//! System Application's login memos: the SingleInstance codeunits "Environment Information Impl." (3702) and "Server
//! Setting Impl." (3703), which keep in their globals what `EnvironmentInformation.IsSaaS()`, `IsSandbox()` and
//! `ServerSetting.GetEnableMembershipEntitlement()` asked the host on first use. Microsoft's login code fills them too.
//!
//! Measured (corpus PR 1490, open, P2LoginMemoTests.al P2A_06 and P2A_11, with the XApp A subscriber of
//! P2XaLoginMemos.al, on BC 27.5, 28.5 and 29.0): an `OnAfterLogin` subscriber of an app the run compiles as an app under
//! test calls those three procedures from its own codeunit; the test, and a table trigger the test runs, get the same
//! three values the subscriber saw. What a partner subscriber left there is kept (`partner_login::kept`).
//!
//! The login is raised through Microsoft's code first, as for any read of what that code writes. Only when that code
//! refuses (`install::NotOpened::retry_own_login`), and only for subscribers that read the memos as the probe's did
//! ([`filled_alone`]) and a test codeunit that reads them as the probe's test did ([`test_reads_measured`]), does the
//! codeunit run again with the subscribers raised alone (`login::Login::Own` with `memo_alone`). That they get there what
//! they get inside the login is the assumption `login-own-memo-alone`, marked at the reads (`partner_login::own_step`).
//! The facades and memos are matched by id, name and publisher: Microsoft's apps are compiled from their real source
//! (ADR-013), whose bodies on BC 27.5, 28.5 and 29.0 the probe ran.
//!
//! Also the caches of Microsoft's apps that a partner subscriber's login fill corpus PR 1571 measured (open, al-runner
//! issue 8945), for `partner_login::kept`: 3702 filled by two subscribers of one app ([`env_info_pair`], cell E1) and the
//! caches 423, 6153, 9170 and 9177 by id, changed globals and raise site ([`microsoft_cache_measured`], cells M1 to M4).

use crate::runtime::written::{JoinedReach, ReachIndex, login_subscriber_reach_facts, login_subscribers_reach_without};
use al_sema::lower::Lowered;
use al_vm::events::Registry;
use std::collections::BTreeSet;

/// The procedures of System Application's facades whose reads of the memos the probe measured: codeunit (id, name),
/// procedure names (corpus PR 1490, P2XaLoginMemos.al, codeunit "LT P2 XA Env Seen", procedure Record).
const MEASURED: &[(u32, &str, &[&str])] = &[
    (457, "Environment Information", &["IsSaaS", "IsSandbox"]),
    (6723, "Server Setting", &["GetEnableMembershipEntitlement"]),
];

/// Whether test codeunit reach `mine` reads of the login memos only globals that the [`MEASURED`] procedures read: the
/// probe's test and table trigger read only the values of those three procedures (P2A_06, P2A_11). Otherwise the login
/// is raised through Microsoft's code, as before.
pub(crate) fn test_reads_measured(
    events: &Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    mine: &JoinedReach,
) -> bool {
    let objs = &lowered.program.objects;
    let mut measured_reads = BTreeSet::new();
    for o in facades(lowered) {
        for r in (0..objs[o].routines.len()).filter(|&r| measured(lowered, o, r)) {
            measured_reads.extend(login_subscriber_reach_facts(events, lowered, index, o, r).globals_read);
        }
    }
    (0..objs.len())
        .filter(|&m| microsoft_login_memo(lowered, m))
        .all(|m| (0..objs[m].globals.len()).all(|g| !mine.reads_global((m, g)) || measured_reads.contains(&(m, g))))
}

/// Whether codeunit `o` is one of System Application's login memos: codeunit 3702 or 3703 of a dependency app published
/// by Microsoft.
pub(crate) fn microsoft_login_memo(lowered: &Lowered, o: usize) -> bool {
    microsoft_dependency(lowered, o) && matches!(lowered.program.objects[o].id, Some(3702 | 3703))
}

/// The caches of the dependency apps published by Microsoft, other than 3702 and 3703, whose fill by one partner login
/// subscriber and what a later test read gets there corpus PR 1571 measured (open; LmLoginCacheTests.al,
/// LmLoginCacheTlTests.al with the login subscribers of LmXaLoginCaches.al; assumption `partner-login-microsoft-cache`):
/// codeunit id, the names of exactly the globals the fill changed, and the `Global Triggers` event inside whose raise it
/// changed them (`""`: outside any raise). Change Log Management 423 loads on the database insert event of the login's first
/// write with the change log inactive (cell M3, ChangeLog_AfterLogin: only its two flags change, so `Get` found no setup
/// row and its temporary records hold no row); API Webhook Notification Mgt. 6153 on the same event (M4,
/// ApiCache_CountingSubscriber); Conf./Personalization Mgt. 9170 by a direct call (M2, Profile_AfterLogin); Application Area
/// Cache 9177 by a direct call (M1, AppAreas_AfterLogin).
const MEASURED_CACHES: &[(u32, &[&str], &str)] = &[
    (
        423,
        &["ChangeLogSetupRead", "MonitorSensitiveFieldSetupRead"],
        "ondatabaseinsert",
    ),
    (
        6153,
        &["CachedApiSubscriptionEnabled", "UseCachedApiSubscriptionEnabled"],
        "ondatabaseinsert",
    ),
    (
        9170,
        &["CurrentAllProfile", "CurrentAllProfileFetched", "ProfileFound"],
        "",
    ),
    (9177, &["ApplicationAreaCache"], ""),
];

/// Whether partner login subscriber change `by` of codeunit `memo` is one of [`MEASURED_CACHES`]: a codeunit of a dependency
/// app published by Microsoft with that id, whose changed globals are by name exactly that row's, each changed inside a raise
/// of that row's event only (or outside any raise for `""`). The caller requires that no subscriber changed it before.
pub(crate) fn microsoft_cache_measured(lowered: &Lowered, memo: usize, by: &crate::partner_login::By) -> bool {
    let obj = &lowered.program.objects[memo];
    microsoft_dependency(lowered, memo)
        && obj.is_kind(al_ir::ObjKind::Codeunit)
        && MEASURED_CACHES.iter().any(|&(id, names, event)| {
            let raised: &[&str] = if event.is_empty() { &[] } else { &[event] };
            obj.id == Some(id)
                && globals_named(lowered, memo, by.globals, names)
                && by.globals.iter().all(|&g| by.raised(g) == raised)
        })
}

/// Whether two login subscribers of one partner app, in the order they ran, changed codeunit 3702 of a dependency app
/// published by Microsoft as cell E1 of corpus PR 1571 measured (open; EnvInfo_AfterLogin, the subscribers of
/// LmXaLoginCaches.al in ascending id order, the first calls IsSandbox(), the second IsSaaS(); assumption
/// `partner-login-env-info-pair`): the first changed exactly IsSandboxConfig and IsSandboxInitialized, the second exactly
/// IsSaaSConfig and IsSaasInitialized (EnvironmentInformationImpl.Codeunit.al of System Application). The caller requires
/// that both changed them outside any `Global Triggers` raise.
pub(crate) fn env_info_pair(lowered: &Lowered, memo: usize, first: &BTreeSet<usize>, second: &BTreeSet<usize>) -> bool {
    microsoft_dependency(lowered, memo)
        && lowered.program.objects[memo].id == Some(3702)
        && globals_named(lowered, memo, first, &["IsSandboxConfig", "IsSandboxInitialized"])
        && globals_named(lowered, memo, second, &["IsSaaSConfig", "IsSaasInitialized"])
}

/// Whether the globals `globals` of codeunit `memo` are by name (any case) exactly `names`.
fn globals_named(lowered: &Lowered, memo: usize, globals: &BTreeSet<usize>, names: &[&str]) -> bool {
    let decl = &lowered.program.objects[memo].globals;
    globals.len() == names.len()
        && globals
            .iter()
            .all(|&g| (decl.get(g)).is_some_and(|d| names.iter().any(|n| d.name.eq_ignore_ascii_case(n))))
}

/// Whether object `o` is in a dependency app published by Microsoft.
pub(crate) fn microsoft_dependency(lowered: &Lowered, o: usize) -> bool {
    let objs = &lowered.program.objects;
    lowered.dependency.get(o).copied().unwrap_or(false)
        && (lowered.program.apps.get(objs[o].app as usize)).is_some_and(|a| a.publisher.trim() == "Microsoft")
}

/// The codeunits of [`MEASURED`] in the dependency apps published by Microsoft.
fn facades(lowered: &Lowered) -> Vec<usize> {
    let objs = &lowered.program.objects;
    (0..objs.len())
        .filter(|&i| {
            microsoft_dependency(lowered, i)
                && objs[i].is_kind(al_ir::ObjKind::Codeunit)
                && MEASURED
                    .iter()
                    .any(|&(id, name, _)| objs[i].id == Some(id) && objs[i].name.eq_ignore_ascii_case(name))
        })
        .collect()
}

fn measured(lowered: &Lowered, o: usize, r: usize) -> bool {
    let obj = &lowered.program.objects[o];
    MEASURED.iter().any(|&(id, _, procs)| {
        obj.id == Some(id)
            && (obj.routines.get(r)).is_some_and(|x| procs.iter().any(|p| x.name.eq_ignore_ascii_case(p)))
    })
}

/// Whether the apps under test's login subscribers `subs` read the login memos only the way the probe measured: in
/// their reach (`visited`, the walk of `login::reads_microsofts_login`, which does not enter the codeunits `skip`), every
/// routine of the facades they enter is a [`MEASURED`] procedure; every call of a facade comes from a codeunit of an app
/// under test (the probe's subscriber calls them from its own codeunit) or from the facades and memos themselves; without
/// the facades their reach reads no global of a memo; and the calls of `skip`'s codeunits (`keyed_reads`, what those
/// calls read) read none either. Otherwise the login is raised through Microsoft's code, as before.
pub(crate) fn filled_alone(
    events: &Registry,
    lowered: &Lowered,
    index: &ReachIndex,
    subs: &[(usize, usize)],
    skip: &[usize],
    visited: &[(usize, usize)],
    keyed_reads: &BTreeSet<(usize, usize)>,
) -> bool {
    let facades = facades(lowered);
    if (visited.iter()).any(|&(o, r)| facades.contains(&o) && !measured(lowered, o, r)) {
        return false;
    }
    if !calls_from_apps_under_test(lowered, visited, &facades) {
        return false;
    }
    if keyed_reads.iter().any(|g| microsoft_login_memo(lowered, g.0)) {
        return false;
    }
    let without: Vec<usize> = skip.iter().chain(&facades).copied().collect();
    let (rest, _) = login_subscribers_reach_without(events, lowered, index, subs, &without);
    !rest.globals_read.iter().any(|g| microsoft_login_memo(lowered, g.0))
}

/// Whether every call of a codeunit of `facades` in the routines `visited` is made by a codeunit of an app under test, a
/// facade or a memo.
fn calls_from_apps_under_test(lowered: &Lowered, visited: &[(usize, usize)], facades: &[usize]) -> bool {
    let objects = &lowered.program.objects;
    let mut ok = true;
    for &(o, r) in visited {
        let caller_ok = !lowered.dependency.get(o).copied().unwrap_or(false)
            || facades.contains(&o)
            || microsoft_login_memo(lowered, o);
        if caller_ok {
            continue;
        }
        let Some(routine) = objects.get(o).and_then(|x| x.routines.get(r)) else {
            continue;
        };
        al_ir::types::for_each_expr_post(routine.body.stmts(), &mut |e| {
            if let al_ir::Expr::Call { object, .. } = e {
                ok &= !facades.contains(object);
            }
        });
    }
    ok
}
