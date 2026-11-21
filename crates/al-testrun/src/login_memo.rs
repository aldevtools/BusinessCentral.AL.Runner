//! System Application's login memos: the SingleInstance codeunits "Environment Information Impl." (3702) and "Server
//! Setting Impl." (3703), which keep in their globals what `EnvironmentInformation.IsSaaS()`, `IsSandbox()` and
//! `ServerSetting.GetEnableMembershipEntitlement()` asked the host on first use. Microsoft's login code fills them too.
//!
//! Measured (corpus PR 1490, open, P2LoginMemoTests.al P2A_06 and P2A_11, with the XApp A subscriber of
//! P2XaLoginMemos.al, on BC 27.5, 28.5 and 29.0): an `OnAfterLogin` subscriber of an app the run compiles as an app under
//! test calls those three procedures from its own codeunit; the test, and a table trigger the test runs, get the same
//! three values the subscriber saw. What a partner subscriber left there is kept (`partner_login::kept`). That an app
//! under test's subscriber, raised alone (`login::Login::Own`), gets there what it gets inside the login is the assumption
//! `login-own-memo-alone` ([`filled_alone`], marked at the reads by `partner_login::own_step`).

use crate::runtime::written::{ReachIndex, login_subscribers_reach_without};
use al_sema::lower::Lowered;
use al_vm::events::Registry;
use std::collections::BTreeSet;

/// The procedures of System Application's facades whose reads of the memos the probe measured: codeunit (id, name),
/// procedure names (corpus PR 1490, P2XaLoginMemos.al, codeunit "LT P2 XA Env Seen", procedure Record).
const MEASURED: &[(u32, &str, &[&str])] = &[
    (457, "Environment Information", &["IsSaaS", "IsSandbox"]),
    (6723, "Server Setting", &["GetEnableMembershipEntitlement"]),
];

/// Whether codeunit `o` is one of System Application's login memos: codeunit 3702 or 3703 of a dependency app published
/// by Microsoft.
pub(crate) fn microsoft_login_memo(lowered: &Lowered, o: usize) -> bool {
    microsoft_dependency(lowered, o) && matches!(lowered.program.objects[o].id, Some(3702 | 3703))
}

fn microsoft_dependency(lowered: &Lowered, o: usize) -> bool {
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
