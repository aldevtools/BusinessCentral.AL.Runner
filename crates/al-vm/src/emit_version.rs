//! The emit version of the BC build a run is (`NavEnvironment.EmitVersion` on BC), the number BC writes in the error for a
//! missing object (`transactions.rs` `missing_object_from`). The front end reads it from the build's platform artifact
//! (`al_deps::emit_version`) and hands the reader in: a core crate does no OS access (ADR-006). The reader runs the first
//! time the number is asked for, so a run that raises no such error never fetches the file it is read from.
//!
//! The emit version of the run's build that the missing-object error prints (`transactions.rs`
//! `missing_object_from`), read once on first ask through a reader the front end hands in. Entry
//! points: `EmitVersion` (`Engine::emit_version`).
use std::sync::{Arc, OnceLock};

/// Reads the number, or says why it cannot.
pub type ReadFn = dyn Fn() -> Result<u32, String> + Send + Sync;

/// The emit version of the run's build, read on first ask and then kept. The default has no reader: the run has no BC
/// build to read it from.
#[derive(Clone, Default)]
pub struct EmitVersion(Option<Arc<Source>>);

struct Source {
    read: Box<ReadFn>,
    value: OnceLock<Result<u32, String>>,
}

impl EmitVersion {
    /// The number `read` gives, asked once.
    pub fn from_fn(read: impl Fn() -> Result<u32, String> + Send + Sync + 'static) -> EmitVersion {
        EmitVersion(Some(Arc::new(Source {
            read: Box::new(read),
            value: OnceLock::new(),
        })))
    }

    /// A number already known (a test that names it).
    pub fn known(n: u32) -> EmitVersion {
        EmitVersion::from_fn(move || Ok(n))
    }

    /// Whether `other` answers from this reader (a clone of it), or both have none. Two readers made apart, even of one
    /// build, are not the same: the number is not read to compare them (al-testrun's login key holds the reader).
    pub fn same_reader(&self, other: &EmitVersion) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }

    /// The number, or why it is not known.
    pub fn get(&self) -> Result<u32, String> {
        match &self.0 {
            Some(s) => s.value.get_or_init(|| (s.read)()).clone(),
            None => Err("the run has no BC build to read it from".into()),
        }
    }
}

impl std::fmt::Debug for EmitVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(s) => match s.value.get() {
                Some(Ok(n)) => write!(f, "EmitVersion({n})"),
                Some(Err(why)) => write!(f, "EmitVersion(not known: {why})"),
                None => f.write_str("EmitVersion(not read yet)"),
            },
            None => f.write_str("EmitVersion(none)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn the_reader_runs_once_and_its_answer_is_kept() {
        let calls = Arc::new(AtomicU32::new(0));
        let c = calls.clone();
        let v = EmitVersion::from_fn(move || {
            c.fetch_add(1, Ordering::Relaxed);
            Err("offline".into())
        });
        let copy = v.clone();
        assert_eq!(v.get(), Err("offline".into()));
        assert_eq!(copy.get(), Err("offline".into()));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(EmitVersion::known(29022).get(), Ok(29022));
        assert!(EmitVersion::default().get().is_err());
    }

    #[test]
    fn a_clone_has_the_same_reader_and_a_reader_made_apart_does_not() {
        let v = EmitVersion::known(29022);
        assert!(v.same_reader(&v.clone()));
        assert!(!v.same_reader(&EmitVersion::known(29022)));
        assert!(!v.same_reader(&EmitVersion::default()));
        assert!(EmitVersion::default().same_reader(&EmitVersion::default()));
    }
}
