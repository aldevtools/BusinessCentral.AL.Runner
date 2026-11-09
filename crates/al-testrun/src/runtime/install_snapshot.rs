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
    why_private: std::cell::RefCell<Option<WhyPrivate>>,
    /// Why it is not written as bytes either, when it is not: the first value the bytes have no form for.
    why_no_bytes: std::cell::RefCell<Option<al_data::memory::NotShareable>>,
    /// Counts the frozen baseline once in the fixed part of the memory budget (ADR-069 decision 10, `Pool::add_fixed`).
    count_fixed: Box<dyn Fn(u64)>,
}

/// Why the test threads do not share the install baseline.
enum WhyPrivate {
    /// `RunOptions::private_baseline` (`AL_PRIVATE_BASELINE=1`).
    Asked,
    NotShared(al_vm::NotShared),
}

impl WhyPrivate {
    /// The reason in words, naming the table, field and kind of value that blocked sharing (from `program`'s tables).
    fn describe(&self, program: &al_ir::Program) -> String {
        let n = match self {
            WhyPrivate::Asked => return "AL_PRIVATE_BASELINE is set".into(),
            WhyPrivate::NotShared(al_vm::NotShared::NoReadBack) => {
                return "the install baseline does not read back from its bytes".into();
            }
            WhyPrivate::NotShared(al_vm::NotShared::Value(n)) => n,
        };
        format!(
            "a row of the install baseline holds a value that has no plain form: {}",
            describe_value(n, program)
        )
    }
}

/// Where a value sits (table, company, field) and what it is, with the names `program` gives them.
fn describe_value(n: &al_data::memory::NotShareable, program: &al_ir::Program) -> String {
    let table = program.objects.get(n.object as usize);
    let def = table.and_then(|o| o.table.as_ref());
    let field = def.and_then(|t| {
        let at = if n.in_key {
            *t.keys.first()?.get(n.position)?
        } else {
            n.position
        };
        t.fields.get(at)
    });
    let where_ = if n.in_key { "key field" } else { "field" };
    format!(
        "table {} '{}', company '{}', {where_} {}, {}",
        table.and_then(|o| o.id).map_or("?".into(), |id| id.to_string()),
        table.map_or("?", |o| o.name.as_str()),
        n.company,
        field
            .map(|f| format!("{} '{}'", f.id, f.name))
            .unwrap_or_else(|| format!("at position {}", n.position)),
        n.kind
    )
}

impl InstallSnapshots {
    /// `count_fixed` is told the size of the frozen baseline when it is made.
    pub(crate) fn new(private: bool, count_fixed: impl Fn(u64) + 'static) -> InstallSnapshots {
        InstallSnapshots {
            once: Default::default(),
            private,
            why_private: Default::default(),
            why_no_bytes: Default::default(),
            count_fixed: Box::new(count_fixed),
        }
    }

    /// The snapshot of `baseline`, made on the first call.
    pub(crate) fn get_or_make(&self, baseline: Option<&al_vm::Baseline>) -> Option<&InstallSnapshot> {
        self.once
            .get_or_init(|| {
                let b = baseline?;
                if self.private {
                    self.why_private.replace(Some(WhyPrivate::Asked));
                    return b.to_bytes().map(InstallSnapshot::Bytes);
                }
                match b.freeze() {
                    Ok(s) => {
                        (self.count_fixed)(s.bytes());
                        Some(InstallSnapshot::Shared(Box::new(s)))
                    }
                    Err(why) => {
                        self.why_private.replace(Some(WhyPrivate::NotShared(why)));
                        let bytes = b.to_bytes();
                        if bytes.is_none() {
                            self.why_no_bytes.replace(b.first_without_bytes());
                        }
                        bytes.map(InstallSnapshot::Bytes)
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
    /// `program` names the table and field that blocked sharing.
    pub(crate) fn line(&self, program: &al_ir::Program) -> Option<String> {
        let why = || self.why_private.borrow().as_ref().map(|w| w.describe(program));
        let Some(made) = self.once.get()?.as_ref() else {
            // A baseline that is neither shared nor written as bytes: each test thread runs the install code again.
            let no_bytes = match &*self.why_no_bytes.borrow() {
                Some(n) => format!("; it cannot be written as bytes either: {}", describe_value(n, program)),
                None => "; it cannot be written as bytes either".into(),
            };
            return why().map(|why| format!("install baseline: run again by each test thread ({why}{no_bytes})"));
        };
        match made {
            InstallSnapshot::Shared(s) => {
                let unchanged = s.unchanged();
                debug_assert!(
                    unchanged,
                    "a test thread wrote to the shared install baseline (ADR-069 risk 1)"
                );
                Some(format!(
                    "install baseline: shared by the test threads, {} rows{}, {} at the end",
                    s.rows(),
                    held_note(&s.held_tables(), program),
                    if unchanged { "unchanged" } else { "CHANGED" }
                ))
            }
            InstallSnapshot::Bytes(_) => Some(format!(
                "install baseline: a copy per test thread ({})",
                why().unwrap_or_else(|| "not shared".into())
            )),
        }
    }
}

/// The tables of a shared baseline that are a copy per thread because a row holds a RecordId or a TableFilter
/// (`SharedBaseline::held_tables`), for the `--timings` line: empty when there are none.
fn held_note(held: &[(String, u32, usize)], program: &al_ir::Program) -> String {
    if held.is_empty() {
        return String::new();
    }
    const SHOWN: usize = 5;
    let mut names: Vec<String> = held
        .iter()
        .take(SHOWN)
        .map(|(company, object, rows)| {
            let o = program.objects.get(*object as usize);
            format!(
                "table {} '{}' of company '{company}' ({rows} rows)",
                o.and_then(|o| o.id).map_or("?".into(), |id| id.to_string()),
                o.map_or("?", |o| o.name.as_str())
            )
        })
        .collect();
    if held.len() > SHOWN {
        names.push(format!("and {} more", held.len() - SHOWN));
    }
    format!(
        " ({} of them in {} tables a copy per thread, a RecordId or TableFilter: {})",
        held.iter().map(|h| h.2).sum::<usize>(),
        held.len(),
        names.join(", ")
    )
}
