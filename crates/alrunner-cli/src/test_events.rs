//! What `al-runner serve` hears of each selected test as it finishes (ADR-028): the `on_test` hook of a request's run
//! and of each of its entries (al-runner issue 5635).

use crate::apps::AppInput;
use crate::run::OnTest;
use crate::{capture, coverage};
use al_testrun::events::TestEvent;
use al_testrun::failure_stop::Counts;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// What the hooks of one run share: where the tests are told, how their captures are labeled, and the next loop id.
#[derive(Clone)]
pub(crate) struct Stream {
    forward: OnTest,
    cfg: al_vm::CaptureConfig,
    paths: Arc<Vec<PathBuf>>,
    cwd: PathBuf,
    /// Loop ids are not repeated within a request, whatever test they come from.
    next_loop: Arc<AtomicU32>,
}

impl Stream {
    /// The stream of a run to `forward`; `None` without it.
    pub(crate) fn new(forward: Option<OnTest>, cfg: al_vm::CaptureConfig, inputs: &[AppInput]) -> Option<Stream> {
        Some(Stream {
            forward: forward?,
            cfg,
            paths: Arc::new(inputs.iter().map(|i| i.path.clone()).collect()),
            cwd: std::env::current_dir().unwrap_or_default(),
            next_loop: Arc::new(AtomicU32::new(0)),
        })
    }

    /// The `on_test` hook: each test `picks` selects is told, with `entry` and with its capture as JSON.
    pub(crate) fn on_test(&self, picks: Counts, entry: Option<usize>) -> Arc<dyn Fn(TestEvent) + Send + Sync> {
        let s = self.clone();
        Arc::new(move |mut e: TestEvent| {
            if picks(e.codeunit, &e.test) {
                if entry.is_some() {
                    e.entry = entry;
                }
                let json = e.captured.as_ref().map(|c| {
                    let base = s.next_loop.fetch_add(c.captured.loops.len() as u32, Ordering::SeqCst);
                    capture::json(c, s.cfg, base, &|app: usize, file: &str| {
                        coverage::label(&s.paths[app], file, &s.cwd)
                    })
                });
                (s.forward)(e, json)
            }
        })
    }
}
