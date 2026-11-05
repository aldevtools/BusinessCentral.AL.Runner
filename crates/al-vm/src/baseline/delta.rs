//! What a run changed in the baseline it started from ([`Delta`]), to predict where a later run starts (the carry
//! chain's runs ahead, al-testrun `chain_walk.rs`), and a baseline's [`Baseline::fingerprint`], which decides whether
//! two baselines start a session the same way.

use super::{Baseline, SequenceSteps};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// A baseline's [`Baseline::fingerprint`], with the rows of some tables apart ([`Baseline::fingerprint_apart`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    /// Everything but the rows of the tables apart.
    pub main: Vec<u8>,
    /// The rows of the tables apart.
    pub rows: Vec<u8>,
}

impl Fingerprint {
    /// Both parts as one value: equal for two baselines exactly when both parts are.
    pub fn whole(mut self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.main.len() + self.rows.len() + 4);
        al_builtins::value_codec::put_u32(&mut out, self.main.len() as u32);
        out.append(&mut self.main);
        out.append(&mut self.rows);
        out
    }
}

/// The parts of a baseline a delta sets as they are, not by difference.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Marks {
    last_stamp: i64,
    workspace_installs: Option<Vec<(BTreeSet<u32>, String)>>,
    company_built: bool,
    install_rows: Option<BTreeSet<u32>>,
    company_unknown: Vec<(String, u32, String)>,
    full_copies: Vec<String>,
}

/// A table's rows, encoded, and whether a DeleteAll emptied it.
type EncodedTable = (Arc<Vec<u8>>, bool);

/// What a run changed in the baseline it started from ([`Baseline::delta_from`]): the tables it wrote, as their rows
/// after it (encoded, so a delta can go to another thread), the counters and number sequences by how far they moved on,
/// and the AutoIncrement counters and the marks as they are after it. Applied to another baseline ([`Baseline::with_delta`]) it predicts what the run
/// would leave there; a prediction is only ever checked, by [`Baseline::fingerprint`], never trusted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Delta {
    /// (company, object) -> the encoded rows and whether a DeleteAll emptied the table.
    tables: BTreeMap<(String, u32), EncodedTable>,
    gone: BTreeSet<(String, u32)>,
    next_guid: u64,
    next_row_version: i64,
    committed_row_version: i64,
    /// The AutoIncrement counters it changed, as they are after it.
    autoinc: BTreeMap<(String, u32), i64>,
    steps: SequenceSteps,
    marks: Option<Marks>,
}

impl Delta {
    /// This delta followed by `then`.
    pub fn then(&self, then: &Delta) -> Delta {
        let mut d = self.clone();
        for k in &then.gone {
            d.tables.remove(k);
            d.gone.insert(k.clone());
        }
        for (k, v) in &then.tables {
            d.gone.remove(k);
            d.tables.insert(k.clone(), v.clone());
        }
        d.next_guid = d.next_guid.wrapping_add(then.next_guid);
        d.next_row_version = d.next_row_version.wrapping_add(then.next_row_version);
        d.committed_row_version = d.committed_row_version.wrapping_add(then.committed_row_version);
        d.autoinc.extend(then.autoinc.iter().map(|(k, v)| (k.clone(), *v)));
        for (k, n) in &then.steps {
            *d.steps.entry(k.clone()).or_default() += n;
        }
        if then.marks.is_some() {
            d.marks = then.marks.clone();
        }
        d
    }
}

impl Baseline {
    /// What changed from `start` to this baseline, when this one is `start` restored in a session, written and captured
    /// (directly or through such steps and [`Baseline::with_delta`]): the tables whose stamps moved, with their rows;
    /// the counters and sequences by how far they moved on; the AutoIncrement counters it changed and the marks as they are. `None` when it
    /// is not such a change: isolated storage, company copies or the set of sequences or AutoIncrement tables differ, a
    /// sequence moved otherwise than by `Next`, or a row is not a plain value.
    pub fn delta_from(&self, start: &Baseline) -> Option<Delta> {
        if self.isolated_storage != start.isolated_storage {
            return None;
        }
        let steps = self.steps_since(start)?;
        let (changed, gone) = self.store.committed_changes_from(&start.store)?;
        let mut tables = BTreeMap::new();
        for (company, object) in changed {
            let (t, emptied) = self.store.committed_table(&company, object)?;
            let mut out = Vec::new();
            al_builtins::value_codec::put_u32(&mut out, t.rows.len() as u32);
            for (key, row) in &t.rows {
                if !al_builtins::value_codec::encode_row(key, &mut out)
                    || !al_builtins::value_codec::encode_row(row, &mut out)
                {
                    return None;
                }
            }
            tables.insert((company, object), (Arc::new(out), emptied));
        }
        if start.autoinc.keys().any(|k| !self.autoinc.contains_key(k)) {
            return None;
        }
        // As they are: a key the caller chose raises a counter to it rather than moving it on (`autoinc.rs`
        // `note_explicit_key`), as the Test Runner's lines do, so a difference would predict worse.
        let autoinc = self
            .autoinc
            .iter()
            .filter(|(k, v)| start.autoinc.get(*k) != Some(*v))
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        Some(Delta {
            tables,
            gone: gone.into_iter().collect(),
            next_guid: self.next_guid.wrapping_sub(start.next_guid),
            next_row_version: self.next_row_version.wrapping_sub(start.next_row_version),
            committed_row_version: self.committed_row_version.wrapping_sub(start.committed_row_version),
            autoinc,
            steps,
            marks: Some(Marks {
                last_stamp: self.last_stamp,
                workspace_installs: self.workspace_installs.clone(),
                company_built: self.company_built,
                install_rows: self.install_rows.clone(),
                company_unknown: self.company_unknown.clone(),
                full_copies: self.full_copies.clone(),
            }),
        })
    }

    /// This baseline with `delta` applied ([`Baseline::delta_from`]), as a baseline of its own (a new origin). `None`
    /// when the delta does not apply: a sequence it moves on is not held, or its rows do not read back.
    pub fn with_delta(&self, delta: &Delta) -> Option<Baseline> {
        let mut b = self.clone();
        b.sequences = self.stepped(&delta.steps)?.sequences;
        let mut tables = Vec::with_capacity(delta.tables.len());
        for ((company, object), (bytes, emptied)) in &delta.tables {
            let mut r = al_builtins::value_codec::Reader::new(bytes);
            let n = r.u32()? as usize;
            let mut rows = Vec::with_capacity(n.min(1 << 16));
            for _ in 0..n {
                let key = al_builtins::value_codec::decode_row(&mut r)?;
                rows.push((key, al_builtins::value_codec::decode_row(&mut r)?));
            }
            if !r.is_empty() {
                return None;
            }
            let t = al_data::memory::ExportedTable {
                company: company.clone(),
                object: *object,
                rows,
            };
            tables.push((t, *emptied));
        }
        let gone: Vec<(String, u32)> = delta.gone.iter().cloned().collect();
        b.store = b.store.with_committed_tables(tables, &gone);
        b.next_guid = b.next_guid.wrapping_add(delta.next_guid);
        b.next_row_version = b.next_row_version.wrapping_add(delta.next_row_version);
        b.committed_row_version = b.committed_row_version.wrapping_add(delta.committed_row_version);
        b.autoinc.extend(delta.autoinc.iter().map(|(k, v)| (k.clone(), *v)));
        if let Some(m) = &delta.marks {
            b.last_stamp = m.last_stamp;
            b.workspace_installs = m.workspace_installs.clone();
            b.company_built = m.company_built;
            b.install_rows = m.install_rows.clone();
            b.company_unknown = m.company_unknown.clone();
            b.full_copies = m.full_copies.clone();
        }
        b.origin = super::new_origin();
        Some(b)
    }

    /// Everything a session restored from this baseline starts from, as bytes: two baselines with the same fingerprint
    /// start a session the same way ([`Session::restore_baseline`](crate::Session::restore_baseline) reads nothing
    /// else). The bytes of [`Baseline::to_bytes`] with the last stamp left out (a session starts on the clock, and a
    /// stamp only keeps the next one later: it is not the same in two runs of anything), then what they leave out: the
    /// install rows as a session takes them, the tables a DeleteAll emptied and the company copies. `None` when a row is
    /// not a plain value.
    pub fn fingerprint(&self) -> Option<Vec<u8>> {
        Some(self.fingerprint_apart(&[])?.whole())
    }

    /// The [`Baseline::fingerprint`] in two parts: everything but the committed rows of the table objects `apart` (in
    /// any company), and those rows. Two baselines whose first parts are equal start a session the same way but for
    /// the rows of `apart`: a run that empties those tables before anything reads them starts the same way from both
    /// (al-testrun `suite_rows.rs`, the Test Runner's tables). `None` when a row is not a plain value.
    pub fn fingerprint_apart(&self, apart: &[u32]) -> Option<Fingerprint> {
        use al_builtins::value_codec::*;
        let mut b = self.clone();
        let mut rows = Vec::new();
        let mut gone = Vec::new();
        for &object in apart {
            for company in self.store.committed_companies_of(object) {
                let (t, emptied) = self.store.committed_table(&company, object)?;
                put_str(&mut rows, &company);
                put_u32(&mut rows, object);
                rows.push(emptied as u8);
                put_u32(&mut rows, t.rows.len() as u32);
                for (key, row) in &t.rows {
                    if !encode_row(key, &mut rows) || !encode_row(row, &mut rows) {
                        return None;
                    }
                }
                gone.push((company, object));
            }
        }
        if !gone.is_empty() {
            b.store = b.store.with_committed_tables(Vec::new(), &gone);
            if let Some(set) = b.install_rows.as_mut() {
                set.retain(|t| !apart.contains(t));
            }
        }
        Some(Fingerprint {
            main: b.fingerprint_bytes()?,
            rows,
        })
    }

    fn fingerprint_bytes(mut self) -> Option<Vec<u8>> {
        use al_builtins::value_codec::*;
        self.last_stamp = 0;
        let mut out = self.to_bytes()?;
        let rows = self.install_rows.clone().unwrap_or_else(|| self.nonempty_tables());
        put_u32(&mut out, rows.len() as u32);
        for t in rows {
            put_u32(&mut out, t);
        }
        let (emptied, copies) = self.store.committed_marks();
        put_u32(&mut out, emptied.len() as u32);
        for (company, object) in emptied {
            put_str(&mut out, &company);
            put_u32(&mut out, object);
        }
        put_u32(&mut out, copies.len() as u32);
        for (copy, from) in copies {
            put_str(&mut out, &copy);
            put_str(&mut out, &from);
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::sample;
    use al_builtins::value::Value;

    #[test]
    fn a_delta_applied_to_its_start_gives_the_same_fingerprint_and_predicts_a_later_start() {
        let start = sample();
        let key = ("".to_string(), "Seq".to_string());
        let mut later = start.stepped(&[(key.clone(), 2)].into_iter().collect()).unwrap();
        later.next_guid += 3;
        later.store = later.store.clone().with_committed_tables(
            vec![(
                al_data::memory::ExportedTable {
                    company: "CRONUS".into(),
                    object: 130450,
                    rows: vec![(vec![Value::Integer(1)], vec![Value::Integer(1), Value::Integer(5)])],
                },
                true,
            )],
            &[],
        );
        let d = later.delta_from(&start).expect("only plain changes");
        let again = start.with_delta(&d).unwrap();
        assert_eq!(again.fingerprint(), later.fingerprint());
        assert_ne!(start.fingerprint(), later.fingerprint());
        // Twice the change: what a second run like it would leave.
        let twice = again.with_delta(&d).unwrap();
        assert_eq!(
            start.with_delta(&d.then(&d)).unwrap().fingerprint(),
            twice.fingerprint()
        );
        assert_eq!(twice.next_guid, start.next_guid + 6);
        assert_eq!(twice.delta_from(&start).unwrap().steps[&key], 4);
        // Isolated storage is not predicted.
        let mut stored = later.clone();
        stored.isolated_storage.clear();
        assert!(stored.delta_from(&start).is_none());
    }

    #[test]
    fn a_fingerprint_with_a_table_apart_compares_the_rest_alone() {
        let start = sample();
        let rows = |n: i32| {
            vec![(
                al_data::memory::ExportedTable {
                    company: "CRONUS".into(),
                    object: 130450,
                    rows: vec![(vec![Value::Integer(n)], vec![Value::Integer(n), Value::Integer(5)])],
                },
                false,
            )]
        };
        let mut a = start.with_delta(&Default::default()).unwrap();
        a.store = a.store.clone().with_committed_tables(rows(1), &[]);
        let mut b = start.clone();
        b.store = b.store.clone().with_committed_tables(rows(2), &[]);
        assert_ne!(a.fingerprint(), b.fingerprint());
        let (fa, fb) = (
            a.fingerprint_apart(&[130450]).unwrap(),
            b.fingerprint_apart(&[130450]).unwrap(),
        );
        assert_eq!(fa.main, fb.main, "only the rows apart differ");
        assert_ne!(fa.rows, fb.rows);
        // Without the table at all the rest is the same too; another table's rows are not apart.
        assert_eq!(start.fingerprint_apart(&[130450]).unwrap().main, fa.main);
        assert_eq!(start.fingerprint_apart(&[130450]).unwrap().rows, Vec::<u8>::new());
        assert_eq!(a.fingerprint_apart(&[]).unwrap().whole(), a.fingerprint().unwrap());
        assert_ne!(start.fingerprint_apart(&[311]).unwrap().main, fa.main);
    }
}
