//! The run's one login on the TestRunnerHub that a codeunit with RequiredTestIsolation = Disabled holds (assumption
//! `hub-disabled-login-once`, corpus PR 1642; later runners: `hub-login-later-runners`, corpus PR 1644): which codeunit
//! holds it (`login_once_on_hub`, `first_login`), how a later session takes it (`thaw_held`), which later codeunits are
//! not measured (`login_held_unmeasured`, `login_rows_as_held`) and what their tests may not read (`held_reads`), and how
//! a Disabled codeunit's login writes meet what it carries (`carried_company_open`). Called by run.rs `open_company` and
//! `run_codeunit_inner`. Entry points: `HeldOpened::of`, `held_reads`.

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

/// A session that starts from data holding the run's one login takes that login's session state from its image in place
/// of a second login, and what the runner noted of it, marked (LgnDisabledLoginWrites.al:471, :546: no second login;
/// SingleInstance state lasts for the session, TestCuP7BReader.al:24-37). `None`: no image, or the session is not a thaw
/// target; nothing of it changed.
pub(super) fn thaw_held(
    session: &mut Session,
    held: &al_vm::HeldLogin,
) -> Option<(crate::install::CompanyOpenWrites, al_builtins::assumptions::HitBits)> {
    let opened = held_opened(held)?;
    session.thaw_held_login().ok()?;
    Some((
        opened.wrote.clone(),
        opened
            .hits
            .with(al_builtins::assumptions::Assumption::HubDisabledLoginOnce),
    ))
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

/// What a test of a session that starts from the run's one login (`held`) may not read: under runner 130450
/// (`unmeasured`, its reason) the tables the held login and this session's own login wrote; on the hub, the tables the
/// held login wrote when their rows are not as the probe measured them (`rows_as_held` false, `login_rows_as_held`),
/// and without the held image (`thawed` false) the SingleInstance codeunits the held login left, whose state this
/// session's own login set again. `None`: nothing.
pub(super) fn held_reads(
    held: &al_vm::HeldLogin,
    unmeasured: Option<&'static str>,
    thawed: bool,
    rows_as_held: bool,
    wrote: &crate::install::CompanyOpenWrites,
) -> Option<crate::install::HeldReads> {
    let first = held_opened(held)
        .map(|opened| written(&opened.wrote))
        .unwrap_or_default();
    let Some(why) = unmeasured else {
        let reads = crate::install::HeldReads {
            tables: if rows_as_held { Default::default() } else { first },
            tables_why: "an earlier codeunit with RequiredTestIsolation = Disabled on the TestRunnerHub changed the \
                         rows the run's one login wrote, and what a codeunit with Codeunit or Function isolation sees of \
                         them then is not pinned",
            singles: if thawed {
                Default::default()
            } else {
                held.singles.iter().copied().collect()
            },
            singles_why: "an earlier codeunit with RequiredTestIsolation = Disabled on the TestRunnerHub left what the \
                          run's one login wrote, and the session state that login set could not be kept for this \
                          codeunit: what its SingleInstance codeunits hold is not pinned",
        };
        return (!reads.tables.is_empty() || !reads.singles.is_empty()).then_some(reads);
    };
    let mut tables = written(wrote);
    tables.extend(first);
    // The session's own login filled its SingleInstance codeunits again (LgnHeldLoginRunners.al:608): none refused.
    Some(crate::install::HeldReads {
        tables,
        tables_why: why,
        singles: Default::default(),
        singles_why: why,
    })
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

/// Why a codeunit that starts from what a Disabled codeunit on the hub left after the run's one login
/// (`login_once_on_hub`) may not read the tables that login and its own wrote (`held_reads`): runner 130450 runs it in
/// a session of its own after every hub codeunit, whose login runs on what the hub codeunits committed and fills its
/// SingleInstance codeunits again; the probe saw only that session's own login rows (assumption
/// `hub-login-later-runners`, corpus PR 1644, LgnHeldLoginRunners.al:608, :656, :697). `None`: a codeunit on the hub
/// (`Scratch::hub_leg`), under Disabled, Codeunit or Function isolation, which takes the held login's session state
/// (LgnDisabledLoginWrites.al:471, :546; LgnHeldLoginRunners.al:452, :491, :528).
pub(super) fn login_held_unmeasured(hub_leg: bool, isolation: TestIsolation) -> Option<&'static str> {
    match (hub_leg, isolation) {
        (true, TestIsolation::Disabled | TestIsolation::Codeunit | TestIsolation::Function) => None,
        (false, TestIsolation::Disabled | TestIsolation::Codeunit | TestIsolation::Function) => Some(
            "an earlier codeunit with RequiredTestIsolation = Disabled on the TestRunnerHub left what the run's one login \
             wrote, and which login rows of other sessions a codeunit that runner 130450 runs sees is not pinned",
        ),
    }
}

/// The mark of a session that starts from the run's one login the probe of later runners measured
/// (`login_held_unmeasured`): runner 130450, and Function isolation on the hub.
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
