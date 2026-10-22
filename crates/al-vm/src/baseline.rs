//! The install baseline (ADR-013 decision 5, ADR-004): the data a session holds after the
//! dependency apps' install codeunits and `Company-Initialize` ran, captured once and put back
//! at the start of every test codeunit. Only data is part of it: the rows (a persistent map, so
//! the capture is a clone of a root pointer), the counters that must not repeat (system ids,
//! row versions, AutoIncrement), isolated storage and number sequences. Execution state
//! (single-instance codeunits, bindings, handlers) starts fresh, as in a new BC session.

use crate::Session;

/// What a session keeps of the baseline run; see the module comment.
#[derive(Clone)]
pub struct Baseline {
    store: al_data::memory::MemStore,
    next_guid: u64,
    next_row_version: i64,
    committed_row_version: i64,
    autoinc: std::collections::HashMap<(String, u32), i64>,
    isolated_storage: std::collections::HashMap<(String, String), (String, bool, bool)>,
    sequences: std::collections::HashMap<(String, String), crate::sessionops::Sequence>,
    last_stamp: i64,
    /// The workspace app's own install codeunits ran once, before this capture (the runner does it once per run, as
    /// publishing an app does, not once per test codeunit): the tables of those that could not run, and why.
    /// `None`: they did not run here and each test codeunit's session runs them.
    pub workspace_installs: Option<Vec<(std::collections::BTreeSet<u32>, String)>>,
}

/// The first bytes of a snapshot: what it is and which layout it has. A change of the layout changes the last digit.
const MAGIC: &[u8; 8] = b"ALBASE\0\x01";

impl Baseline {
    /// The baseline as bytes, to keep on disk or to hand to another thread (a `Baseline` holds `Rc`s and cannot go
    /// there itself). `None` when it cannot be written: a row holds something that is not a plain value. Over test
    /// data the bytes hold the tables the install code wrote to (see [`Baseline::backed_by`]). The same baseline always gives the same bytes.
    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        use al_builtins::value_codec::*;
        let mut out = MAGIC.to_vec();
        let tables = self.store.export_committed()?;
        put_u32(&mut out, tables.len() as u32);
        for t in &tables {
            put_str(&mut out, &t.company);
            put_u32(&mut out, t.object);
            put_u32(&mut out, t.rows.len() as u32);
            for (key, row) in &t.rows {
                if !encode_row(key, &mut out) || !encode_row(row, &mut out) {
                    return None;
                }
            }
        }
        put_u64(&mut out, self.next_guid);
        put_i64(&mut out, self.next_row_version);
        put_i64(&mut out, self.committed_row_version);
        put_i64(&mut out, self.last_stamp);
        fn sorted<K: Ord + Clone, V: Clone>(m: &std::collections::HashMap<K, V>) -> Vec<(K, V)> {
            let mut v: Vec<(K, V)> = m.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            v.sort_by(|a, b| a.0.cmp(&b.0));
            v
        }
        let autoinc = sorted(&self.autoinc);
        put_u32(&mut out, autoinc.len() as u32);
        for ((company, table), last) in autoinc {
            put_str(&mut out, &company);
            put_u32(&mut out, table);
            put_i64(&mut out, last);
        }
        let storage = sorted(&self.isolated_storage);
        put_u32(&mut out, storage.len() as u32);
        for ((company, key), (value, encrypted, secret)) in storage {
            put_str(&mut out, &company);
            put_str(&mut out, &key);
            put_str(&mut out, &value);
            // Bit 0: stored encrypted; bit 1: stored from a SecretText.
            put_u8(&mut out, encrypted as u8 | (secret as u8) << 1);
        }
        let sequences = sorted(&self.sequences);
        put_u32(&mut out, sequences.len() as u32);
        for ((company, name), q) in sequences {
            put_str(&mut out, &company);
            put_str(&mut out, &name);
            put_i64(&mut out, q.increment);
            put_i64(&mut out, q.seed);
            match q.last {
                Some(v) => {
                    put_u8(&mut out, 1);
                    put_i64(&mut out, v);
                }
                None => put_u8(&mut out, 0),
            }
        }
        match &self.workspace_installs {
            None => put_u8(&mut out, 0),
            Some(list) => {
                put_u8(&mut out, 1);
                put_u32(&mut out, list.len() as u32);
                for (tables, why) in list {
                    put_u32(&mut out, tables.len() as u32);
                    for t in tables {
                        put_u32(&mut out, *t);
                    }
                    put_str(&mut out, why);
                }
            }
        }
        Some(out)
    }

    /// The baseline [`Baseline::to_bytes`] wrote; `None` for anything else (other layout, cut short, bytes left over).
    pub fn from_bytes(bytes: &[u8]) -> Option<Baseline> {
        use al_builtins::value_codec::*;
        let mut r = Reader::new(bytes.strip_prefix(&MAGIC[..])?);
        let n_tables = r.u32()? as usize;
        let mut tables = Vec::with_capacity(n_tables.min(4096));
        for _ in 0..n_tables {
            let company = r.str()?;
            let object = r.u32()?;
            let n_rows = r.u32()? as usize;
            let mut rows = Vec::with_capacity(n_rows.min(1 << 16));
            for _ in 0..n_rows {
                let key = decode_row(&mut r)?;
                let row = decode_row(&mut r)?;
                rows.push((key, row));
            }
            tables.push(al_data::memory::ExportedTable { company, object, rows });
        }
        let next_guid = r.u64()?;
        let next_row_version = r.i64()?;
        let committed_row_version = r.i64()?;
        let last_stamp = r.i64()?;
        let mut autoinc = std::collections::HashMap::new();
        for _ in 0..r.u32()? {
            let company = r.str()?;
            let table = r.u32()?;
            autoinc.insert((company, table), r.i64()?);
        }
        let mut isolated_storage = std::collections::HashMap::new();
        for _ in 0..r.u32()? {
            let company = r.str()?;
            let key = r.str()?;
            let value = r.str()?;
            let flags = r.u8()?;
            isolated_storage.insert((company, key), (value, flags & 1 != 0, flags & 2 != 0));
        }
        let mut sequences = std::collections::HashMap::new();
        for _ in 0..r.u32()? {
            let company = r.str()?;
            let name = r.str()?;
            let increment = r.i64()?;
            let seed = r.i64()?;
            let last = match r.u8()? {
                0 => None,
                _ => Some(r.i64()?),
            };
            sequences.insert((company, name), crate::sessionops::Sequence { increment, seed, last });
        }
        let workspace_installs = match r.u8()? {
            0 => None,
            _ => {
                let mut list = Vec::new();
                for _ in 0..r.u32()? {
                    let mut set = std::collections::BTreeSet::new();
                    for _ in 0..r.u32()? {
                        set.insert(r.u32()?);
                    }
                    list.push((set, r.str()?));
                }
                Some(list)
            }
        };
        if !r.is_empty() {
            return None;
        }
        Some(Baseline {
            store: al_data::memory::MemStore::from_exported(tables),
            next_guid,
            next_row_version,
            committed_row_version,
            autoinc,
            isolated_storage,
            sequences,
            last_stamp,
            workspace_installs,
        })
    }

    /// This baseline over the test data of `engine`, for a baseline read from bytes: the bytes hold the tables the
    /// install code wrote to, and every other table reads the restored database through the engine, as it did
    /// in the run that made them. An engine without test data leaves the baseline as it is.
    pub fn backed_by(mut self, engine: &std::sync::Arc<crate::Engine>) -> Baseline {
        if let Some(td) = &engine.test_data {
            self.store = self
                .store
                .with_backing(std::rc::Rc::new(crate::testdata::TestBacking::new(
                    engine.clone(),
                    td.clone(),
                )));
        }
        self
    }

    /// The table objects that hold a row in the baseline.
    pub fn nonempty_tables(&self) -> std::collections::BTreeSet<u32> {
        self.store.nonempty_tables()
    }
}

impl Session {
    /// Whether a restored company (test data, ADR-020) has rows for table `object` in the session
    /// company. Reading it loads the table.
    pub fn backs_rows(&self, object: u32) -> bool {
        // A company-independent table (DataPerCompany = false) is stored under no company.
        let own = self.engine.tables.get(object as usize).and_then(|t| t.as_ref());
        let company = if own.is_some_and(|t| !t.per_company) {
            ""
        } else {
            &*self.cx.company
        };
        self.store.backed_rows(company, object).is_some_and(|r| !r.is_empty())
    }

    /// Commits and captures the session's data.
    pub fn capture_baseline(&mut self) -> Baseline {
        self.commit_transaction();
        Baseline {
            store: self.store.clone().detached(),
            next_guid: self.next_guid,
            next_row_version: self.next_row_version,
            committed_row_version: self.tx.committed_row_version,
            autoinc: self.autoinc.clone(),
            isolated_storage: self.isolated_storage.clone(),
            sequences: self.sequences.clone(),
            last_stamp: self.last_stamp,
            workspace_installs: None,
        }
    }

    /// Chooses the company the session works in; the empty name is no company, as in
    /// `OnInstallAppPerDatabase`, where `CompanyName()` is empty and a per-company table cannot
    /// be accessed.
    pub fn set_company(&mut self, company: &str) -> String {
        let old = std::mem::replace(&mut self.cx.company, company.into()).to_string();
        if old != company && !self.events.global_setups.is_empty() {
            // UNPROBED (assumption database-trigger-setup-per-company): Microsoft's setup subscribers answer by
            // CompanyName(), so what was asked in one company is not reused in another.
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::DatabaseTriggerSetupPerCompany);
            self.events.global_setups.clear();
        }
        old
    }

    /// The isolated storage and the number sequences of the session now differ from those of `start` (state a later
    /// session sees besides the tables).
    pub fn storage_or_sequences_differ(&self, start: &Baseline) -> bool {
        self.isolated_storage != start.isolated_storage || self.sequences != start.sequences
    }

    /// Replaces the session's data with the baseline's (a new session starts here).
    pub fn restore_baseline(&mut self, b: &Baseline) {
        let provider = self.store.virtual_tables();
        self.store = b.store.clone().attached(provider);
        self.next_guid = b.next_guid;
        self.next_row_version = b.next_row_version;
        self.tx.committed_row_version = b.committed_row_version;
        self.autoinc = b.autoinc.clone();
        self.isolated_storage = b.isolated_storage.clone();
        self.isolated_committed = b.isolated_storage.clone();
        self.sequences = b.sequences.clone();
        // The install ran on a clock that its own Sleep calls had moved forward (hours, in Business Central's install
        // code); the session starts on the real clock again, and a stamp must not stay ahead of it: a row inserted by the
        // first test would otherwise be stamped hours after CurrentDateTime (corpus record/TestSystemFieldStampsContracts.al:
        // Insert_StampsBothFieldsWithTheUtcClock_SystemIdIsSetBeforeOnInsert).
        self.last_stamp = b.last_stamp.min(self.now_millis());
        self.baseline_rows = b.nonempty_tables();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_data::memory::ExportedTable;

    fn sample() -> Baseline {
        let table = ExportedTable {
            company: "CRONUS".into(),
            object: 311,
            rows: vec![(
                vec![al_builtins::value::Value::Code("".into())],
                vec![
                    al_builtins::value::Value::Code("".into()),
                    al_builtins::value::Value::Integer(7),
                ],
            )],
        };
        Baseline {
            store: al_data::memory::MemStore::from_exported(vec![table]),
            next_guid: 41,
            next_row_version: 1_000_000_007,
            committed_row_version: 1_000_000_001,
            autoinc: [(("CRONUS".to_string(), 311), 12_i64)].into_iter().collect(),
            isolated_storage: [(
                ("CRONUS".to_string(), "key".to_string()),
                ("secret".to_string(), true, true),
            )]
            .into_iter()
            .collect(),
            sequences: [(
                ("".to_string(), "Seq".to_string()),
                crate::sessionops::Sequence {
                    increment: 2,
                    seed: 5,
                    last: Some(9),
                },
            )]
            .into_iter()
            .collect(),
            last_stamp: 639_277_389_274_520,
            workspace_installs: Some(vec![([50000_u32, 50001].into_iter().collect(), "why".to_string())]),
        }
    }

    /// Everything a session takes from a baseline, as text.
    fn summary(b: &Baseline) -> String {
        let rows = b.store.export_committed().expect("no backing");
        format!(
            "{rows:?} {} {} {} {:?} {:?} {:?} {} {:?}",
            b.next_guid,
            b.next_row_version,
            b.committed_row_version,
            b.autoinc.iter().collect::<std::collections::BTreeMap<_, _>>(),
            b.isolated_storage.iter().collect::<std::collections::BTreeMap<_, _>>(),
            b.sequences.iter().collect::<std::collections::BTreeMap<_, _>>(),
            b.last_stamp,
            b.workspace_installs
        )
    }

    #[test]
    fn a_baseline_survives_a_round_trip_through_bytes() {
        let b = sample();
        let bytes = b.to_bytes().expect("rows are plain values");
        let back = Baseline::from_bytes(&bytes).expect("what to_bytes wrote reads back");
        assert_eq!(summary(&back), summary(&b));
        assert_eq!(back.to_bytes().as_deref(), Some(&bytes[..]), "the bytes are stable");
    }

    #[test]
    fn a_baseline_without_workspace_installs_reads_back_as_none() {
        let mut b = sample();
        b.workspace_installs = None;
        let back = Baseline::from_bytes(&b.to_bytes().unwrap()).unwrap();
        assert!(back.workspace_installs.is_none());
    }

    #[test]
    fn damaged_or_foreign_bytes_give_nothing() {
        let bytes = sample().to_bytes().unwrap();
        for cut in [0, 1, 4, bytes.len() / 2, bytes.len() - 1] {
            assert!(Baseline::from_bytes(&bytes[..cut]).is_none(), "cut at {cut}");
        }
        let mut other = bytes.clone();
        other[0] ^= 0xFF;
        assert!(Baseline::from_bytes(&other).is_none(), "a different magic");
        let mut longer = bytes.clone();
        longer.push(0);
        assert!(Baseline::from_bytes(&longer).is_none(), "bytes left over");
    }

    #[test]
    fn a_baseline_over_test_data_has_bytes_of_the_tables_it_holds() {
        struct B;
        impl al_data::memory::Backing for B {
            fn company(&self) -> &str {
                "C"
            }
            fn rows(&self, _: &str, _: u32) -> Option<al_data::memory::Rows> {
                None
            }
        }
        let mut b = sample();
        b.store = b.store.with_backing(std::rc::Rc::new(B));
        let bytes = b.to_bytes().expect("the held tables are plain rows");
        let back = Baseline::from_bytes(&bytes).unwrap();
        assert_eq!(summary(&back), summary(&b));
    }

    // corpus record/TestSystemFieldStampsContracts.al:Insert_StampsBothFieldsWithTheUtcClock_SystemIdIsSetBeforeOnInsert:
    // the install ran on a clock its Sleep calls had moved forward; a session restored from its baseline stamps on the real clock.
    #[test]
    fn a_restored_session_does_not_stamp_ahead_of_the_clock() {
        let engine = crate::Engine::new(al_ir::Program::default(), Vec::new());
        let mut session = Session::new(engine);
        let mut b = sample();
        b.last_stamp = session.now_millis() + 6 * 3_600_000;
        session.restore_baseline(&b);
        let stamp = session.stamp_millis();
        assert!(
            stamp <= session.now_millis() + 1_000,
            "stamped {} ms ahead",
            stamp - session.now_millis()
        );
        // A baseline stamp in the past still holds: the next stamp is later than it.
        let mut older = sample();
        older.last_stamp = session.now_millis() - 10_000;
        session.restore_baseline(&older);
        assert!(session.stamp_millis() > older.last_stamp);
    }
}
