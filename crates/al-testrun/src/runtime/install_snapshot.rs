//! The install baseline as the test threads take it (ADR-069): frozen once per run and shared by every thread, or,
//! when it cannot be frozen or `RunOptions::private_baseline` asks for it, as bytes each thread reads into a copy of
//! its own (the path before ADR-069, kept as the reference of the differential check, decision 11).

/// What the test threads start from (see the module comment).
pub(crate) enum InstallSnapshot {
    /// One read-only baseline for every thread (decision 1).
    Shared(Box<al_vm::SharedBaseline>),
    /// `Baseline::to_bytes`: each thread decodes its own copy.
    Bytes(Vec<u8>),
}

impl InstallSnapshot {
    /// A test thread's own baseline over the test data of `engine`; `origin` names the baseline the snapshot was made
    /// from. `None` when the bytes do not read back.
    pub(crate) fn thread_copy(
        &self,
        engine: &std::sync::Arc<al_vm::Engine>,
        origin: Option<u64>,
    ) -> Option<al_vm::Baseline> {
        match self {
            InstallSnapshot::Shared(s) => Some(s.thaw(Some(engine))),
            InstallSnapshot::Bytes(bytes) => al_vm::Baseline::from_bytes(bytes).map(|b| match origin {
                Some(origin) => b.backed_by(engine).with_origin(origin),
                None => b.backed_by(engine),
            }),
        }
    }
}

/// The run's snapshot of the install baseline, made when a pass first runs on several threads, once per run.
pub(crate) struct InstallSnapshots {
    once: std::cell::OnceCell<Option<InstallSnapshot>>,
    /// `RunOptions::private_baseline`.
    private: bool,
    /// Why the baseline is not shared, when it is not.
    why_private: std::cell::Cell<Option<&'static str>>,
}

impl InstallSnapshots {
    pub(crate) fn new(private: bool) -> InstallSnapshots {
        InstallSnapshots {
            once: Default::default(),
            private,
            why_private: Default::default(),
        }
    }

    /// The snapshot of `baseline`, made on the first call.
    pub(crate) fn get_or_make(&self, baseline: Option<&al_vm::Baseline>) -> Option<&InstallSnapshot> {
        self.once
            .get_or_init(|| {
                let b = baseline?;
                if self.private {
                    self.why_private.set(Some("AL_PRIVATE_BASELINE is set"));
                    return b.to_bytes().map(InstallSnapshot::Bytes);
                }
                match b.freeze() {
                    Ok(s) => Some(InstallSnapshot::Shared(Box::new(s))),
                    Err(why) => {
                        self.why_private.set(Some(why));
                        b.to_bytes().map(InstallSnapshot::Bytes)
                    }
                }
            })
            .as_ref()
    }

    /// The snapshot, if one was made.
    pub(crate) fn get(&self) -> Option<&InstallSnapshot> {
        self.once.get().and_then(Option::as_ref)
    }

    /// One line for `--timings` on how the test threads took the install baseline; `None` when no snapshot was made
    /// (one test thread, or no baseline). It says whether the shared baseline still holds at the end of the run what it
    /// held when it was frozen ("Base unchanged" of ADR-069); debug builds stop when it does not.
    pub(crate) fn line(&self) -> Option<String> {
        let Some(made) = self.once.get()?.as_ref() else {
            // A baseline that is neither shared nor written as bytes: each test thread runs the install code again.
            return self.why_private.get().map(|why| {
                format!("install baseline: run again by each test thread ({why}; it cannot be written as bytes either)")
            });
        };
        match made {
            InstallSnapshot::Shared(s) => {
                let unchanged = s.unchanged();
                debug_assert!(
                    unchanged,
                    "a test thread wrote to the shared install baseline (ADR-069 risk 1)"
                );
                Some(format!(
                    "install baseline: shared by the test threads, {} rows, {} at the end",
                    s.rows(),
                    if unchanged { "unchanged" } else { "CHANGED" }
                ))
            }
            InstallSnapshot::Bytes(_) => Some(format!(
                "install baseline: a copy per test thread ({})",
                self.why_private.get().unwrap_or("not shared")
            )),
        }
    }
}
