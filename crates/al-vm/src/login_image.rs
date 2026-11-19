//! The session image (al-runner issue 8211, PERF-01 step 2): what a test session holds after the raised login, as owned
//! data any thread may keep ([`LoginImage`], [`Session::login_image`]), and a new session made to hold it
//! ([`Session::thaw_login_image`]). Nothing replays from it yet: al-testrun `login_replay.rs` freezes each new key's
//! session, thaws it into a twin and compares the two with the login replay check.

use crate::Session;

mod size;

/// What a session holds after the login, as an image any thread may keep (see the module comment).
pub struct LoginImage {
    _built: (),
}

// The image is kept on the engine and given to sessions of other threads: `Send + Sync` by its types, no `unsafe`.
const _: fn() = || {
    fn both<T: Send + Sync>() {}
    both::<LoginImage>();
};

/// Why a session has no image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotImaged {
    /// A `Session` field (or `tx.x`, `events.x`) does not hold a new session's value.
    Field(&'static str),
    /// Code runs or a call left something behind (`Session::at_rest`).
    NotAtRest,
    /// A value with no exact form: where (ids and indexes only, never a value) and what.
    Value { path: String, what: String },
    /// A handle something outside the imaged globals also holds.
    HeldOutside { path: String },
    /// The rows: an uncommitted change, or a row value with no bytes form.
    Rows(String),
}

impl NotImaged {
    /// One line for the report, with no value in it.
    pub fn reason(&self) -> String {
        match self {
            NotImaged::Field(name) => format!("field {name}"),
            NotImaged::NotAtRest => "not at rest".to_string(),
            NotImaged::Value { path, what } => format!("{path}: {what}"),
            NotImaged::HeldOutside { path } => format!("{path}: held outside the image"),
            NotImaged::Rows(why) => format!("rows: {why}"),
        }
    }
}

/// How much an image holds ([`LoginImage::size`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImageSize {
    pub rows_bytes: u64,
    pub rows_own_bytes: u64,
    pub rows: usize,
    pub state_bytes: u64,
    pub nodes: usize,
    pub instances: usize,
    pub temp_tables: usize,
    pub temp_rows: usize,
}

impl Session {
    /// What this session holds, as an image any thread may keep. `before_login`: the store's change counter before the
    /// login ran (`MemStore::change_counter`), for `ImageSize::rows_own_bytes`.
    pub fn login_image(&self, before_login: u64) -> Result<LoginImage, NotImaged> {
        let _ = before_login;
        Err(NotImaged::Field("not built"))
    }

    /// Makes this session hold what the image holds. `self`: made by `Session::with_host` on the image's engine, with the
    /// run's session flags applied.
    pub fn thaw_login_image(&mut self, image: &LoginImage) {
        let _ = image;
    }
}

#[cfg(test)]
#[allow(dead_code)] // A1 only: removed once tests.rs uses both hooks.
impl LoginImage {
    /// Gives the second reference to the first instance that has two a node of its own. `false`: none has two.
    pub(crate) fn split_first_shared_instance(&mut self) -> bool {
        false
    }

    /// Forgets the clock's draws.
    pub(crate) fn forget_clock_draws(&mut self) {}
}

#[cfg(test)]
mod tests;
