//! What the runner needs to install two partner apps in both orders and compare the result (ADR-046 amendment 7, owner
//! 2026-11-08, al-runner issue 6644): which pairs of apps with no known install order the install code reached, a rewind
//! to the database before the apps installed, and the comparison of the databases two orders leave.

use super::Baseline;
use crate::Session;
use std::collections::BTreeSet;

/// The system fields, SystemId (2000000000) to SystemRowVersion (2000000005): BC gives them new values on every install,
/// whatever the order, so the comparison leaves them out.
const SYSTEM_FIELDS: std::ops::RangeInclusive<u32> = 2_000_000_000..=2_000_000_005;

impl Session {
    /// Notes that the install code of the installing app reached code of app `other`, whose install order to it is not
    /// known (the refusals of `Session::set_apps_install_order_unknown`). [`Session::take_order_unknown_hits`] reads them.
    pub fn note_order_unknown_hit(&self, other: u32) {
        let installing = self.events.event_stats.install_view.borrow().installing;
        if let Some(installing) = installing {
            self.events.order_unknown_hits.borrow_mut().insert((installing, other));
        }
    }

    /// The pairs (installing app, other app) noted since the last call, and forgets them.
    pub fn take_order_unknown_hits(&mut self) -> BTreeSet<(u32, u32)> {
        std::mem::take(&mut *self.events.order_unknown_hits.borrow_mut())
    }

    /// Sets how far the session's clock moved on besides the real clock ([`Session::clock_offset_ms`]), so that a second
    /// install order starts on the clock the first one started on.
    pub fn set_clock_offset_ms(&mut self, ms: i64) {
        self.waited_ms = ms;
    }

    /// Puts the data of `b`, a capture taken during the same install run, back into the session, so that the install code
    /// can run again from there in another order. Unlike [`Session::restore_baseline`] it keeps what the session holds
    /// about the baseline it started from: the install run did not start from `b`.
    pub fn rewind_install(&mut self, b: &Baseline) {
        let rows = std::mem::take(&mut self.baseline_rows);
        let origin = self.baseline_origin;
        let company_built = self.company_built;
        self.restore_baseline(b);
        self.baseline_rows = rows;
        self.baseline_origin = origin;
        self.company_built = company_built;
    }
}

impl Baseline {
    /// The tables (object ids) whose committed rows differ between `self` and `other`, two captures of install runs that
    /// both started from `start`, the system fields left out. A table counts as differing also when its AutoIncrement
    /// counter does. `None` when something besides the tables differs (isolated storage, number sequences, the
    /// companies not known or copied), or a row is not a plain value: the caller cannot name what differs by table.
    pub fn install_order_differences(
        &self,
        other: &Baseline,
        start: &Baseline,
        engine: &crate::Engine,
    ) -> Option<BTreeSet<u32>> {
        if self.isolated_storage != other.isolated_storage
            || self.sequences != other.sequences
            || self.company_unknown != other.company_unknown
            || self.full_copies != other.full_copies
        {
            return None;
        }
        let (mine, my_gone) = self.store.committed_changes_from(&start.store)?;
        let (theirs, their_gone) = other.store.committed_changes_from(&start.store)?;
        let mut out = BTreeSet::new();
        let slots: BTreeSet<(String, u32)> = mine.into_iter().chain(theirs).collect();
        for (company, object) in slots {
            if table_rows(self, &company, object, engine)? != table_rows(other, &company, object, engine)? {
                out.insert(object);
            }
        }
        let gone_a: BTreeSet<_> = my_gone.into_iter().collect();
        let gone_b: BTreeSet<_> = their_gone.into_iter().collect();
        out.extend(gone_a.symmetric_difference(&gone_b).map(|(_, o)| *o));
        for k in self.autoinc.keys().chain(other.autoinc.keys()) {
            if self.autoinc.get(k) != other.autoinc.get(k) {
                out.insert(k.1);
            }
        }
        Some(out)
    }
}

/// The committed rows of `object` in `company`, encoded with the system fields blanked; empty when the store holds none.
fn table_rows(b: &Baseline, company: &str, object: u32, engine: &crate::Engine) -> Option<Vec<u8>> {
    let Some((t, _)) = b.store.committed_table(company, object) else {
        return Some(Vec::new());
    };
    let system: Vec<usize> = engine
        .tables
        .get(object as usize)
        .and_then(|d| d.as_ref())
        .map(|d| {
            d.fields
                .iter()
                .enumerate()
                .filter(|(_, f)| SYSTEM_FIELDS.contains(&f.id))
                .map(|(i, _)| i)
                .collect()
        })
        .unwrap_or_default();
    let mut out = Vec::new();
    al_builtins::value_codec::put_u32(&mut out, t.rows.len() as u32);
    for (key, row) in &t.rows {
        let mut row = row.clone();
        for &i in &system {
            if let Some(v) = row.get_mut(i) {
                *v = al_builtins::value::Value::Integer(0);
            }
        }
        if !al_builtins::value_codec::encode_row(key, &mut out) || !al_builtins::value_codec::encode_row(&row, &mut out)
        {
            return None;
        }
    }
    Some(out)
}

impl Session {
    /// The apps with no known install order to the app that installs (`Session::set_apps_install_order_unknown`).
    pub fn apps_install_order_unknown(&self) -> Option<&BTreeSet<u32>> {
        self.events.order_unknown_apps.as_ref()
    }

    /// The apps not installed yet (`Session::set_apps_not_installed`).
    pub fn apps_not_installed(&self) -> Option<&BTreeSet<u32>> {
        self.events.not_installed_apps.as_ref()
    }
}
