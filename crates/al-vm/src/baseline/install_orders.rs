//! What the runner needs to install two partner apps in both orders and compare the result (ADR-046 amendment 7, owner
//! 2026-11-08, al-runner issue 6644): which pairs of apps with no known install order the install code reached, a rewind
//! to the database before the apps installed, and the comparison of the databases two orders leave.

use super::Baseline;
use crate::Session;
use al_builtins::value::Value;
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

    /// The clock now, in milliseconds since the Unix epoch, without moving it on: the edges of the window in which one
    /// install order read the clock (`Baseline::install_order_differences`).
    pub fn clock_now_millis(&self) -> i64 {
        self.now_millis()
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

/// The clock readings around one install order: the first before its install code ran and the last after, in
/// milliseconds since the Unix epoch (`Session::clock_now_millis`).
pub type ClockWindow = (i64, i64);

/// How far a value read from the clock may lie outside its window: a DateTime field holds the SQL grid of 1/300 s
/// (`sqltime.rs`), so a stored value may be rounded down by up to 3 ms.
const CLOCK_SLACK_MS: i64 = 10;

const MS_PER_DAY: i64 = 86_400_000;

/// The differences of the committed rows of two captures, per table (object id): one line per difference, naming the key
/// and the field (`Baseline::install_order_differences`).
pub type OrderDifferences = std::collections::BTreeMap<u32, Vec<String>>;

impl Baseline {
    /// The tables (object ids) whose committed rows differ between `self` and `other`, two captures of install runs that
    /// both started from `start`, with a line per difference that names the key and the field. Left out: the system
    /// fields, and values read from the clock (`clock_value`): a field that holds a value inside the clock window of its
    /// own order in both captures counts as the same (al-runner issue 6644 b). BC keeps no order between two apps with no
    /// dependency path between them, so no test can depend on which of the two read the clock first; the rows each
    /// order inserts, and every value that is not a reading of the clock, still count. A table counts as differing also
    /// when its AutoIncrement counter does. `None` when something besides the tables differs (isolated storage, number
    /// sequences, the companies not known or copied), or a row is not a plain value: the caller cannot name what differs
    /// by table.
    pub fn install_order_differences(
        &self,
        other: &Baseline,
        start: &Baseline,
        engine: &crate::Engine,
        clocks: (ClockWindow, ClockWindow),
    ) -> Option<OrderDifferences> {
        if self.isolated_storage != other.isolated_storage
            || self.sequences != other.sequences
            || self.company_unknown != other.company_unknown
            || self.full_copies != other.full_copies
        {
            return None;
        }
        let (mine, my_gone) = self.store.committed_changes_from(&start.store)?;
        let (theirs, their_gone) = other.store.committed_changes_from(&start.store)?;
        let mut out = OrderDifferences::new();
        let slots: BTreeSet<(String, u32)> = mine.into_iter().chain(theirs).collect();
        for (company, object) in slots {
            let a = table_rows(self, &company, object, engine, clocks.0)?;
            let b = table_rows(other, &company, object, engine, clocks.1)?;
            if a.encoded != b.encoded {
                out.entry(object).or_default().extend(describe(&a, &b, object, engine));
            }
        }
        let gone_a: BTreeSet<_> = my_gone.into_iter().collect();
        let gone_b: BTreeSet<_> = their_gone.into_iter().collect();
        for (_, o) in gone_a.symmetric_difference(&gone_b) {
            out.entry(*o)
                .or_default()
                .push("the table is emptied in one order only".into());
        }
        for k in self.autoinc.keys().chain(other.autoinc.keys()) {
            if self.autoinc.get(k) != other.autoinc.get(k) {
                out.entry(k.1).or_default().push("the AutoIncrement counter".into());
            }
        }
        for v in out.values_mut() {
            v.sort();
            v.dedup();
        }
        Some(out)
    }
}

/// The committed rows of one table, with the system fields and the clock values blanked.
struct Rows {
    /// The rows, each as (key, row) with the blanked values.
    rows: Vec<(Vec<Value>, Vec<Value>)>,
    /// The rows encoded, sorted (two rows whose keys differ only in a clock value may swap places).
    encoded: Vec<(Vec<u8>, Vec<u8>)>,
}

/// Whether `v` is a value read from the clock inside `window`: a DateTime at an instant of the window, a Time at a time
/// of day of it, a Date of a day of it. The undefined values (0D, 0T, 0DT) are not.
fn clock_value(v: &Value, (from, to): ClockWindow) -> bool {
    let (from, to) = (from - CLOCK_SLACK_MS, to + CLOCK_SLACK_MS);
    match *v {
        Value::DateTime(t) if t != 0 => {
            let unix = crate::hostvalues::unix_millis_of_datetime(t);
            (from..=to).contains(&unix)
        }
        Value::Time(t) if t != 0 => {
            let at = t - 1;
            if to - from >= MS_PER_DAY {
                return true;
            }
            let (a, b) = (from.rem_euclid(MS_PER_DAY), to.rem_euclid(MS_PER_DAY));
            if a <= b {
                (a..=b).contains(&at)
            } else {
                at >= a || at <= b
            }
        }
        Value::Date(d, _) if d != 0 => {
            let day = |ms: i64| crate::hostvalues::day_number_of_unix_millis(ms);
            (day(from)..=day(to)).contains(&i64::from(d))
        }
        _ => false,
    }
}

/// The committed rows of `object` in `company`, the system fields and the values read from the clock in `window` blanked;
/// empty when the store holds none.
fn table_rows(b: &Baseline, company: &str, object: u32, engine: &crate::Engine, window: ClockWindow) -> Option<Rows> {
    let mut out = Rows {
        rows: Vec::new(),
        encoded: Vec::new(),
    };
    let Some((t, _)) = b.store.committed_table(company, object) else {
        return Some(out);
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
    let blank = |v: &mut Value| {
        if clock_value(v, window) {
            *v = Value::Null;
        }
    };
    for (key, row) in &t.rows {
        let mut key = key.clone();
        let mut row = row.clone();
        for &i in &system {
            if let Some(v) = row.get_mut(i) {
                *v = Value::Integer(0);
            }
        }
        key.iter_mut().for_each(blank);
        row.iter_mut().for_each(blank);
        let (mut k, mut r) = (Vec::new(), Vec::new());
        if !al_builtins::value_codec::encode_row(&key, &mut k) || !al_builtins::value_codec::encode_row(&row, &mut r) {
            return None;
        }
        out.encoded.push((k, r));
        out.rows.push((key, row));
    }
    out.encoded.sort();
    Some(out)
}

/// A key or field value for a line of `describe`.
fn show(v: &Value) -> String {
    match v {
        Value::Null => "(a clock value)".into(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::BigInteger(i) => i.to_string(),
        Value::Text(t) | Value::Code(t) => t.to_string(),
        other => format!("{other:?}"),
    }
}

fn encoded(v: &[Value]) -> Vec<u8> {
    let mut out = Vec::new();
    al_builtins::value_codec::encode_row(v, &mut out);
    out
}

/// One line per difference between the rows of two orders: a key in one order only, or a field whose value differs.
fn describe(a: &Rows, b: &Rows, object: u32, engine: &crate::Engine) -> Vec<String> {
    let def = engine.tables.get(object as usize).and_then(|d| d.as_ref());
    let field = |i: usize| {
        def.and_then(|d| d.fields.get(i))
            .map_or(format!("field {}", i + 1), |f| f.name.clone())
    };
    let key = |k: &[Value]| k.iter().map(show).collect::<Vec<_>>().join(", ");
    let find = |rows: &Rows, k: &[Value]| {
        let e = encoded(k);
        rows.rows.iter().find(|(x, _)| encoded(x) == e).map(|(_, r)| r.clone())
    };
    let mut out = Vec::new();
    for (k, row) in &a.rows {
        match find(b, k) {
            None => out.push(format!("key {}: a row in the first order only", key(k))),
            Some(other) => {
                for (i, (x, y)) in row.iter().zip(&other).enumerate() {
                    let mut ex = Vec::new();
                    let mut ey = Vec::new();
                    al_builtins::value_codec::encode_row(std::slice::from_ref(x), &mut ex);
                    al_builtins::value_codec::encode_row(std::slice::from_ref(y), &mut ey);
                    if ex != ey {
                        out.push(format!(
                            "key {}, field {}: {} against {}",
                            key(k),
                            field(i),
                            show(x),
                            show(y)
                        ));
                    }
                }
            }
        }
    }
    for (k, _) in &b.rows {
        if find(a, k).is_none() {
            out.push(format!("key {}: a row in the other order only", key(k)));
        }
    }
    out
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
