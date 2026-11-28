//! What the run's one login on the TestRunnerHub left in a session whose data a codeunit with RequiredTestIsolation =
//! Disabled then carried ([`HeldLogin`], al-testrun runtime/held_login.rs `login_once_on_hub`): the session image of that login, the
//! SingleInstance codeunits it left an instance of, and what the runner noted of the opening. A later session of the
//! same run starts from the carried data with the held login's session state thawed in place of a second login
//! ([`Session::thaw_held_login`]), or, where that is not possible, runs its login and has its writes undone
//! ([`Session::undo_login_writes`]). Entry points: [`Session::hold_login`], [`Session::login_held`].

use crate::{Baseline, LoginImage, Session};
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

impl Session {
    /// The session's data now holds what the run's one login wrote, and `image` (taken right after that login) is its
    /// session state. A session that already holds a login keeps the first.
    pub fn hold_login(&mut self, image: Result<LoginImage, String>, opened: Box<dyn std::any::Any + Send + Sync>) {
        if self.login_held.is_some() {
            return;
        }
        let mut singles: Vec<usize> = self.singletons.keys().copied().collect();
        singles.sort_unstable();
        self.login_held = Some(Arc::new(HeldLogin { image, singles, opened }));
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
        self.thaw_login_image(image).map_err(|e| format!("{e:?}"))?;
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
    use crate::{Engine, Session};

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
