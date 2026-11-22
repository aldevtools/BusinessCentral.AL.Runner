//! The login subscribers of the partner dependency apps in the test session (al-runner issue 8115, scenario s09).
//!
//! BC raises "System Initialization".`OnAfterLogin` in every session, the test session too, and a dependency app's
//! subscriber runs there: a group 1 filter its body sets is there in the test (G1sInstallSessionTests.al:98, :103, the
//! subscriber at G1sXaInstall.al:116-124, a dependency app whose publisher is "AL Language"), and so are the globals it
//! sets on a SingleInstance codeunit of its own app, an Integer, a List of [Text] and a Text (PlXaLoginSingles.al:12-27;
//! the first test reads them, PlLoginSingleTests.al:54-74, the next test reads what the first left, :79-98, and the
//! session-open code after the login reads them, :66-74). Where the test session raises the login through Microsoft's
//! code (`login::Login::Raise`) that code runs them. Otherwise ([`subscribers`]) they run here, at the place
//! "System Initialization Impl." raises the login, together with the apps under test's own (`install::own_login`, in the
//! order the event reaches them, [`steps`]), and what they leave is watched ([`raise`]): the measured filter and globals
//! stay, and so do the SingleInstance codeunits of the measured shapes (some through an assumption entry, [`kept`]); a
//! SingleInstance codeunit of another shape refuses every read of its globals; rows refuse their readers; other session
//! state refuses the codeunit or its readers. In the default order each test codeunit starts with fresh SingleInstance
//! state and its own login (ADR-066 amendment 12); in BC's order the single instances moved in from the codeunit before
//! replace the ones this login made (`Session::adopt_single_instances`; PlLoginSingleTests.al:109-128).

use crate::login::OwnEvent;
use al_builtins::assumptions::Assumption;
use al_sema::lower::Lowered;
use al_vm::Session;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// What the partner dependency apps' login subscribers left in the test session ([`raise`]).
#[derive(Debug, Default)]
pub(crate) struct PartnerLogin {
    /// The tables they wrote: a test or an OnRun that reads or writes one is not run.
    pub tables: BTreeSet<u32>,
    /// The tables whose group 1 filters they changed: the filter stays for the session (G1sInstallSessionTests.al:103).
    pub group1: BTreeSet<u32>,
    /// The first of `tables` and `group1` that code after them in the session open read or wrote, while the window
    /// ([`PartnerLogin::watch_after`]) was open on fewer tables than now.
    window_hit: Option<u32>,
    /// SingleInstance codeunit -> the login subscribers that changed it so far, in the order they ran ([`kept`]).
    changed_by: BTreeMap<usize, Vec<Change>>,
}

/// SingleInstance codeunit -> the globals (index) of it that a run of code changed (al-vm `LeftChange::singles`).
type Singles = BTreeMap<usize, BTreeSet<usize>>;

/// One login subscriber's change of a SingleInstance codeunit ([`PartnerLogin::changed_by`]).
#[derive(Debug)]
struct Change {
    /// The partner subscriber's app; `None` for a subscriber of an app under test.
    app: Option<u32>,
    /// The globals (index) it changed.
    globals: BTreeSet<usize>,
}

impl PartnerLogin {
    /// The tables a test is watched for.
    pub fn watched(&self) -> impl Iterator<Item = &u32> {
        self.tables.iter().chain(&self.group1)
    }

    /// After a test that began, of the tables it touched (`hits`), the first one they wrote.
    pub fn test_hit(&self, hits: &BTreeSet<u32>) -> Option<u32> {
        hits.iter().find(|t| self.tables.contains(t)).copied()
    }

    /// After a step of theirs: the code that runs after it in the session open reads what they left. Opens the store's
    /// window on the tables they wrote or filtered ([`PartnerLogin::after_open`] refuses on a hit).
    fn watch_after(&mut self, session: &mut Session) {
        let store = session.store_mut();
        if let Some(t) = store.watch_window_end() {
            self.window_hit.get_or_insert(t);
        }
        let tables: BTreeSet<u32> = self.watched().copied().collect();
        if !tables.is_empty() {
            store.watch_window(tables);
        }
    }

    /// At the end of the session open (`install::test_session_company_open`), with `end` what closing the store's window
    /// of [`PartnerLogin::watch_after`] gave: why the codeunit is not run when code after them read or wrote a table they
    /// wrote or filtered (what it found there is not measured).
    pub fn after_open(&self, end: Option<u32>, lowered: &Lowered) -> Option<String> {
        let why = self.window_hit.or(end).and_then(|t| self.touched(t, lowered))?;
        Some(format!("session open, code after the partner login subscribers: {why}"))
    }

    /// After the test codeunit's OnRun, which touched the tables `hits`: `Err` why the codeunit is not run when one of
    /// them is a table they wrote or set a group 1 filter on (what an OnRun finds there is not measured).
    pub fn after_on_run(&self, hits: &BTreeSet<u32>, lowered: &Lowered) -> Result<(), String> {
        let hit = hits.iter().find(|t| self.tables.contains(t) || self.group1.contains(t));
        match hit.and_then(|&t| self.touched(t, lowered)) {
            Some(why) => Err(why),
            None => Ok(()),
        }
    }

    /// Why code that read or wrote table `t` is not run: `None` when they did not write or filter `t`.
    pub fn touched(&self, t: u32, lowered: &Lowered) -> Option<String> {
        (self.tables.contains(&t) || self.group1.contains(&t)).then(|| {
            format!(
                "the login subscribers of the partner dependency apps ran before the first test and wrote or filtered \
                 table {}, which this code read or wrote: what BC leaves there is not pinned",
                lowered.program.objects.get(t as usize).map_or("?", |o| o.name.as_str())
            )
        })
    }
}

/// The partner dependency apps' login subscribers the test session runs itself, or none. Every fact the probe measured,
/// by equality: the login is not raised through Microsoft's code (`raised` false, `own_watched` none: no login subscriber
/// of an app under test, or `Login::Own` in the measured session); a session with a client and Codeunit isolation
/// (`session_watched` none: not `--no-client`, not Disabled; the corpus test session, G1sInstallSessionTests.al:31-34);
/// the automatic subscribers to "System Initialization".`OnAfterLogin` of a dependency app whose publisher is known and is
/// not Microsoft (`install_orders::is_partner`; G1sXaInstall.al:116, the app "AL Language"). A subscriber the compile left
/// out is not in the registry: it is not run, as on main.
pub(crate) fn subscribers(
    lowered: &Lowered,
    events: &al_vm::events::Registry,
    raised: bool,
    session_watched: Option<&'static str>,
) -> Vec<(usize, usize)> {
    if raised || session_watched.is_some() {
        return Vec::new();
    }
    let objs = &lowered.program.objects;
    (crate::login::after_login_subscribers(lowered, events).iter().copied())
        .filter(|&(o, _)| lowered.dependency[o] && crate::install_orders::is_partner(lowered, objs[o].app))
        .collect()
}

/// One raise of a login event to some of its subscribers (`install::own_login`).
pub(crate) struct Step {
    /// The publisher routine (object, routine).
    pub publisher: (usize, usize),
    /// Lower-case event name.
    pub event: &'static str,
    /// The subscribers, in the order the event reaches them.
    pub subscribers: Vec<(usize, usize)>,
    /// The subscribers are the partner dependency apps' ([`raise`]), else the apps under test's own.
    pub partner: bool,
}

/// The raises of the login: the apps under test's own events `own` in their order, and `OnAfterLogin` raised to the
/// apps' own subscribers and the partner subscribers `partner` in the order the event reaches them (the registry's), one
/// step per run of subscribers of the same side, so that what each side wrote is told apart.
pub(crate) fn steps(
    lowered: &Lowered,
    events: &al_vm::events::Registry,
    own: &[OwnEvent],
    partner: &[(usize, usize)],
) -> Vec<Step> {
    let after_login = "onafterlogin";
    let mut out: Vec<Step> = (own.iter())
        .filter(|e| e.event != after_login || partner.is_empty())
        .map(|e| Step {
            publisher: e.publisher,
            event: e.event,
            subscribers: e.subscribers.clone(),
            partner: false,
        })
        .collect();
    if partner.is_empty() {
        return out;
    }
    let own_after: &[(usize, usize)] = (own.iter().find(|e| e.event == after_login)).map_or(&[], |e| &e.subscribers);
    let Some(publisher) = crate::login::after_login_publisher(lowered, events) else {
        return out;
    };
    let order = events
        .subscribers
        .get(&(publisher.0, after_login.to_string()))
        .into_iter()
        .flatten();
    for s in order {
        let sub = (s.object, s.routine);
        let side = if partner.contains(&sub) {
            true
        } else if own_after.contains(&sub) {
            false
        } else {
            continue;
        };
        match out.last_mut() {
            Some(last) if last.event == after_login && last.partner == side => last.subscribers.push(sub),
            _ => out.push(Step {
                publisher,
                event: after_login,
                subscribers: vec![sub],
                partner: side,
            }),
        }
    }
    out
}

/// Raises step `step` of the partner subscribers, one subscriber at a time in the order the event runs them, and notes
/// what they left in `left`: the tables they wrote, the tables whose group 1 filters they changed; each changed
/// SingleInstance codeunit keeps its state or is marked ([`kept`]). `Err`: why the codeunit is not run (a subscriber that
/// cannot run, an error, a change of the session state besides rows that the probe does not measure and the watch cannot
/// follow: of the session language, WorkDate or the bindings, or of other state the test codeunit can see,
/// [`seen_by_test`]).
pub(crate) fn raise(
    lowered: &Lowered,
    session: &mut Session,
    step: &Step,
    left: &mut PartnerLogin,
) -> Result<(), crate::install::NotOpened> {
    let who = "OnAfterLogin raised to the login subscribers of the partner dependency apps";
    for &(o, r) in &step.subscribers {
        if let Some(reason) = crate::runtime::blocked_reason(lowered, o, r) {
            let why = format!(
                "{who}: {} before the first test: {reason}",
                crate::install::label(lowered, o, r)
            );
            return Err(crate::install::NotOpened::refused_at_lowering(why, o, r));
        }
    }
    let before = session.left_stamp();
    let since = session.store_mut().change_counter();
    let (po, pr) = step.publisher;
    // The SingleInstance codeunits each subscriber changed, with the globals it changed, in the order they ran.
    let mut singles_by: Vec<((usize, usize), Singles)> = Vec::new();
    crate::install::call_before_first_test(lowered, session, who, |s| {
        for sub in s.subscriber_order(po, step.event, &step.subscribers)? {
            let at = s.left_stamp();
            s.raise_to(po, pr, &[sub])?;
            singles_by.push((sub, at.changed(&s.left_stamp()).singles));
        }
        Ok(())
    })?;
    let store = session.store_mut();
    if store.virtual_write_pending() {
        return Err(format!(
            "{who} changed a platform table in the test session before the first test: what the test runner does with the change is not pinned"
        )
        .into());
    }
    left.tables.extend(store.tables_changed_since(since));
    let after = session.left_stamp();
    let change = before.changed(&after);
    if let Some(what) = change.session {
        return Err(format!(
            "{who} ran before the first test and changed {what}, which is not a row the run can watch: what BC leaves there is not pinned"
        )
        .into());
    }
    if let Some(what) = seen_by_test(lowered, session, &change) {
        return Err(format!(
            "{who} ran before the first test and changed {what}, which this test codeunit can read and which is not a row the run can watch: what BC leaves there is not pinned"
        )
        .into());
    }
    for (sub, singles) in singles_by {
        let app = lowered.program.objects[sub.0].app;
        for (o, globals) in singles.iter() {
            let earlier = left.changed_by.entry(*o).or_default();
            let verdict = kept(lowered, session, step, *o, (app, globals), singles.len(), earlier);
            earlier.push(Change {
                app: Some(app),
                globals: globals.clone(),
            });
            match verdict {
                Kept::Pinned => {}
                Kept::Assumed => session.mark_single_assumed(*o, Assumption::PartnerLoginMemoKept),
                Kept::Refused(what) => refuse_reads(lowered, session, *o, &what),
            }
        }
    }
    left.group1.extend(before.group1_changed(&after));
    left.watch_after(session);
    Ok(())
}

/// Marks SingleInstance codeunit `memo`, which a partner login subscriber changed, so that every read of one of its
/// globals refuses, for `what`, the fact no probe measured.
fn refuse_reads(lowered: &Lowered, session: &mut Session, memo: usize, what: &str) {
    let why = format!(
        "a read of a global of SingleInstance codeunit {}: OnAfterLogin raised to the login subscribers of the partner dependency apps changed it before the first test, and {what}: what BC leaves there is not pinned",
        lowered.program.objects[memo].name
    );
    session.mark_single_unmeasured(memo, why.into());
}

/// After a login step of the apps under test's own subscribers (`install::own_login`) that changed `singles` (codeunit ->
/// globals): notes the changes in `left`, for the partner subscribers after it ([`kept`] refuses), and refuses the reads
/// of each codeunit a partner subscriber changed before it. No probe has an own subscriber next to a partner one that
/// changes the same codeunit (corpus PR 1490 has none). With `memo_alone` (`login::Login::Own`, used because
/// Microsoft's login code refused), a login memo of System Application (`login_memo::microsoft_login_memo`) they
/// changed marks `login-own-memo-alone` at every read of its globals.
pub(crate) fn own_step(
    lowered: &Lowered,
    session: &mut Session,
    singles: Singles,
    left: &mut PartnerLogin,
    memo_alone: bool,
) {
    for (o, globals) in singles {
        let earlier = left.changed_by.entry(o).or_default();
        let partner_before = earlier.iter().any(|c| c.app.is_some());
        earlier.push(Change { app: None, globals });
        if partner_before {
            let what = "a subscriber of an app under test changed it after a partner subscriber did";
            refuse_reads(lowered, session, o, what);
        } else if memo_alone && crate::login_memo::microsoft_login_memo(lowered, o) {
            session.mark_single_assumed(o, Assumption::LoginOwnMemoAlone);
        }
    }
}

/// What a read of a SingleInstance codeunit a partner login subscriber changed gets ([`kept`]).
#[derive(Debug, PartialEq)]
enum Kept {
    /// What the subscriber left, pinned.
    Pinned,
    /// What the subscribers left, through assumption `partner-login-memo-kept` (corpus PR 1490).
    Assumed,
    /// Refused: the fact no probe measured, for the reason text.
    Refused(String),
}

/// What the test session gives a read of a global of SingleInstance codeunit `memo` after a partner subscriber of step
/// `step`, of app `by.0`, changed its globals `by.1` and `changed` codeunits in all; `earlier` are the changes of the
/// login subscribers that ran before it, partner and own ([`own_step`]). The facts, by equality:
/// - Pinned (PlXaLoginSingles.al:12-27, the subscriber at :56-62; read at PlLoginSingleTests.al:54-98): the step has one
///   subscriber, it changes one SingleInstance codeunit, of its own app, no subscriber changed it before, and that
///   codeunit's globals are two Integers, one List of [Text] and one Text.
/// - Assumed (`partner-login-memo-kept`, corpus PR 1490, P2LoginMemoTests.al P2A_02 to P2A_14 and P2B_01): the codeunit
///   is of a dependency app (P2XaLoginMemos.al: XApp A), the subscriber's own app (P2A_02) or an app the subscriber's
///   app depends on (P2A_03, P2A_04: XApp B's subscriber, P2XbLoginMemos.al); no subscriber changed it before, or one
///   partner subscriber of another app did, one of the two is of the codeunit's own app, and they changed other globals
///   (P2A_05: XApp A sets FromA, XApp B sets FromB); and it is codeunit 3702 or 3703 of an app published by Microsoft
///   (Environment Information Impl. and Server Setting Impl., filled by the XApp A subscriber's IsSaaS() call, P2A_06,
///   P2A_11), or of an app of another publisher with globals that are all Boolean, Integer, Text or List of [Text]
///   (P2XaLoginMemos.al codeunits 98490 to 98494).
fn kept(
    lowered: &Lowered,
    session: &Session,
    step: &Step,
    memo: usize,
    by: (u32, &BTreeSet<usize>),
    changed: usize,
    earlier: &[Change],
) -> Kept {
    use al_builtins::value::RtType;
    let objs = &lowered.program.objects;
    let (sub_app, memo_app) = (by.0, objs[memo].app);
    let globals = &objs[memo].globals;
    let count = |ty: &RtType| globals.iter().filter(|g| g.ty == *ty).count();
    let list_of_text = RtType::List(Box::new(RtType::Text(None)));
    let shape = (
        globals.len(),
        count(&RtType::Integer),
        count(&list_of_text),
        count(&RtType::Text(None)),
    );
    if step.subscribers.len() == 1 && changed == 1 && earlier.is_empty() && memo_app == sub_app && shape == (4, 2, 1, 1)
    {
        return Kept::Pinned;
    }
    if !lowered.dependency[memo] {
        return Kept::Refused("it is a codeunit of an app under test".into());
    }
    let depends = |a: u32, b: u32| {
        (session.engine.events.depends.get(a as usize)).is_some_and(|row| row.get(b as usize) == Some(&true))
    };
    if !(memo_app == sub_app || depends(sub_app, memo_app)) {
        return Kept::Refused(
            "its app is neither the subscriber's app nor an app the subscriber's app depends on".into(),
        );
    }
    if earlier.iter().any(|c| c.app.is_none()) {
        return Kept::Refused("a subscriber of an app under test changed it too".into());
    }
    // P2A_05: XApp A's subscriber and XApp B's change other globals of XApp A's memo.
    match earlier {
        [] => {}
        [Change { app: Some(a), .. }] if *a == sub_app => {
            return Kept::Refused("two subscribers of the same app changed it".into());
        }
        [e @ Change { app: Some(a), .. }] if *a == memo_app || sub_app == memo_app => {
            if let Some(&g) = e.globals.intersection(by.1).next() {
                let name = globals.get(g).map_or("?", |g| g.name.as_str());
                return Kept::Refused(format!(
                    "two subscribers changed its global {name}: the order of two subscribers whose changes do not commute is not measured"
                ));
            }
        }
        [_] => return Kept::Refused("two subscribers changed it, neither of them of its own app".into()),
        _ => return Kept::Refused("more than two subscribers changed it".into()),
    }
    let microsoft = (lowered.program.apps.get(memo_app as usize)).is_some_and(|a| a.publisher.trim() == "Microsoft");
    if microsoft {
        return if crate::login_memo::microsoft_login_memo(lowered, memo) {
            Kept::Assumed
        } else {
            Kept::Refused("it is a codeunit of an app published by Microsoft other than 3702 and 3703".into())
        };
    }
    let measured_type =
        |ty: &RtType| matches!(ty, RtType::Boolean | RtType::Integer | RtType::Text(None)) || *ty == list_of_text;
    match globals.iter().find(|g| !measured_type(&g.ty)) {
        None => Kept::Assumed,
        Some(other) => Kept::Refused(format!(
            "its global {} is of a type other than Boolean, Integer, Text and List of [Text]",
            other.name
        )),
    }
}

/// One kind of session state other than rows that the partner subscribers changed ([`seen_by_test`]).
#[derive(Clone, Copy)]
enum Left {
    NumberSequences,
    IsolatedStorage,
    Random,
}

impl Left {
    /// The kinds `change` holds.
    fn of(change: &al_vm::LeftChange) -> Vec<Left> {
        (change.number_sequences.then_some(Left::NumberSequences).into_iter())
            .chain(change.isolated_storage.then_some(Left::IsolatedStorage))
            .chain(change.random.then_some(Left::Random))
            .collect()
    }

    /// Whether a test codeunit whose whole static reach is `mine` can see this state.
    fn seen_by(self, mine: &crate::runtime::written::JoinedReach) -> bool {
        use crate::runtime::written::state;
        match self {
            Left::NumberSequences => mine.uses_state(state::NUMBER_SEQUENCE),
            Left::IsolatedStorage => mine.uses_state(state::ISOLATED_STORAGE),
            Left::Random => mine.random(),
        }
    }

    /// For a reason text.
    fn what(self) -> String {
        match self {
            Left::NumberSequences => "a number sequence".into(),
            Left::IsolatedStorage => "isolated storage".into(),
            Left::Random => "the Random sequence".into(),
        }
    }
}

/// What of `change` other than SingleInstance globals ([`kept`]) the static reach of the session's test
/// codeunit can see, for a reason text ([`Left::seen_by`]). When part of the reach is not known, or the session has
/// no test codeunit, every change counts. Changed state no test of the codeunit can see stays as the subscribers left it.
fn seen_by_test(lowered: &Lowered, session: &Session, change: &al_vm::LeftChange) -> Option<String> {
    let left = Left::of(change);
    let first = *left.first()?;
    let Some(test) = session.test_object else {
        return Some(first.what());
    };
    crate::login::ask_test_reach(&session.engine, lowered, test, |mine| {
        if let Some(why) = mine.unknown() {
            return Some(format!("{} (part of its reach is not known: {why})", first.what()));
        }
        (left.iter()).find(|l| l.seen_by(mine)).map(|l| l.what())
    })
}
