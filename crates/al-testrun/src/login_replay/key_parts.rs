//! The parts of a key's `database` and `session` digests, each with a SHA-256 digest of its own, so the keys line can say
//! which parts a new key differs in from its nearest earlier key, by name only (issue 8211: a key field alone, such as
//! `session`, does not say which state moved). The parts:
//! - per company and table object, the committed rows of that table: named `company N/table object T`, where N numbers
//!   the company by first sight over the whole check ([`Companies`]: a company name can be a customer's, so it is never
//!   shown), or `global/table object T` for a table that is not per company;
//! - per state line name, the session state lines of that name (`Session::state_lines`: every line starts with a name the
//!   runtime writes, `session`, `single`, `counters`, ...: its first word). A session that holds a value with no
//!   comparable form has no key (`Key::of`).
//!
//! A table the store exports without rows has no part and no digest ([`store_table_digests`]): the key holds rows, and
//! such an entry holds none, so the whole `database` digest does not see it either, and the keys line never names it.
//! Whether a table has an entry is the replay's base shape (al-vm `BaseShape`), which the start's gate compares apart
//! from the key.
//!
//! The parts are never compared: a key holds them as [`NotCompared`], so which keys are equal is decided by the key's
//! fields alone (the maintainer's review of this check, issue 8211).

use super::rows_digest::{NoByteForm, table_digest as rows_table_digest};
use super::{LinesDigest, digest};
use std::collections::{BTreeMap, BTreeSet};

/// A detail a key carries beside its fields, never compared: two keys are equal by their fields alone, whatever their
/// details hold.
#[derive(Debug, Clone, Default)]
pub(crate) struct NotCompared<T>(pub(crate) T);

impl<T> PartialEq for NotCompared<T> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl<T> Eq for NotCompared<T> {}

/// The parts of a key (see the module comment).
#[derive(Debug, Clone, Default)]
pub(crate) struct KeyParts {
    /// Per (company, table object), the digest of its committed rows (`rows_digest::table_digest`).
    database: BTreeMap<(String, u32), LinesDigest>,
    /// Per state line name, the digest of the lines of that name, in their order.
    session: BTreeMap<String, LinesDigest>,
}

/// Per (company, table object), the digest of the committed rows of each of `tables` that holds a row
/// (`rows_digest::table_digest`); every row of `tables` has a byte form.
#[cfg(test)]
pub(crate) fn table_digests(tables: &[al_data::memory::ExportedTable]) -> BTreeMap<(String, u32), LinesDigest> {
    (tables.iter().filter(|t| !t.rows.is_empty()))
        .map(|t| {
            let rows = t.rows.iter().map(|(_, row)| row);
            let d = rows_table_digest(&t.company, t.object, t.rows.len(), rows);
            ((t.company.clone(), t.object), d.expect("a row without a byte form"))
        })
        .collect()
}

/// Per (company, table object), the digest of the committed rows of each table of `store` that holds a row
/// (`rows_digest::table_digest`), each made once per state of its table: a table's stamp names one state of it in every
/// store of the process (al-data `MemStore::committed_rows`), so a digest kept for a stamp is the digest of the rows the
/// table holds at that stamp, and a later session hashes only the tables that changed. A table with no stamp is hashed
/// each time. Kept per engine (`Engine::derived`). A table entry without rows has no digest (the module comment).
/// [`NoByteForm`] when a row holds a value with no byte form (nothing is kept for its table).
pub(crate) fn store_table_digests(
    engine: &al_vm::Engine,
    store: &al_data::memory::MemStore,
) -> Result<BTreeMap<(String, u32), LinesDigest>, NoByteForm> {
    #[derive(Default)]
    struct Memo(std::sync::Mutex<std::collections::HashMap<(String, u32, u64), LinesDigest>>);
    /// How many table states the memo keeps before it starts again.
    const KEPT: usize = 1 << 16;
    let memo = engine.derived(Memo::default);
    let mut out = BTreeMap::new();
    for entry in store.committed_rows() {
        // A table without rows has no digest, so a known stamp is one of a table with rows.
        if entry.is_empty() {
            continue;
        }
        let (company, object, stamp) = (entry.company, entry.object, entry.stamp);
        let known = (stamp != 0)
            .then(|| {
                (memo.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
                    .get(&(company.to_string(), object, stamp))
                    .copied()
            })
            .flatten();
        let d = match known {
            Some(d) => d,
            None => {
                // By reference: no copy of the table's rows.
                let d = rows_table_digest(company, object, entry.len(), entry.rows())?;
                if stamp != 0 {
                    let mut m = memo.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    if m.len() >= KEPT {
                        m.clear();
                    }
                    m.insert((company.to_string(), object, stamp), d);
                }
                d
            }
        };
        out.insert((company.to_string(), object), d);
    }
    Ok(out)
}

/// The `database` digest of a key: the digest of the per-table digests `tables`, each with its company and table.
pub(crate) fn database_digest(tables: &BTreeMap<(String, u32), LinesDigest>) -> LinesDigest {
    let lines: Vec<String> = (tables.iter())
        .map(|((company, object), d)| format!("{company}|{object}|{}", d.hex()))
        .collect();
    digest(&lines)
}

impl KeyParts {
    /// The parts of the committed rows (per table, [`store_table_digests`]) and the state `state`.
    pub(crate) fn of(database: BTreeMap<(String, u32), LinesDigest>, state: &al_vm::StateLines) -> KeyParts {
        let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for line in &state.lines {
            let name = line.split(' ').next().unwrap_or_default();
            by_name.entry(name.to_string()).or_default().push(line.clone());
        }
        let session = (by_name.into_iter())
            .map(|(name, lines)| (name, digest(&lines)))
            .collect();
        KeyParts { database, session }
    }

    /// The tables whose committed rows differ between `self` and `other` (one has rows the other has not, or other rows),
    /// named by `companies`.
    pub(crate) fn database_differs(&self, other: &KeyParts, companies: &mut Companies) -> Vec<String> {
        differing(&self.database, &other.database)
            .into_iter()
            .map(|(company, table)| companies.table(company, *table))
            .collect()
    }

    /// The names of the session state lines that differ between `self` and `other`.
    pub(crate) fn session_differs(&self, other: &KeyParts) -> Vec<String> {
        differing(&self.session, &other.session).into_iter().cloned().collect()
    }
}

/// The companies the keys line names, numbered by first sight over one check, so one number means one company in every
/// entry of the line. The names stay here: a company name can be a customer's, so the line shows only the number.
#[derive(Default)]
pub(crate) struct Companies(Vec<String>);

impl Companies {
    /// `table object T` of `company`: `company N/table object T`, or `global/table object T` for a table that is not per
    /// company (the empty company of `ExportedTable`).
    fn table(&mut self, company: &str, table: u32) -> String {
        if company.is_empty() {
            return format!("global/table object {table}");
        }
        let n = match self.0.iter().position(|c| c == company) {
            Some(p) => p + 1,
            None => {
                self.0.push(company.to_string());
                self.0.len()
            }
        };
        format!("company {n}/table object {table}")
    }
}

/// The keys of `a` and `b` whose digests differ, one of the two having no entry included, in key order.
fn differing<'a, K: Ord>(a: &'a BTreeMap<K, LinesDigest>, b: &'a BTreeMap<K, LinesDigest>) -> Vec<&'a K> {
    let keys: BTreeSet<&K> = a.keys().chain(b.keys()).collect();
    keys.into_iter().filter(|k| a.get(*k) != b.get(*k)).collect()
}
