//! A SingleInstance codeunit that more than two partner login subscribers changed in one run of
//! "System Initialization".`OnAfterLogin` (al-runner issue 8945): [`kept`] says whether it keeps what they left through
//! assumption `login-subscribers-shared-memo` (corpus PR 1567, LsLoginSubscriberTests.al with the memos of
//! LsXaLoginSubscribers.al, cells A and B). One shape per measured cell, every fact by equality; any other shape keeps
//! `partner_login::kept`'s refusal. Called by `partner_login::raise` when that refuses. Entry point: [`kept`].

use al_builtins::value::RtType;
use al_sema::lower::Lowered;
use al_vm::Session;
use std::collections::BTreeMap;

/// The globals of a measured memo, as (type, count).
type Globals = BTreeMap<RtTypeKey, usize>;

/// A global's type as the cells name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RtTypeKey {
    Boolean,
    Integer,
    Text,
    ListOfText,
    /// A temporary record of a table of the memo's own app.
    OwnTemporaryRecord,
}

/// Whether SingleInstance codeunit `memo`, changed in one partner step whose subscribers ran in the order
/// `step_subscribers`, by subscribers of the apps `changers` in the order they ran (`None`: a subscriber of an app under
/// test), keeps what they left through assumption `login-subscribers-shared-memo`. `in_global_raise`: a `Global
/// Triggers` event raised during the step changed it (al-vm `Session::take_global_trigger_changes`). The facts, by
/// equality, cited to corpus PR 1567 (open):
/// - the memo is of a dependency app whose publisher is known and is not Microsoft (LsXaLoginSubscribers.al:95, :142,
///   :226, XApp A);
/// - no `Global Triggers` event raised in the step changed it: a load-once cache filled from the setup asks can load in
///   Microsoft's login code in some sessions (LsLoginSubscriberTests.al:43-52, cell C), and what such a load keeps
///   depends on its body (corpus PR 1571, open, LmLoginMemoShapeTests.al:30-36: F3 and F4 differ by session kind);
/// - every changer is a partner subscriber, and their apps in run order, grouped, are one of the measured cells:
///   State One, three of the memo's own app (:337-354, the subscribers 98530, 98533, 98536); State Many, one each of
///   three apps (:356-373, A1, B1, C1); Order, six of the memo's app, then two and two (:323-335);
/// - for State Many and Order, the first app is the memo's, each later app depends on every earlier one, and the lowest
///   subscriber codeunit id of the step of each app is above the one before (98530 < 98531 < 98532; the probe does not
///   tell dependency order, publish order and lowest id apart, :37-38);
/// - the memo's globals are State One's and State Many's (two temporary records of its own app's tables, three Booleans,
///   two Texts, two Integers; LsXaLoginSubscribers.al:147-155, :231-239) for those cells, and the Order memo's (one
///   List of [Text], one Text; :100-101) for Order.
pub(crate) fn kept(
    lowered: &Lowered,
    session: &Session,
    step_subscribers: &[(usize, usize)],
    memo: usize,
    changers: &[Option<u32>],
    in_global_raise: bool,
) -> bool {
    let objs = &lowered.program.objects;
    let memo_app = objs[memo].app;
    if in_global_raise || !lowered.dependency[memo] || !crate::install_orders::is_partner(lowered, memo_app) {
        return false;
    }
    let Some(apps) = changers.iter().copied().collect::<Option<Vec<u32>>>() else {
        return false;
    };
    let runs = runs(&apps);
    let counts: Vec<usize> = runs.iter().map(|r| r.1).collect();
    let globals = globals(lowered, memo);
    let state = Globals::from([
        (RtTypeKey::OwnTemporaryRecord, 2),
        (RtTypeKey::Boolean, 3),
        (RtTypeKey::Text, 2),
        (RtTypeKey::Integer, 2),
    ]);
    let order = Globals::from([(RtTypeKey::ListOfText, 1), (RtTypeKey::Text, 1)]);
    let chain = || chain(lowered, session, step_subscribers, memo_app, &runs);
    match counts.as_slice() {
        [3] => runs[0].0 == memo_app && globals == Some(state),
        [1, 1, 1] => chain() && globals == Some(state),
        [6, 2, 2] => chain() && globals == Some(order),
        _ => false,
    }
}

/// `apps` grouped into runs of the same app, in order: (app, length).
fn runs(apps: &[u32]) -> Vec<(u32, usize)> {
    let mut out: Vec<(u32, usize)> = Vec::new();
    for &a in apps {
        match out.last_mut() {
            Some((last, n)) if *last == a => *n += 1,
            _ => out.push((a, 1)),
        }
    }
    out
}

/// The apps of `runs` are a dependency chain that starts at `memo_app` (each depends on every earlier one, and no app
/// comes twice), and the lowest codeunit id of each app's subscribers in `step_subscribers` ascends along it.
fn chain(
    lowered: &Lowered,
    session: &Session,
    step_subscribers: &[(usize, usize)],
    memo_app: u32,
    runs: &[(u32, usize)],
) -> bool {
    let apps: Vec<u32> = runs.iter().map(|r| r.0).collect();
    let lowest = |app: u32| {
        (step_subscribers.iter())
            .filter(|s| lowered.program.objects[s.0].app == app)
            .filter_map(|s| lowered.program.objects[s.0].id)
            .min()
    };
    let ids: Vec<Option<u32>> = apps.iter().map(|&a| lowest(a)).collect();
    apps.first() == Some(&memo_app)
        && (1..apps.len()).all(|i| (0..i).all(|j| crate::partner_login::depends(session, apps[i], apps[j])))
        && ids.iter().all(Option::is_some)
        && ids.windows(2).all(|w| w[0] < w[1])
}

/// The globals of `memo` by measured type; `None` when one is of another type.
fn globals(lowered: &Lowered, memo: usize) -> Option<Globals> {
    let objs = &lowered.program.objects;
    let memo_app = objs[memo].app;
    let mut out = Globals::new();
    for g in &objs[memo].globals {
        let key = match &g.ty {
            RtType::Boolean => RtTypeKey::Boolean,
            RtType::Integer => RtTypeKey::Integer,
            RtType::Text(None) => RtTypeKey::Text,
            RtType::List(t) if **t == RtType::Text(None) => RtTypeKey::ListOfText,
            RtType::Record(t, true) if objs.get(*t as usize).is_some_and(|o| o.app == memo_app) => {
                RtTypeKey::OwnTemporaryRecord
            }
            _ => return None,
        };
        *out.entry(key).or_default() += 1;
    }
    Some(out)
}
