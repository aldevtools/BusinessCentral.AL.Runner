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
//! stay; rows refuse their readers; a SingleInstance codeunit of another shape or of another app refuses every read of
//! its globals; other session state refuses the codeunit or its readers. In the default order each test codeunit starts with fresh SingleInstance
//! state and its own login (ADR-066 amendment 12); in BC's order the single instances moved in from the codeunit before
//! replace the ones this login made (`Session::adopt_single_instances`; PlLoginSingleTests.al:109-128).

use crate::login::OwnEvent;
use al_sema::lower::Lowered;
use al_vm::Session;
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

/// Raises step `step` of the partner subscribers and notes what they left in `left`: the tables they wrote, the tables whose
/// group 1 filters they changed; a changed SingleInstance codeunit other than the measured one is marked so that every
/// read of it is refused ([`unmeasured_singles`]). `Err`: why the codeunit is not run (a subscriber that cannot run, an
/// error, a change of the session state besides rows that the probe does not measure and the watch cannot follow: of the
/// session language, WorkDate or the bindings, or of other state the test codeunit can see, [`seen_by_test`]).
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
    crate::install::call_before_first_test(lowered, session, who, |s| s.raise_to(po, pr, &step.subscribers))?;
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
    for o in unmeasured_singles(lowered, step, &change.singles) {
        let why = format!(
            "a read of a global of SingleInstance codeunit {}: {who} changed it before the first test, and it is not the one codeunit of the one subscriber's app with the measured globals (two Integer, a List of [Text], a Text): what BC leaves there is not pinned",
            lowered.program.objects[o].name
        );
        session.mark_single_unmeasured(o, why.into());
    }
    left.group1.extend(before.group1_changed(&after));
    left.watch_after(session);
    Ok(())
}

/// Of the SingleInstance codeunits whose state the step changed (`singles`), the ones outside the measured shape: every
/// read of one of their globals is refused (al-vm `single_unmeasured.rs`; al-runner issue 8239), a test or other code that reads
/// none of them runs. The pin (PlXaLoginSingles.al:12-27, the subscriber at :56-62): the step has one subscriber, it changes
/// one SingleInstance codeunit of its own app, and that codeunit's globals are two Integers, one List of [Text] and one Text.
fn unmeasured_singles(lowered: &Lowered, step: &Step, singles: &BTreeSet<usize>) -> Vec<usize> {
    use al_builtins::value::RtType;
    let objs = &lowered.program.objects;
    let measured = |o: usize| {
        let globals = |ty: &RtType| objs[o].globals.iter().filter(|g| g.ty == *ty).count();
        let shape = (
            objs[o].globals.len(),
            globals(&RtType::Integer),
            globals(&RtType::List(Box::new(RtType::Text(None)))),
            globals(&RtType::Text(None)),
        );
        matches!(step.subscribers.as_slice(), [(s, _)] if objs[*s].app == objs[o].app)
            && singles.len() == 1
            && shape == (4, 2, 1, 1)
    };
    singles.iter().copied().filter(|&o| !measured(o)).collect()
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

/// What of `change` other than SingleInstance globals ([`unmeasured_single`]) the static reach of the session's test
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
