//! The parts of a key's `database` and `session` digests, each with a SHA-256 digest of its own, so the keys line can say
//! which parts a new key differs in from its nearest earlier key, by name only (issue 8211: a key field alone, such as
//! `session`, does not say which state moved). The parts:
//! - per company and table object, the committed rows of that table: named `company N/table T`, where N is the company's
//!   place among the companies of the two keys (a company name can be a customer's, so it is never shown);
//! - per state line name, the session state lines of that name (`Session::state_lines`: every line starts with a name the
//!   runtime writes, `session`, `single`, `counters`, ...: its first word), and the lines with no comparable form as
//!   `not comparable`.
//!
//! A table the store exports without rows has no part: `rows` writes no line for it, so the whole `database` digest does
//! not see it either, and the keys line never names it.
//!
//! The parts are never compared: a key holds them as [`NotCompared`], so which keys are equal is decided by the key's
//! fields alone (the maintainer's review of this check, issue 8211).

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
    /// Per (company, table object), the digest of its committed row lines.
    database: BTreeMap<(String, u32), LinesDigest>,
    /// Per state line name, the digest of the lines of that name, in their order.
    session: BTreeMap<String, LinesDigest>,
}

impl KeyParts {
    /// The parts of the committed rows `tables` (one line per row, as `rows` writes them) and the state `state`.
    pub(crate) fn of(tables: &[al_data::memory::ExportedTable], state: &al_vm::StateLines) -> KeyParts {
        let database = (tables.iter().filter(|t| !t.rows.is_empty()))
            .map(|t| {
                (
                    (t.company.clone(), t.object),
                    digest(&super::rows(std::slice::from_ref(t))),
                )
            })
            .collect();
        let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for line in &state.lines {
            let name = line.split(' ').next().unwrap_or_default();
            by_name.entry(name.to_string()).or_default().push(line.clone());
        }
        if !state.opaque.is_empty() {
            by_name.insert("not comparable".into(), state.opaque.clone());
        }
        let session = (by_name.into_iter())
            .map(|(name, lines)| (name, digest(&lines)))
            .collect();
        KeyParts { database, session }
    }

    /// The tables whose committed rows differ between `self` and `other` (one has rows the other has not, or other rows),
    /// as `company N/table T`.
    pub(crate) fn database_differs(&self, other: &KeyParts) -> Vec<String> {
        let companies: BTreeSet<&String> = (self.database.keys().chain(other.database.keys()))
            .map(|(c, _)| c)
            .collect();
        let place = |c: &String| companies.iter().position(|x| *x == c).map_or(0, |p| p + 1);
        differing(&self.database, &other.database)
            .into_iter()
            .map(|(company, table)| format!("company {}/table {table}", place(company)))
            .collect()
    }

    /// The names of the session state lines that differ between `self` and `other`.
    pub(crate) fn session_differs(&self, other: &KeyParts) -> Vec<String> {
        differing(&self.session, &other.session).into_iter().cloned().collect()
    }
}

/// The keys of `a` and `b` whose digests differ, one of the two having no entry included, in key order.
fn differing<'a, K: Ord>(a: &'a BTreeMap<K, LinesDigest>, b: &'a BTreeMap<K, LinesDigest>) -> Vec<&'a K> {
    let keys: BTreeSet<&K> = a.keys().chain(b.keys()).collect();
    keys.into_iter().filter(|k| a.get(*k) != b.get(*k)).collect()
}
