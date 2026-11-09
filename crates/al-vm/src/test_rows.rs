//! The rows of the test data built once per process (ADR-069 decision 4).
//!
//! Each thread's `TestBacking` used to convert the raw cells of every table it touched into rows of its own. The rows
//! are a pure function of the test data and the program, so one engine keeps one cache for all its threads: one
//! `OnceLock` per (test data, table). The first thread that touches a table builds it; a thread that arrives meanwhile
//! waits for it, and tables nobody builds yet go on in parallel. The engine names the program (`Backing::identity` is
//! the test data's id and the engine's uid), so rows built for another program are never reused (risk 3).

use al_data::memory::{FrozenRows, Rows};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// What a table's slot holds once built.
enum Built {
    /// The rows every thread shares; `None`: the test data has none to offer for the table.
    Shared(Option<FrozenRows>),
    /// The rows hold a value that cannot be shared (decision 3): each thread builds its own.
    Own,
}

/// One engine's cache of the test data's rows (see the module comment).
#[derive(Default)]
pub(crate) struct TestRows {
    slots: Mutex<HashMap<(u64, u32), Arc<OnceLock<Built>>>>,
}

impl TestRows {
    /// The rows of table `object` of the test data with id `data`: built by `build` on the first ask in the process,
    /// shared after. `build` runs again on a thread of its own only when the rows could not be shared.
    pub(crate) fn get(&self, data: u64, object: u32, build: impl Fn() -> Option<Rows>) -> Option<Rows> {
        let slot = self
            .slots
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry((data, object))
            .or_default()
            .clone();
        let mut own = None;
        let built = slot.get_or_init(|| match build() {
            None => Built::Shared(None),
            Some(rows) => match FrozenRows::freeze(rows) {
                Ok(f) => Built::Shared(Some(f)),
                Err(rows) => {
                    own = Some(rows);
                    Built::Own
                }
            },
        });
        match built {
            Built::Shared(rows) => rows.as_ref().map(FrozenRows::thaw),
            Built::Own => own.or_else(build),
        }
    }

    /// Forgets every table built (a warm engine reset for its next request).
    pub(crate) fn clear(&self) {
        self.slots.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    /// How many tables were built and are shared.
    #[cfg(test)]
    pub(crate) fn shared(&self) -> usize {
        let slots = self.slots.lock().unwrap_or_else(|e| e.into_inner());
        slots
            .values()
            .filter(|s| matches!(s.get(), Some(Built::Shared(Some(_)))))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_builtins::value::Value;
    use al_data::contract::{row, table};

    #[test]
    fn two_threads_that_touch_the_same_table_build_it_once() {
        let cache = Arc::new(TestRows::default());
        let builds = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let (cache, builds) = (cache.clone(), builds.clone());
                std::thread::spawn(move || {
                    let build = || {
                        builds.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        Some(Rows::from_rows(&table(), vec![row("A", 1, 1), row("B", 2, 1)]).unwrap())
                    };
                    cache.get(1, 7, build).map(|r| r.len())
                })
            })
            .collect();
        for t in threads {
            assert_eq!(t.join().unwrap(), Some(2));
        }
        assert_eq!(builds.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(cache.shared(), 1);
        assert_eq!(
            cache.get(2, 7, || None).map(|r| r.len()),
            None,
            "another test data builds its own"
        );
    }

    #[test]
    fn rows_that_cannot_be_shared_are_built_per_thread() {
        let cache = TestRows::default();
        let builds = std::cell::Cell::new(0);
        let build = || {
            builds.set(builds.get() + 1);
            let r = vec![
                Value::Code("A".into()),
                Value::List(Default::default()),
                Value::Integer(1),
            ];
            Some(Rows::from_rows(&table(), vec![r]).unwrap())
        };
        assert_eq!(cache.get(1, 7, build).map(|r| r.len()), Some(1));
        assert_eq!(cache.get(1, 7, build).map(|r| r.len()), Some(1));
        assert_eq!(builds.get(), 2, "the first build is used, a later ask builds again");
        assert_eq!(cache.shared(), 0);
    }
}
