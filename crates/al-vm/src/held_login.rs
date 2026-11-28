//! What the run's one login on the TestRunnerHub left in a session whose data a codeunit with RequiredTestIsolation =
//! Disabled then carried ([`HeldLogin`], al-testrun runtime/held_login.rs `login_once_on_hub`), and what the login of the
//! one session runner 130450 runs its codeunits in after it left ([`OwnLogin`], [`Session::hold_runner_login`]; corpus PR
//! 1657): the session image of that login, the SingleInstance codeunits it left an instance of, and what the runner noted
//! of the opening. A later session of the same run starts from the carried data with that login's session state thawed
//! in place of a second login ([`Session::thaw_held_login`]), or, where that is not possible, runs its login and has its
//! writes undone ([`Session::undo_login_writes`]). Entry points: [`Session::hold_login`], [`Session::login_held`],
//! [`Session::hold_runner_login`], [`Session::runner_login`].

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
}

/// The logins whose writes a session's data holds, each with its image: the run's one login on the hub (`hub`,
/// [`Session::hold_login`]) and the login of the session runner 130450 runs its codeunits in after it (`runner`,
/// [`Session::hold_runner_login`]). Kept by the session and by the baselines captured from it, not by their bytes.
#[derive(Clone, Debug, Default)]
pub struct HeldLogins {
    pub hub: Option<Arc<HeldLogin>>,
    pub runner: Option<Arc<HeldLogin>>,
}

impl Session {
    /// The session is the one runner 130450 runs its codeunits in after the held login (`None`: it is not). Kept where
    /// the readers of the session's id see it (`EventStats::own_login`); each session starts without.
    pub fn set_own_login(&self, own: Option<OwnLogin>) {
        self.events.event_stats.own_login.set(own);
    }

    /// The session runner 130450 runs its codeunits in after the held login ([`Session::set_own_login`]).
    pub fn own_login(&self) -> Option<OwnLogin> {
        self.events.event_stats.own_login.get()
    }

    /// The SingleInstance codeunits (object index, ascending) the session holds an instance of.
    pub fn single_instances(&self) -> Vec<usize> {
        let mut singles: Vec<usize> = self.singletons.keys().copied().collect();
        singles.sort_unstable();
        singles
    }

    /// `Session.StopSession(n)` in the session runner 130450 runs its codeunits in after the held login: refused when `n`
    /// is the hub session's id, whose stop from there is not measured (assumption `hub-login-later-runners`; the probe
    /// measures only that the hub session is not active there, corpus PR 1657, LgnRunnerSessions.al:439).
    pub(crate) fn stop_of_hub_session(&self, n: i32) -> Result<(), Signal> {
        match self.events.event_stats.own_login.get() {
            Some(own) if n == own.hub_id => Err(Signal::unsupported(
                "Session.StopSession of the hub session, from the session runner 130450 runs its codeunits in after the held login: not measured (assumption `hub-login-later-runners`)",
            )),
            _ => Ok(()),
        }
    }

    /// The session's data now holds what the run's one login wrote, and `image` (taken right after that login) is its
    /// session state. A session that already holds a login keeps the first.
    pub fn hold_login(&mut self, image: Result<LoginImage, String>, opened: Box<dyn std::any::Any + Send + Sync>) {
        if self.held_logins.hub.is_some() {
            return;
        }
        self.held_logins.hub = Some(self.held_now(image, opened));
    }

    /// The session runner 130450 runs its codeunits in after the held login ran its own login, the first of those
    /// codeunits (al-testrun runtime/held_login.rs `own_session_id`): the session's data now holds what that login wrote,
    /// and `image` (taken right after it) is its session state, which the later codeunits of that session take in place
    /// of a second login (corpus PR 1657, LgnRunnerSessions.al:424, :426, :433). A session that already holds one keeps
    /// the first.
    pub fn hold_runner_login(
        &mut self,
        image: Result<LoginImage, String>,
        opened: Box<dyn std::any::Any + Send + Sync>,
    ) {
        if self.held_logins.runner.is_some() {
            return;
        }
        self.held_logins.runner = Some(self.held_now(image, opened));
    }

    /// The login the session just ran, with its image and the SingleInstance codeunits it holds now.
    fn held_now(
        &self,
        image: Result<LoginImage, String>,
        opened: Box<dyn std::any::Any + Send + Sync>,
    ) -> Arc<HeldLogin> {
        Arc::new(HeldLogin {
            image,
            singles: self.single_instances(),
            opened,
        })
    }

    /// See [`Session::hold_runner_login`].
    pub fn runner_login(&self) -> Option<Arc<HeldLogin>> {
        self.held_logins.runner.clone()
    }

    /// The session's data is what a test codeunit left, not the suite's install baseline (`Baseline::codeunit_left`).
    pub fn starts_from_what_a_codeunit_left(&self) -> bool {
        self.codeunit_left
    }

    /// See [`Session::hold_login`].
    pub fn login_held(&self) -> Option<Arc<HeldLogin>> {
        self.held_logins.hub.clone()
    }

    /// Makes this session, which starts from data that holds the login `held` ran ([`Session::login_held`],
    /// [`Session::runner_login`]), hold that login's session state in place of running a second one: the image is
    /// thawed, then the data put back as it was before. `Err`: why not (no image, or the session is not a thaw target);
    /// nothing of the session changed then.
    pub fn thaw_held_login(&mut self, held: &HeldLogin) -> Result<(), String> {
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

/// How far the id of the session runner 130450 runs its codeunits in after the held login is from the hub session's, so
/// that it is none of the ids install sessions (2 + k, al-testrun `install.rs`) and background sessions (the starting
/// session's id plus a count, `bgsession.rs`) take.
const OWN_SESSION_STRIDE: i32 = 1000;

/// The session runner 130450 runs its codeunits in after the held login: one session for all of them, not the hub
/// session (corpus PR 1657, LgnRunnerSessions.al:423 not the hub's; :436 every earlier such codeunit had the same id).
/// Its `SessionId()` and the hub session's id. Set by al-testrun ([`Session::set_own_login`]) on each session of such a
/// codeunit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnLogin {
    pub id: i32,
    hub_id: i32,
}

impl OwnLogin {
    /// The session runner 130450 runs its codeunits in after the hub session `hub_id`.
    pub fn after(hub_id: i32) -> Self {
        OwnLogin {
            id: hub_id.saturating_add(OWN_SESSION_STRIDE),
            hub_id,
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
    use super::OwnLogin;
    use crate::{Engine, Session};
    use al_builtins::native::Signal;
    use al_builtins::value::Value::{self, Boolean, Integer};

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

    /// Calls `routine` of the probe in a session whose host answers id 1, made the session runner 130450 runs its
    /// codeunits in after the held login when `own` (al-testrun sets the host's id to `OwnLogin::id` too).
    fn in_own(own: bool, routine: &str, args: Vec<Value>) -> Result<Value, Signal> {
        let l = al_testkit::compile(OWN_SRC);
        let own = own.then(|| OwnLogin::after(1));
        let mut s = Session::new(Engine::new(l.program.clone(), al_testkit::blocked(&l)));
        if let Some(own) = own {
            s.session_id_override = Some(own.id);
        }
        s.set_own_login(own);
        let ri = (s.engine.program.objects[0].routines.iter())
            .position(|r| r.name == routine)
            .expect("routine");
        let inst = s.codeunit_instance(0);
        s.call(0, ri, inst, args).map(|(v, _)| v)
    }

    fn refused(r: &Result<Value, Signal>) -> bool {
        matches!(r, Err(Signal::Unsupported(_)))
    }

    fn answer(r: Result<Value, Signal>) -> String {
        format!("{:?}", r.ok())
    }

    // In the session runner 130450 runs its codeunits in after the held login, the hub session is not active (corpus PR
    // 1657, LgnRunnerSessions.al:439) and the session's own id is (:438). Stopping the hub session from there is not
    // measured: refused. An id no session had keeps main's answer (session-system/TestSessionComplete.al:45-52).
    #[test]
    fn the_runner_session_sees_the_hub_session_inactive_and_refuses_to_stop_it() {
        assert_eq!(answer(in_own(true, "Id", vec![])), answer(Ok(Integer(1001))));
        assert_eq!(
            answer(in_own(true, "Active", vec![Integer(1)])),
            answer(Ok(Boolean(false)))
        );
        assert_eq!(
            answer(in_own(true, "Active", vec![Integer(1001)])),
            answer(Ok(Boolean(true)))
        );
        assert!(refused(&in_own(true, "Stop", vec![Integer(1)])));
        assert_eq!(
            answer(in_own(true, "Stop", vec![Integer(99999)])),
            answer(Ok(Boolean(false)))
        );
        // Main's answers outside that session: the host's own id is active, another is not.
        assert_eq!(
            answer(in_own(false, "Active", vec![Integer(1)])),
            answer(Ok(Boolean(true)))
        );
        assert_eq!(
            answer(in_own(false, "Stop", vec![Integer(99999)])),
            answer(Ok(Boolean(false)))
        );
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
