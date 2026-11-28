//! What the run's one login on the TestRunnerHub left in a session whose data a codeunit with RequiredTestIsolation =
//! Disabled then carried ([`HeldLogin`], al-testrun runtime/held_login.rs `login_once_on_hub`): the session image of that login, the
//! SingleInstance codeunits it left an instance of, and what the runner noted of the opening. A later session of the
//! same run starts from the carried data with the held login's session state thawed in place of a second login
//! ([`Session::thaw_held_login`]), or, where that is not possible, runs its login and has its writes undone
//! ([`Session::undo_login_writes`]). Entry points: [`Session::hold_login`], [`Session::login_held`].

use crate::{Baseline, LoginImage, Session};
use al_builtins::native::Signal;
use std::sync::Arc;

/// The run's one login, as a Disabled codeunit on the hub carries it (see the module comment). Kept by the session
/// (`Session::login_held`) and by the baselines captured from it (`Baseline::login_held`, not part of the bytes).
pub struct HeldLogin {
    /// The session after the login ([`Session::login_image`]); `Err`: why no image was made.
    pub image: Result<LoginImage, String>,
    /// The SingleInstance codeunits (object index, ascending) the session held an instance of after the login.
    pub singles: Vec<usize>,
    /// What the runner noted of the opening and the assumption marks it relied on (al-testrun `CompanyOpenWrites`,
    /// `HitBits`): a session that thaws the image takes them in place of its own login's.
    pub opened: Box<dyn std::any::Any + Send + Sync>,
    /// How many later sessions ran a login of their own on it so far ([`HeldLogin::next_own_login`]).
    pub own_logins: std::sync::atomic::AtomicI32,
}

impl Session {
    /// The session is one runner 130450 made for a login of its own after the held login (`None`: it is not). Kept where
    /// the readers of `SessionId()` and the session tables see it (`EventStats::own_login`); each session starts without.
    pub fn set_own_login(&self, own: Option<OwnLogin>) {
        self.events.event_stats.own_login.set(own);
    }

    /// `what` answers the session's id (`SessionId()`, a started session's id): refused in a session runner 130450 made
    /// for a login of its own after another such session, since whether those codeunits share one session is not
    /// measured (assumption `hub-login-later-runners`; corpus PR 1644 compares each with the hub session only,
    /// LgnHeldLoginRunners.al:664, :705).
    pub(crate) fn own_login_id_read(&self, what: &str) -> Result<(), Signal> {
        match self.events.event_stats.own_login.get() {
            Some(own) if own.nth > 1 => Err(Signal::unsupported(format!(
                "{what} in a later codeunit that runner 130450 runs after the held login: whether it shares the session of an earlier one is not measured (assumption `hub-login-later-runners`)"
            ))),
            _ => Ok(()),
        }
    }

    /// `what` asks about session `n`: refused in a session runner 130450 made for a login of its own when `n` is the
    /// hub session's id or an earlier such session's, whose state from there is not measured (assumption
    /// `hub-login-later-runners`).
    pub(crate) fn own_login_other_session(&self, n: i32, what: &str) -> Result<(), Signal> {
        match self.events.event_stats.own_login.get() {
            Some(own) if n == own.hub_id || (1..own.nth).any(|k| n == own_id(own.hub_id, k)) => {
                Err(Signal::unsupported(format!(
                    "{what} of the hub session or an earlier runner 130450 session, from a session runner 130450 made after the held login: not measured (assumption `hub-login-later-runners`)"
                )))
            }
            _ => Ok(()),
        }
    }

    /// The session's data now holds what the run's one login wrote, and `image` (taken right after that login) is its
    /// session state. A session that already holds a login keeps the first.
    pub fn hold_login(&mut self, image: Result<LoginImage, String>, opened: Box<dyn std::any::Any + Send + Sync>) {
        if self.login_held.is_some() {
            return;
        }
        let mut singles: Vec<usize> = self.singletons.keys().copied().collect();
        singles.sort_unstable();
        self.login_held = Some(Arc::new(HeldLogin {
            image,
            singles,
            opened,
            own_logins: Default::default(),
        }));
    }

    /// The session's data is what a test codeunit left, not the suite's install baseline (`Baseline::codeunit_left`).
    pub fn starts_from_what_a_codeunit_left(&self) -> bool {
        self.codeunit_left
    }

    /// See [`Session::hold_login`].
    pub fn login_held(&self) -> Option<Arc<HeldLogin>> {
        self.login_held.clone()
    }

    /// Makes this session, which starts from data that holds the run's one login, hold that login's session state in
    /// place of running a second one: the image is thawed, then the data put back as it was before. `Err`: why not (no
    /// image, or the session is not a thaw target); nothing of the session changed then.
    pub fn thaw_held_login(&mut self) -> Result<(), String> {
        let held = self.login_held.clone().ok_or("no login held")?;
        let image = held.image.as_ref().map_err(Clone::clone)?;
        let before = self.capture_baseline();
        self.thaw_login_image(image).map_err(|not| not.reason())?;
        self.undo_login_writes(&before);
        Ok(())
    }

    /// Puts back the data of `before`, a capture taken before the session's login ran (or its image was thawed), so
    /// that what the login wrote is gone again; the marks the login left on isolated storage and number sequences stay,
    /// and so does its session state (al-testrun `run.rs` `open_company`).
    pub fn undo_login_writes(&mut self, before: &Baseline) {
        let marks = std::mem::take(&mut self.unmeasured);
        self.restore_baseline(before);
        self.unmeasured = marks;
    }
}

impl HeldLogin {
    /// A later session runs a login of its own on the held one's rows (al-testrun runtime/held_login.rs
    /// `own_session_id`), where the hub session answers `hub_id`: the session's place among those sessions, from 1, and
    /// its id. The baselines captured from the session share the held login (`Baseline::login_held`), so a codeunit after
    /// another one takes the next place.
    pub fn next_own_login(&self, hub_id: i32) -> OwnLogin {
        let nth = (self.own_logins).fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        OwnLogin::new(hub_id, nth)
    }
}

/// How far apart the ids of the sessions that run a login of their own are, so that they are none of the ids install
/// sessions (2 + k, al-testrun `install.rs`) and background sessions (the starting session's id plus a count,
/// `bgsession.rs`) take.
const OWN_SESSION_STRIDE: i32 = 1000;

/// The id of the `nth` session that runs a login of its own after the hub session `hub_id`.
fn own_id(hub_id: i32, nth: i32) -> i32 {
    hub_id.saturating_add(OWN_SESSION_STRIDE.saturating_mul(nth))
}

/// A session that runner 130450 made for a login of its own after the held login ([`HeldLogin::next_own_login`]):
/// its `SessionId()`, the hub session's id and its place among those sessions, from 1. Set by al-testrun
/// ([`Session::set_own_login`]) on each session of such a codeunit; the readers refuse what is not measured of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnLogin {
    pub id: i32,
    hub_id: i32,
    nth: i32,
}

impl OwnLogin {
    /// The `nth` session that runs a login of its own after the hub session `hub_id`.
    pub(crate) fn new(hub_id: i32, nth: i32) -> Self {
        OwnLogin {
            id: own_id(hub_id, nth),
            hub_id,
            nth,
        }
    }
}

impl std::fmt::Debug for HeldLogin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeldLogin")
            .field("imaged", &self.image.as_ref().map(|_| ()))
            .field("singles", &self.singles)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::{OwnLogin, own_id};
    use crate::{Engine, Session};
    use al_builtins::native::Signal;

    const OWN_SRC: &str = r#"
codeunit 50100 Probe
{
    procedure Id(): Integer
    begin
        exit(SessionId());
    end;
    procedure Active(Id: Integer): Boolean
    begin
        exit(Session.IsSessionActive(Id));
    end;
    procedure Stop(Id: Integer): Boolean
    begin
        exit(Session.StopSession(Id));
    end;
}
"#;

    /// Calls `routine` of the probe in a session that is the `nth` one runner 130450 made after the hub session 1.
    fn in_own(
        nth: Option<i32>,
        routine: &str,
        args: Vec<al_builtins::value::Value>,
    ) -> Result<al_builtins::value::Value, Signal> {
        let l = al_testkit::compile(OWN_SRC);
        let mut s = Session::new(Engine::new(l.program.clone(), al_testkit::blocked(&l)));
        s.set_own_login(nth.map(|n| OwnLogin::new(1, n)));
        let ri = (s.engine.program.objects[0].routines.iter())
            .position(|r| r.name == routine)
            .expect("routine");
        let inst = s.codeunit_instance(0);
        s.call(0, ri, inst, args).map(|(v, _)| v)
    }

    fn refused(r: &Result<al_builtins::value::Value, Signal>) -> bool {
        matches!(r, Err(Signal::Unsupported(_)))
    }

    // Whether the codeunits runner 130450 runs after the held login share one session is not measured (assumption
    // `hub-login-later-runners`, corpus PR 1644): the first such session answers its id, a later one's read is refused.
    #[test]
    fn a_later_own_login_session_refuses_its_session_id() {
        assert!(!refused(&in_own(None, "Id", vec![])));
        assert!(!refused(&in_own(Some(1), "Id", vec![])));
        assert!(refused(&in_own(Some(2), "Id", vec![])));
    }

    // From a session runner 130450 made after the held login, whether the hub session (or an earlier such session) is
    // active is not measured: IsSessionActive and StopSession of its id are refused; an id no session had keeps main's
    // answer (session-system/TestSessionComplete.al:45-52).
    #[test]
    fn an_own_login_session_refuses_the_hub_session_id() {
        use al_builtins::value::Value::Integer;
        for routine in ["Active", "Stop"] {
            assert!(refused(&in_own(Some(1), routine, vec![Integer(1)])), "{routine}");
            assert!(
                refused(&in_own(Some(2), routine, vec![Integer(own_id(1, 1))])),
                "{routine}"
            );
            assert!(!refused(&in_own(Some(1), routine, vec![Integer(99999)])), "{routine}");
            assert!(!refused(&in_own(None, routine, vec![Integer(99999)])), "{routine}");
        }
    }

    // The marks the login left on number sequences stay after its writes are undone; the restore alone drops them.
    #[test]
    fn undoing_the_login_writes_keeps_the_unmeasured_marks() {
        let l = al_testkit::compile("codeunit 50100 C { }");
        let mut s = Session::new(Engine::new(l.program.clone(), al_testkit::blocked(&l)));
        let before = s.capture_baseline();
        s.refuse_unmeasured_sequence(&("CRONUS".into(), "Seq".into()), "why".into());
        s.undo_login_writes(&before);
        assert!(!s.unmeasured.as_new());
        s.restore_baseline(&before);
        assert!(
            s.unmeasured.as_new(),
            "a plain restore drops the marks, so the undo must keep them"
        );
    }
}
