//! The run's one login on the TestRunnerHub that a codeunit with RequiredTestIsolation = Disabled holds (assumption
//! `hub-disabled-login-once`, corpus PR 1642; later runners: `hub-login-later-runners`, corpus PR 1644): which codeunit
//! holds it (`login_once_on_hub`, `first_login`), how each session logs in (`login_plan`: a later session takes a held
//! login with `thaw_held`), which later codeunits run their own login (`runs_own_login`), in the one session runner
//! 130450 runs them in (`own_session_id`; corpus PR 1657), or see other rows (`login_rows_as_held`) and what their tests
//! may not read (`held_reads`), which OnRun writes that session rolls back (`RunnerLoginRows`), the data its login runs
//! again on without an image (`runner_login_data`), and how a Disabled codeunit's login writes meet what it carries
//! (`carried_company_open`). Called by run.rs `open_company` and `run_codeunit_inner`. Entry points: `login_plan`,
//! `HeldOpened::of`, `held_reads`.

use super::*;

/// A codeunit with RequiredTestIsolation = Disabled on the TestRunnerHub (`hub`, BC 28 and later), whose session's login a
/// dependency app's login code raised and wrote (`CompanyOpenWrites::login`), with no Company Triggers subscriber of the
/// apps under test that wrote (`who`): the hub runs every codeunit in one session and the login once, and rows committed
/// before a codeunit starts survive it, so what the codeunit leaves is where the later codeunits start, the login
/// rows as it left them (assumption `hub-disabled-login-once`, corpus PR 1642: probe-disabled-isolation-login-writes,
/// LgnDisabledLoginWrites.al:348, :471, :546, :612, :636).
pub(super) fn login_once_on_hub(
    hub: bool,
    isolation: TestIsolation,
    execute: bool,
    company_open: &crate::install::CompanyOpenWrites,
) -> bool {
    hub && isolation == TestIsolation::Disabled
        && !execute
        && company_open.who.is_none()
        && company_open.login.is_some()
}

/// The session of a codeunit that `login_once_on_hub` would hold starts the run's login (LgnDisabledLoginWrites.al:348:
/// one session, one login before the first codeunit): its data is the suite's install baseline, not what a test codeunit
/// left (`Session::starts_from_what_a_codeunit_left`), and holds no login yet. A session that starts from data that
/// holds the login keeps it (`Session::hold_login`); one that starts from what another codeunit left keeps main's
/// refusal (`carried_company_open`).
pub(super) fn first_login(hub: bool, isolation: TestIsolation, session: &Session) -> bool {
    hub && isolation == TestIsolation::Disabled
        && session.login_held().is_none()
        && !session.starts_from_what_a_codeunit_left()
}

/// A session that starts from data holding a login it shares (`held`: the run's one login, or the login of the session
/// runner 130450 runs its codeunits in, al-vm `Session::runner_login`) takes that login's session state from its image in
/// place of a second login, and what the runner noted of it, marked (LgnDisabledLoginWrites.al:471, :546: no second
/// login; SingleInstance state lasts for the session, TestCuP7BReader.al:24-37; runner 130450: corpus PR 1657,
/// LgnRunnerSessions.al:424, :426). `None`: no image, or the session is not a thaw target; nothing of it changed.
pub(super) fn thaw_held(
    session: &mut Session,
    held: &al_vm::HeldLogin,
) -> Option<(crate::install::CompanyOpenWrites, al_builtins::assumptions::HitBits)> {
    let opened = held_opened(held)?;
    session.thaw_held_login(held).ok()?;
    Some(opened.taken())
}

/// What the runner noted when the held login's session opened the company (`open_company`, `Session::hold_login`).
fn held_opened(held: &al_vm::HeldLogin) -> Option<&HeldOpened> {
    held.opened.downcast_ref()
}

/// What the runner noted when the run's one login opened the company, as `Session::hold_login` keeps it.
pub(super) struct HeldOpened {
    pub(super) wrote: crate::install::CompanyOpenWrites,
    pub(super) hits: al_builtins::assumptions::HitBits,
    /// Per (company, table), the digest of the committed rows of the tables the login wrote (`written`), right after it;
    /// `None`: a row with no byte form.
    rows: Option<RowsDigest>,
}

type RowsDigest = std::collections::BTreeMap<(String, u32), crate::login_replay::LinesDigest>;

impl HeldOpened {
    /// What a session that takes this login in place of a second one notes of it: its writes, and its marks with
    /// `hub-disabled-login-once`.
    fn taken(&self) -> (crate::install::CompanyOpenWrites, al_builtins::assumptions::HitBits) {
        (
            self.wrote.clone(),
            self.hits
                .with(al_builtins::assumptions::Assumption::HubDisabledLoginOnce),
        )
    }

    /// What `open_company` noted of the login that `session` just ran, and the rows it left.
    pub(super) fn of(
        session: &mut Session,
        wrote: crate::install::CompanyOpenWrites,
        hits: al_builtins::assumptions::HitBits,
    ) -> HeldOpened {
        let rows = crate::login_replay::committed_tables_digest(session.store_mut(), &written(&wrote)).ok();
        HeldOpened { wrote, hits, rows }
    }
}

/// The data the login of the session runner 130450 runs its codeunits in after the held login ran on, for a later codeunit
/// of that session (`runner`) when that login left no image to thaw: the login runs again on it, so its SingleInstance
/// codeunits hold what the session's one login set (assumption `hub-login-later-runners`; corpus PR 1657,
/// LgnRunnerSessions.al:790 one login row at the session's first login, where a login on the current data counts 2; :791 the
/// SecretText it set is kept; :789 one login call). `None`: not that session, or the login left an image.
pub(super) fn runner_login_data(held: &al_vm::HeldLogin, runner: bool) -> Option<&al_vm::SharedBaseline> {
    runner.then_some(())?;
    held.ran_on.as_ref()
}

/// The tables a login wrote: those its dependency apps' login code wrote, and those the apps under test's own login
/// subscribers wrote where they are watched.
fn written(w: &crate::install::CompanyOpenWrites) -> std::collections::BTreeSet<u32> {
    let mut out = w.login_tables.clone();
    out.extend(w.watched());
    out
}

/// Whether a codeunit on the hub that starts from the run's one login (`held`) sees the rows that login wrote as the
/// codeunits before it left them, as the probe measured: under Disabled isolation whether or not an earlier Disabled
/// codeunit changed them (LgnDisabledLoginWrites.al:471, :636); under Codeunit or Function isolation only when the
/// committed rows of those tables equal the ones the login left (LgnDisabledLoginWrites.al:546; corpus PR 1644,
/// LgnHeldLoginRunners.al:491, :528). A Codeunit or Function codeunit after a Disabled codeunit changed them: `false`,
/// and its tests may not read them (`held_reads`), as main refused them; the probe measures only a delete of every
/// login row (LgnDisabledLoginWrites.al:663, :694, :720), not yet served here.
pub(super) fn login_rows_as_held(held: &al_vm::HeldLogin, isolation: TestIsolation, session: &mut Session) -> bool {
    match isolation {
        TestIsolation::Disabled => true,
        TestIsolation::Codeunit | TestIsolation::Function => held_opened(held).is_some_and(|opened| {
            let now = crate::login_replay::committed_tables_digest(session.store_mut(), &written(&opened.wrote));
            matches!((now, &opened.rows), (Ok(now), Some(left)) if now == *left)
        }),
    }
}

/// How the session of a test codeunit logs in when it opens the company (run.rs `open_company`), the one owner of which
/// login a session holds and whether what it writes is undone ([`login_plan`]).
pub(super) enum LoginPlan {
    /// The session's data holds a login it shares with the codeunits before it in its session ([`shared_login`]): it
    /// takes that login's session state from its image ([`thaw_held`]); with no image it runs the login and undoes its
    /// writes, since BC runs no second login in a session (LgnDisabledLoginWrites.al:471, :546; corpus PR 1657,
    /// LgnRunnerSessions.al:424). `runner`: the session runner 130450 runs its codeunits in after the held login, where
    /// that login runs on the data the session's one login ran on ([`runner_login_data`]).
    Shared {
        held: std::sync::Arc<al_vm::HeldLogin>,
        runner: bool,
    },
    /// The first codeunit runner 130450 runs after the held login: it runs the login of that session on the rows the hub
    /// codeunits committed, not undone, and the session holds it for the later ones (al-vm `Session::hold_runner_login`;
    /// corpus PR 1657, LgnRunnerSessions.al:424, :426, :433).
    HoldRunner,
    /// The run's first login on the hub ([`first_login`]): it runs, and the session holds it where
    /// [`login_once_on_hub`] holds (al-vm `Session::hold_login`).
    HoldHub,
    /// The session runs its login.
    Run,
}

/// The [`LoginPlan`] of a session (`own_login`: runner 130450 runs it after the held login, [`own_session_id`]).
pub(super) fn login_plan(session: &Session, own_login: bool, hub: bool, isolation: TestIsolation) -> LoginPlan {
    match (shared_login(session, own_login), own_login) {
        (Some(held), runner) => LoginPlan::Shared { held, runner },
        (None, true) => LoginPlan::HoldRunner,
        (None, false) => match first_login(hub, isolation, session) {
            true => LoginPlan::HoldHub,
            false => LoginPlan::Run,
        },
    }
}

/// The login a session shares with the codeunits before it in its session, when its data holds it: on the hub the run's
/// one login (al-vm `Session::login_held`); in the session runner 130450 runs its codeunits in after the held login
/// (`own_login`) the login of that session, which its first codeunit ran (al-vm `Session::runner_login`; corpus PR 1657,
/// LgnRunnerSessions.al:424, :426: one login in that session, :433 one login row of it after the hub's). `None`: the
/// session runs a login of its own (the first codeunit runner 130450 runs after the held login, or no login is held).
fn shared_login(session: &Session, own_login: bool) -> Option<std::sync::Arc<al_vm::HeldLogin>> {
    match own_login {
        true => session.runner_login(),
        false => session.login_held(),
    }
}

/// What a test of a session that starts from a login it shares (`held`, see [`thaw_held`]) may not read: on the hub, the
/// tables the held login wrote when their rows are not as the probe measured them (`rows_as_held` false,
/// `login_rows_as_held`), and without the held image (`thawed` false) the SingleInstance codeunits the held login left,
/// whose state this session's own login set again. In the session runner 130450 runs its codeunits in after the held
/// login (`own_login`) nothing: a later codeunit sees the committed rows as the codeunits before it left them, with no
/// second login row (corpus PR 1657, LgnRunnerSessions.al:429-:434, :792-:794; what an earlier codeunit's OnRun inserted
/// there is gone, [`RunnerLoginRows::onrun_rolled_back`]), and without the image the SingleInstance codeunits as the
/// login, run again on the data the session's one login ran on, filled them ([`runner_login_data`], :789-:791).
/// `None`: nothing.
pub(super) fn held_reads(
    held: &al_vm::HeldLogin,
    own_login: bool,
    thawed: bool,
    rows_as_held: bool,
) -> Option<crate::install::HeldReads> {
    if own_login {
        return runner_held_reads(held, thawed);
    }
    let reads = crate::install::HeldReads {
        tables: match rows_as_held {
            true => Default::default(),
            false => held_opened(held)
                .map(|opened| written(&opened.wrote))
                .unwrap_or_default(),
        },
        tables_why: "an earlier codeunit with RequiredTestIsolation = Disabled on the TestRunnerHub changed the rows \
                     the run's one login wrote, and what a codeunit with Codeunit or Function isolation sees of them \
                     then is not pinned",
        singles: match thawed {
            true => Default::default(),
            false => held.singles.iter().copied().collect(),
        },
        singles_why: "an earlier codeunit with RequiredTestIsolation = Disabled on the TestRunnerHub left what the run's \
                      one login wrote, and the session state that login set could not be kept for this codeunit: what \
                      its SingleInstance codeunits hold is not pinned",
    };
    (!reads.tables.is_empty() || !reads.singles.is_empty()).then_some(reads)
}

/// The committed rows of the tables the login of the session runner 130450 runs its codeunits in after the held login
/// wrote (al-vm `Session::runner_login`), right before a codeunit of that session runs its OnRun trigger (run.rs).
pub(super) struct RunnerLoginRows {
    tables: std::collections::BTreeSet<u32>,
    /// `None`: a row with no byte form.
    rows: Option<RowDigests>,
}

/// Per (company, table), the digest of each committed row in key order.
type RowDigests = std::collections::BTreeMap<(String, u32), Vec<crate::login_replay::LinesDigest>>;

impl RunnerLoginRows {
    /// The rows before OnRun, in a codeunit that runs in that session (`own_login`, [`own_session_id`]) once its login is
    /// held. `None`: not such a codeunit.
    pub(super) fn before_on_run(session: &mut Session, own_login: bool) -> Option<RunnerLoginRows> {
        own_login.then_some(())?;
        let held = session.runner_login()?;
        let tables = written(&held_opened(&held)?.wrote);
        let rows = Self::committed(session, &tables);
        Some(RunnerLoginRows { tables, rows })
    }

    fn committed(session: &mut Session, tables: &std::collections::BTreeSet<u32>) -> Option<RowDigests> {
        (session.store_mut().committed_rows())
            .filter(|t| tables.contains(&t.object))
            .map(|t| {
                let rows = t
                    .rows()
                    .map(|row| crate::login_replay::table_digest(t.company, t.object, 1, std::iter::once(row)).ok());
                Some(((t.company.to_string(), t.object), rows.collect::<Option<_>>()?))
            })
            .collect()
    }

    /// Whether what OnRun wrote is rolled back when the codeunit ends, so the later codeunits start from the rows as they
    /// were before it (assumption `hub-login-later-runners`; corpus PR 1657, LgnRunnerSessions.al:795, :840 seen by the
    /// codeunit's own tests; :839, :877, :917 and :878, :918 gone in every later runner codeunit, with or without Commit()
    /// in OnRun). The measured shape: OnRun inserted rows (`changed`, the tables it changed, not empty) into the tables
    /// that session's login wrote, and nothing else: every row of those tables from before OnRun is still there, unchanged,
    /// and no number sequence or isolated storage changed (`storage_or_sequences` false). Any other OnRun keeps main's
    /// answer: what it wrote reaches the later codeunits (not measured).
    pub(super) fn onrun_rolled_back(
        &self,
        session: &mut Session,
        changed: &std::collections::BTreeSet<u32>,
        storage_or_sequences: bool,
    ) -> bool {
        let into_login_tables = !changed.is_empty() && changed.iter().all(|t| self.tables.contains(t));
        into_login_tables
            && !storage_or_sequences
            && matches!((&self.rows, Self::committed(session, &self.tables)), (Some(before), Some(after)) if only_inserted(before, &after))
    }
}

/// Every row of `before` is in `after`, unchanged and in key order, in the same table entry: only inserts happened.
fn only_inserted(before: &RowDigests, after: &RowDigests) -> bool {
    before.iter().all(|(entry, rows)| {
        let mut now = after.get(entry).into_iter().flatten();
        rows.iter().all(|row| now.any(|r| r == row))
    })
}

/// What a later codeunit of the session runner 130450 runs its codeunits in after the held login may not read: without
/// the image (`thawed` false) and without the data that session's one login ran on ([`runner_login_data`]: a row with no
/// bytes form), the SingleInstance codeunits that login left, whose state a login on the current data would set
/// otherwise than the one login did (corpus PR 1657, LgnRunnerSessions.al:790). `None`: nothing.
fn runner_held_reads(held: &al_vm::HeldLogin, thawed: bool) -> Option<crate::install::HeldReads> {
    match thawed || held.ran_on.is_some() || held.singles.is_empty() {
        true => None,
        false => Some(crate::install::HeldReads {
            tables: Default::default(),
            tables_why: "",
            singles: held.singles.iter().copied().collect(),
            singles_why: "the login of the session runner 130450 runs its codeunits in after the held login left no \
                          image, and the data it ran on has no bytes form to run it again on: what its SingleInstance \
                          codeunits hold for this codeunit is not pinned",
        }),
    }
}

/// How what the session wrote when it opened the company meets what a Disabled codeunit carries: `Ok(true)` when the
/// run's one login on the hub is held (`login_once_on_hub`, `open_company`), `Ok(false)` when nothing is carried or the
/// session opened without writes, `Err` for the shapes not pinned: among them a session that starts from what another
/// codeunit left (`first_login`).
pub(super) fn carried_company_open(
    held: bool,
    hub: bool,
    isolation: TestIsolation,
    execute: bool,
    carries: bool,
    company_open: &crate::install::CompanyOpenWrites,
) -> Result<bool, &'static str> {
    if held && login_once_on_hub(hub, isolation, execute, company_open) {
        return Ok(true);
    }
    match carries && (company_open.who.is_some() || company_open.login.is_some()) {
        true => Err(
            "Disabled test isolation with a Company Triggers subscriber that writes when the test session opens \
             the company: how its writes meet what an earlier codeunit left is not pinned",
        ),
        false => Ok(false),
    }
}

/// Whether a codeunit that starts from what a Disabled codeunit on the hub left after the run's one login
/// (`login_once_on_hub`) runs a login of its own: `true` when runner 130450 runs it, in another session than the hub's after every
/// hub codeunit, whose login runs on what the hub codeunits committed (assumption `hub-login-later-runners`, corpus PR
/// 1644, LgnHeldLoginRunners.al:608, :656, :697). `false`: a codeunit on the hub (`Scratch::hub_leg`), under Disabled,
/// Codeunit or Function isolation, which takes the held login's session state (LgnDisabledLoginWrites.al:471, :546;
/// LgnHeldLoginRunners.al:452, :491, :528).
pub(super) fn runs_own_login(hub_leg: bool, isolation: TestIsolation) -> bool {
    match (hub_leg, isolation) {
        (true, TestIsolation::Disabled | TestIsolation::Codeunit | TestIsolation::Function) => false,
        (false, TestIsolation::Disabled | TestIsolation::Codeunit | TestIsolation::Function) => true,
    }
}

/// The session of a codeunit that runs a login of its own on the run's one login (`held`; `runs_own_login`, not a
/// scratch run: `execute`): runner 130450 runs every such codeunit in one session, another than the hub's (assumption
/// `hub-login-later-runners`; corpus PR 1657, LgnRunnerSessions.al:423 not the hub's; :436 every earlier such codeunit had
/// the same id; :428 the same id in OnRun), whose id is not the hub session's (`host_id`, al-vm `OwnLogin::after`). That
/// session logs in once, in the first of them; the later ones take that login (`open_company`, al-vm
/// `Session::runner_login`). `None`: the session answers the host's id.
pub(super) fn own_session_id(
    held: Option<&al_vm::HeldLogin>,
    hub_leg: bool,
    isolation: TestIsolation,
    execute: bool,
    host_id: i32,
) -> Option<al_vm::OwnLogin> {
    held.filter(|_| !execute && runs_own_login(hub_leg, isolation))?;
    Some(al_vm::OwnLogin::after(host_id))
}

/// `host` with the session id of [`own_session_id`]: `SessionId()`, the Session and Active Session rows and the ids of
/// the background sessions it starts all follow it. `None`: `host` as it is. The session made on it is told too
/// (run.rs `session_from`, al-vm `Session::set_own_login`), so `Session.StopSession` of the hub session refuses.
pub(super) fn in_own_session(host: al_host::Host, own: Option<al_vm::OwnLogin>) -> al_host::Host {
    match own {
        Some(own) => host.with_identity_overrides(al_host::IdentityOverrides {
            session_id: Some(own.id),
            ..Default::default()
        }),
        None => host,
    }
}

/// The mark of a session that starts from the run's one login the probe of later runners measured
/// (`runs_own_login`): runner 130450, and Function isolation on the hub.
pub(super) fn later_runner_marks(
    held: bool,
    hub_leg: bool,
    isolation: TestIsolation,
    hits: al_builtins::assumptions::HitBits,
) -> al_builtins::assumptions::HitBits {
    match held && (!hub_leg || isolation == TestIsolation::Function) {
        true => hits.with(al_builtins::assumptions::Assumption::HubLoginLaterRunners),
        false => hits,
    }
}
