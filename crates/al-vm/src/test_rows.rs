//! The rows of the test data built once per process (ADR-069 decision 4).
//!
//! Each thread's `TestBacking` used to convert the raw cells of every table it touched into rows of its own. The rows
//! are a pure function of the test data, company and program, so one engine keeps one cache for all its threads, and
//! shares it with its siblings over the same program (`Engine::sibling`: the install engine and the tests engine of a
//! run), so a table is built once per process: one `OnceLock` per (test data, company, table). The first thread that touches a table
//! builds it; a thread that arrives meanwhile waits for it, and tables nobody builds yet go on in parallel. An engine
//! made for another program (`Engine::with_config`) has a cache of its own, so rows built for another program are never
//! reused (risk 3).

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

/// One table's slot: built once, by the first thread that asks.
type Slot = Arc<OnceLock<Built>>;

/// One engine's cache of the test data's rows (see the module comment).
#[derive(Default)]
pub(crate) struct TestRows {
    slots: Mutex<HashMap<(u64, String, u32), Slot>>,
    /// About how many bytes the shared rows hold (`FrozenRows::bytes`), for the memory budget (decision 10).
    bytes: std::sync::atomic::AtomicU64,
}

impl TestRows {
    /// The rows of table `object` in `company` of the test data with id `data`: built by `build` on the first ask in the
    /// process, shared after. `build` runs again on a thread of its own only when the rows could not be shared.
    pub(crate) fn get(&self, data: u64, company: &str, object: u32, build: impl Fn() -> Option<Rows>) -> Option<Rows> {
        let slot = self
            .slots
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry((data, company.to_lowercase(), object))
            .or_default()
            .clone();
        let mut own = None;
        let built = slot.get_or_init(|| match build() {
            None => Built::Shared(None),
            Some(rows) => match FrozenRows::freeze(rows) {
                Ok(f) => {
                    self.bytes.fetch_add(f.bytes(), std::sync::atomic::Ordering::Relaxed);
                    Built::Shared(Some(f))
                }
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
        self.bytes.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// About how many bytes the shared rows hold: what the memory budget counts once for all threads (decision 10).
    pub(crate) fn bytes(&self) -> u64 {
        self.bytes.load(std::sync::atomic::Ordering::Relaxed)
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
                    cache.get(1, "Main", 7, build).map(|r| r.len())
                })
            })
            .collect();
        for t in threads {
            assert_eq!(t.join().unwrap(), Some(2));
        }
        assert_eq!(builds.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(cache.shared(), 1);
        let once = cache.bytes();
        assert!(
            once > 0,
            "the memory budget counts the shared rows once (ADR-069 decision 10)"
        );
        cache.get(1, "Main", 7, || None);
        assert_eq!(cache.bytes(), once, "a table already built is not counted again");
        assert_eq!(
            cache.get(2, "Main", 7, || None).map(|r| r.len()),
            None,
            "another test data builds its own"
        );
    }

    /// With several test threads the first runs on the tests' engine and the others on the engine that ran the install
    /// code (`Engine::sibling`): one program, so one cache, and each table is built once per process.
    #[test]
    fn an_engine_and_its_sibling_build_each_table_once() {
        let engine = crate::Engine::new(al_ir::Program::default(), Vec::new());
        let sibling = engine.sibling();
        let builds = std::cell::Cell::new(0);
        let build = || {
            builds.set(builds.get() + 1);
            Some(Rows::from_rows(&table(), vec![row("A", 1, 1)]).unwrap())
        };
        engine.test_rows.get(1, "Main", 7, build);
        sibling.test_rows.get(1, "Main", 7, build);
        assert_eq!(builds.get(), 1);
        assert!(
            sibling.shares_test_rows_with(&engine),
            "the memory budget counts them once"
        );
        assert!(!crate::Engine::new(al_ir::Program::default(), Vec::new()).shares_test_rows_with(&engine));
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
            // A List is not allowed in a key: the table without its secondary key (al-runner issue 7644).
            Some(Rows::from_rows(&al_data::contract::table_without_secondary_key(), vec![r]).unwrap())
        };
        assert_eq!(cache.get(1, "Main", 7, build).map(|r| r.len()), Some(1));
        assert_eq!(cache.get(1, "Main", 7, build).map(|r| r.len()), Some(1));
        assert_eq!(builds.get(), 2, "the first build is used, a later ask builds again");
        assert_eq!(cache.shared(), 0);
        assert_eq!(cache.bytes(), 0, "rows per thread are a thread's own cost");
    }

    #[test]
    fn the_same_table_is_cached_separately_for_each_company() {
        let cache = TestRows::default();
        let builds = std::cell::Cell::new(0);
        let build = || {
            builds.set(builds.get() + 1);
            Some(Rows::from_rows(&table(), vec![row("A", 1, 1)]).unwrap())
        };
        cache.get(1, "Main", 7, build);
        cache.get(1, "Other", 7, build);
        cache.get(1, "Main", 7, || None);
        assert_eq!(builds.get(), 2);
        assert_eq!(cache.shared(), 2);
    }
}
