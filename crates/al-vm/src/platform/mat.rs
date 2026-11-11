//! A computed table whose rows are listed in full once, then scanned like a stored one.

use al_builtins::filter::{FieldFilter, key_cmp};
use al_builtins::value::Value;
use al_data::{Row, Scan, StoreError, TableDef, position};
use std::cmp::Ordering;

/// One row of a listed table. A row that could not be worked out (the declaration says something
/// the provider does not understand) is `poison`: it has its key and the reason, and a read that
/// might touch it is refused, while a read that cannot (the key filters exclude it) is answered.
pub(crate) struct Entry {
    pub row: Row,
    pub poison: Option<String>,
    /// How many leading primary key columns of a poisoned row are known; the rest are placeholders
    /// that no filter may rule the row out by.
    pub known: usize,
    /// Row positions (beyond the key) whose value in a poisoned row is real: a filter on them can rule the row out.
    pub known_cols: Vec<usize>,
    /// The row is seen only by a lookup by its value in this column (see [`Entry::by_name_only`]); with a reason, a
    /// lookup that finds it refuses.
    pub name_only: Option<(usize, Option<String>)>,
    /// With `name_only`: the row stands in for rows a scan does see ([`Entry::in_name_set`]).
    pub name_set: bool,
    /// The row rests on an unprobed assumption: a read that returns or counts it marks it.
    pub assumed: Option<al_builtins::assumptions::Assumption>,
    /// The app (an index in `Program::apps`) whose object the row lists ([`Entry::of_app`]); `None`: no app's, a row
    /// every engine has. A listing built over every app leaves out the rows of the apps an engine does not have
    /// ([`Mat::view`]).
    pub app: Option<u32>,
}

impl Entry {
    pub(crate) fn ok(row: Row) -> Entry {
        Entry {
            row,
            poison: None,
            known: usize::MAX,
            known_cols: Vec::new(),
            name_only: None,
            name_set: false,
            assumed: None,
            app: None,
        }
    }
    pub(crate) fn poisoned(key_only: Row, why: String) -> Entry {
        Entry {
            row: key_only,
            poison: Some(why),
            known: usize::MAX,
            known_cols: Vec::new(),
            name_only: None,
            name_set: false,
            assumed: None,
            app: None,
        }
    }
    /// A poisoned row of which only the first `known` primary key columns are real: it stands for
    /// every row that starts with them.
    pub(crate) fn poisoned_prefix(key_only: Row, known: usize, why: String) -> Entry {
        Entry {
            row: key_only,
            poison: Some(why),
            known,
            known_cols: Vec::new(),
            name_only: None,
            name_set: false,
            assumed: None,
            app: None,
        }
    }
    /// The positions of the columns of a poisoned row that hold real values.
    pub(crate) fn with_known_cols(mut self, cols: Vec<usize>) -> Entry {
        self.known_cols = cols;
        self
    }
    /// A row only a lookup by its value in column `col` finds: a scan whose filters are an equality on `col` that the
    /// row meets and an equality on the leading key column sees it; a scan without a filter on `col`, or with one the
    /// row does not meet, and `get` do not. Any other shape that could reach it refuses, and so does every lookup that
    /// finds it when `refuse` gives a reason. The Field table's rowversion row (fields.rs).
    pub(crate) fn by_name_only(mut self, col: usize, refuse: Option<String>) -> Entry {
        self.name_only = Some((col, refuse));
        self
    }
    /// A stand-in for rows whose names are known to be among a set of names, but whose other values are not: a scan whose
    /// filters on `col` leave the stand-in's name out, or whose filters on the key columns and `known` columns leave the
    /// stand-in out, does not see it; every other scan, one without a filter on `col` included, refuses with `why`
    /// (the "Page Action" rows of area(Promoted), page_action_promoted.rs).
    pub(crate) fn in_name_set(mut self, col: usize, known: Vec<usize>, why: String) -> Entry {
        self.name_only = Some((col, Some(why)));
        self.name_set = true;
        self.known_cols = known;
        self
    }
    /// A row whose values rest on assumption `a`: every read that returns or counts it marks `a`.
    pub(crate) fn assuming(mut self, a: al_builtins::assumptions::Assumption) -> Entry {
        self.assumed = Some(a);
        self
    }
    /// A row of an object of app `app`: an engine whose tenant does not have the app does not see it ([`Mat::view`]).
    pub(crate) fn of_app(mut self, app: u32) -> Entry {
        self.app = Some(app);
        self
    }
}

/// Whether a filter keeps exactly one value: `SetRange(F, A)`, `SetRange(F, A, A)` or `SetFilter(F, 'A')`.
pub(crate) fn single_value(e: &al_builtins::filter::FilterExpr) -> bool {
    use al_builtins::filter::FilterExpr;
    match e {
        FilterExpr::Eq(_) => true,
        FilterExpr::Range(Some(a), Some(b)) => {
            key_cmp(std::slice::from_ref(a), std::slice::from_ref(b)) == Ordering::Equal
        }
        _ => false,
    }
}

/// The value a filter of a single value ([`single_value`]) keeps; `None` for any other filter.
pub(crate) fn single_value_of(e: &al_builtins::filter::FilterExpr) -> Option<Value> {
    use al_builtins::filter::FilterExpr;
    match e {
        FilterExpr::Eq(v) => Some(v.clone()),
        FilterExpr::Range(Some(a), Some(_)) if single_value(e) => Some(a.clone()),
        _ => None,
    }
}

/// All rows of one table, sorted by primary key.
pub(crate) struct Mat {
    rows: Vec<Row>,
    poison: Vec<Option<String>>,
    known: Vec<usize>,
    known_cols: Vec<Vec<usize>>,
    name_only: Vec<Option<(usize, Option<String>)>>,
    /// Per row: a stand-in of a name set ([`Entry::in_name_set`]); empty when no row is one.
    name_set: Vec<bool>,
    /// Per row: the assumption a read of it marks ([`Entry::assuming`]); empty when no row has one.
    assumed: Vec<Option<al_builtins::assumptions::Assumption>>,
    /// Per row: the app whose object it lists ([`Entry::of_app`]); empty when no row has one.
    apps: Vec<Option<u32>>,
    /// The distinct apps of `apps` and `unviewable`, sorted: what [`Mat::lists_other_apps`] asks on every read.
    apps_listed: Vec<u32>,
    /// The apps a view cannot leave out by rows ([`Mat::view`]): two entries of one key that were made for different
    /// apps (the first stays, so the other app's row is lost), and the apps a builder marked with [`Mat::mixed_with`]
    /// (an object of the app changes the row of an object of another app, as a page extension changes its page's row).
    unviewable: Vec<u32>,
    /// No row is poisoned. Only then are the values of the key columns all real, so the rows are in the order of
    /// the key and the filters on its leading columns bound a scan (`band`).
    clean: bool,
    /// Per column: some row leaves it unknown (`Value::Unknown`). A filter on such a column, or an order by it, is
    /// refused for a row that leaves it unknown, since whether that row passes or where it sorts is not known.
    unknown_cols: Box<[bool]>,
    /// The rows that passed a set of filters, for a set seen before (`passing`): a lookup that scans a big listing
    /// on a column the key does not cover (the fields that relate to a table) repeats with the same filters.
    memo: Box<std::sync::Mutex<Memo>>,
}

/// The filter sets a big listing has been scanned with: how often, and for the repeated ones the indexes of the
/// rows that pass, in order.
#[derive(Default)]
struct Memo {
    seen: std::collections::HashMap<String, Seen>,
}

/// How often a filter set was scanned with, and, from the second time on, the indexes of the rows that pass.
type Seen = (u32, Option<std::sync::Arc<Vec<u32>>>);

/// Listings with fewer rows than this are walked each time.
const MEMO_MIN_ROWS: usize = 2048;

/// How many filter sets a listing remembers; a listing that is scanned with more kinds stops remembering new ones.
const MEMO_MAX_SETS: usize = 256;

/// A listing that several threads read. `Value` is not `Send` because some variants hold `Rc`
/// handles; the rows of a listing are made of plain values only (checked when it is made), whose
/// only shared parts are `Arc`s, and a `Mat` is never changed after it is built.
pub(crate) struct SharedMat(Mat);

impl SharedMat {
    /// The listing for sharing, or the listing back when a row holds a handle (a provider listed
    /// something that cannot be shared between threads).
    pub(crate) fn try_new(m: Mat) -> Result<SharedMat, Box<Mat>> {
        if m.rows.iter().all(|row| row.iter().all(Value::is_plain)) {
            Ok(SharedMat(m))
        } else {
            Err(Box::new(m))
        }
    }
}

impl std::ops::Deref for SharedMat {
    type Target = Mat;
    fn deref(&self) -> &Mat {
        &self.0
    }
}

// SAFETY: `SharedMat::try_new` checked that every value is plain (no `Rc`, no `RefCell`); the
// remaining reference-counted parts are `Arc`s. The contents are read-only from then on.
unsafe impl Send for SharedMat {}
unsafe impl Sync for SharedMat {}

/// Per column of the table: whether a row that is not poisoned leaves it unknown.
fn unknown_cols(
    t: &TableDef,
    rows: &[Row],
    poison: &[Option<String>],
    name_only: &[Option<(usize, Option<String>)>],
) -> Box<[bool]> {
    let mut out = vec![false; t.fields.len()];
    for ((r, p), n) in rows.iter().zip(poison).zip(name_only) {
        if p.is_none() && n.is_none() {
            for (c, v) in r.iter().enumerate() {
                if matches!(v, Value::Unknown(_)) {
                    out[c] = true;
                }
            }
        }
    }
    out.into_boxed_slice()
}

#[cfg(test)]
thread_local! {
    /// How many rows `passes` looked at: the tests of the key band count what a scan reads.
    static ROWS_LOOKED_AT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn passes(row: &Row, filters: &[FieldFilter]) -> bool {
    #[cfg(test)]
    ROWS_LOOKED_AT.with(|c| c.set(c.get() + 1));
    al_builtins::filter::row_passes_checked(row, filters)
}

/// Whether a row known only by its key can pass: the filters on key columns are decided, the
/// others could go either way.
fn may_pass(t: &TableDef, row: &Row, known: usize, known_cols: &[usize], filters: &[FieldFilter]) -> bool {
    filters
        .iter()
        .filter(|f| f.group != al_builtins::filter::OR_GROUP)
        .filter(|f| {
            known_cols.contains(&f.field) || t.keys[0].iter().position(|k| *k == f.field).is_some_and(|i| i < known)
        })
        .all(|f| f.expr.matches_checked(&row[f.field]))
}

/// Why a filter on the row cannot be decided: it reads a column the row leaves unknown, and the filters on the
/// columns the row knows do not leave it out (all of them must hold, outside the cross-column OR group).
fn unknown_filtered<'a>(row: &'a Row, filters: &[FieldFilter]) -> Option<&'a str> {
    let why = filters.iter().find_map(|f| row[f.field].unknown())?;
    let or_group = filters.iter().any(|f| f.group == al_builtins::filter::OR_GROUP);
    let left_out = !or_group
        && filters
            .iter()
            .any(|f| row[f.field].unknown().is_none() && !f.expr.matches_checked(&row[f.field]));
    (!left_out).then_some(why)
}

/// [`unknown_filtered`] when `check` (no row leaves a column of the filters unknown, so the answer is `None` and the
/// scan of the filters is skipped: it ran for every row of a big listing).
#[inline]
fn unknown_filtered_if<'a>(check: bool, row: &'a Row, filters: &[FieldFilter]) -> Option<&'a str> {
    if check { unknown_filtered(row, filters) } else { None }
}

fn refusal(t: &TableDef, why: &str) -> StoreError {
    StoreError::Refused(format!("{}: {why}", t.name))
}

impl Mat {
    pub(crate) fn new(t: &TableDef, mut entries: Vec<Entry>) -> Mat {
        // Each row's key is built once: building it per comparison was most of the time of a
        // 37,000-row listing. The sort is stable and the first of equal keys stays, as before.
        let mut keyed: Vec<(Vec<Value>, Entry)> = entries.drain(..).map(|e| (t.primary_key(&e.row), e)).collect();
        keyed.sort_by(|a, b| key_cmp(&a.0, &b.0));
        let mut unviewable = Vec::new();
        keyed.dedup_by(|a, b| {
            let same = key_cmp(&a.0, &b.0) == Ordering::Equal;
            if same && a.1.app != b.1.app {
                unviewable.extend(a.1.app.into_iter().chain(b.1.app));
            }
            same
        });
        let mut rows = Vec::new();
        let mut poison = Vec::new();
        let mut known = Vec::new();
        let mut known_cols = Vec::new();
        let mut name_only = Vec::new();
        let mut name_set = Vec::new();
        let mut assumed = Vec::new();
        let mut apps = Vec::new();
        for (_, e) in keyed {
            assumed.push(e.assumed);
            apps.push(e.app);
            rows.push(e.row);
            poison.push(e.poison);
            known.push(e.known);
            known_cols.push(e.known_cols);
            name_only.push(e.name_only);
            name_set.push(e.name_set);
        }
        if !name_set.contains(&true) {
            name_set = Vec::new();
        }
        let clean = poison.iter().all(Option::is_none);
        let unknown_cols = unknown_cols(t, &rows, &poison, &name_only);
        if assumed.iter().all(Option::is_none) {
            assumed = Vec::new();
        }
        if apps.iter().all(Option::is_none) {
            apps = Vec::new();
        }
        unviewable.sort_unstable();
        unviewable.dedup();
        Mat {
            rows,
            poison,
            known,
            known_cols,
            name_only,
            name_set,
            assumed,
            apps_listed: Mat::apps_listed(&apps, &unviewable),
            apps,
            unviewable,
            clean,
            unknown_cols,
            memo: Default::default(),
        }
    }

    /// Whether a filter of the set reads a column some row leaves unknown: only then are the rows checked one by one.
    fn filters_unknown(&self, filters: &[FieldFilter]) -> bool {
        filters
            .iter()
            .any(|f| self.unknown_cols.get(f.field).copied().unwrap_or(false))
    }

    /// Row `i` is read: mark the assumption it rests on, if any.
    #[inline]
    fn seen(&self, i: usize) {
        if let Some(Some(a)) = self.assumed.get(i) {
            al_builtins::assumptions::assume(*a);
        }
    }

    /// The same listing with every row changed by `f` (the key columns must stay as they are).
    pub(crate) fn mapped(&self, f: impl Fn(&Row) -> Row) -> Mat {
        Mat {
            rows: self.rows.iter().map(f).collect(),
            poison: self.poison.clone(),
            known: self.known.clone(),
            known_cols: self.known_cols.clone(),
            name_only: self.name_only.clone(),
            name_set: self.name_set.clone(),
            assumed: self.assumed.clone(),
            apps: self.apps.clone(),
            apps_listed: self.apps_listed.clone(),
            unviewable: self.unviewable.clone(),
            clean: self.clean,
            unknown_cols: self.unknown_cols.clone(),
            memo: Default::default(),
        }
    }

    /// Marks `apps` as apps whose objects change rows of other apps' objects: a view that leaves one of them out is not
    /// made from this listing ([`Mat::view`]).
    pub(crate) fn mixed_with(mut self, apps: impl IntoIterator<Item = u32>) -> Mat {
        self.unviewable.extend(apps);
        self.unviewable.sort_unstable();
        self.unviewable.dedup();
        self.apps_listed = Mat::apps_listed(&self.apps, &self.unviewable);
        self
    }

    /// Whether a row of an app that `installed` leaves out is in this listing.
    pub(crate) fn lists_other_apps(&self, installed: impl Fn(u32) -> bool) -> bool {
        self.apps_listed.iter().any(|&a| !installed(a))
    }

    /// The distinct apps of the rows and of `unviewable`, sorted.
    fn apps_listed(apps: &[Option<u32>], unviewable: &[u32]) -> Vec<u32> {
        let mut all: Vec<u32> = apps.iter().flatten().chain(unviewable).copied().collect();
        all.sort_unstable();
        all.dedup();
        all
    }

    /// This listing without the rows of the apps that `installed` leaves out (ADR-073 Phase 2: an engine's tenant
    /// does not have them, `Engine::app_installed`), in the same order. `None` when the rows cannot be left out one by
    /// one: an app left out is one of [`Mat::unviewable`]'s. Its rows are the rows the builder makes over the apps
    /// `installed` keeps, as every row is made from one object alone.
    pub(crate) fn view(&self, t: &TableDef, installed: impl Fn(u32) -> bool) -> Option<Mat> {
        if self.unviewable.iter().any(|&a| !installed(a)) {
            return None;
        }
        let keep: Vec<usize> = (0..self.rows.len())
            .filter(|&i| self.apps.get(i).copied().flatten().is_none_or(&installed))
            .collect();
        fn pick<T: Clone>(v: &[T], keep: &[usize]) -> Vec<T> {
            if v.is_empty() {
                Vec::new()
            } else {
                keep.iter().map(|&i| v[i].clone()).collect()
            }
        }
        let (rows, poison, name_only) = (
            pick(&self.rows, &keep),
            pick(&self.poison, &keep),
            pick(&self.name_only, &keep),
        );
        let mut name_set = pick(&self.name_set, &keep);
        if !name_set.contains(&true) {
            name_set = Vec::new();
        }
        let mut assumed = pick(&self.assumed, &keep);
        if assumed.iter().all(Option::is_none) {
            assumed = Vec::new();
        }
        let mut apps = pick(&self.apps, &keep);
        if apps.iter().all(Option::is_none) {
            apps = Vec::new();
        }
        Some(Mat {
            clean: poison.iter().all(Option::is_none),
            unknown_cols: unknown_cols(t, &rows, &poison, &name_only),
            known: pick(&self.known, &keep),
            known_cols: pick(&self.known_cols, &keep),
            rows,
            poison,
            name_only,
            name_set,
            assumed,
            apps_listed: Mat::apps_listed(&apps, &self.unviewable),
            apps,
            unviewable: self.unviewable.clone(),
            memo: Default::default(),
        })
    }

    /// Row `i` in key order, poisoned or not (a test walks the keys of a listing).
    pub(crate) fn row_at(&self, i: usize) -> Option<&Row> {
        self.rows.get(i)
    }

    pub(crate) fn get(&self, t: &TableDef, pk: &[Value]) -> Result<Option<Row>, StoreError> {
        // A poisoned row known only by a key prefix stands for every key that starts with it: a Get of such a key cannot
        // answer that the row is missing (al-runner issue 5790: the one Feature Key row of a minor not measured).
        if !self.clean {
            for (i, row) in self.rows.iter().enumerate() {
                if let Some(why) = &self.poison[i]
                    && self.known[i] < pk.len()
                    && key_cmp(&t.primary_key(row)[..self.known[i]], &pk[..self.known[i]]) == Ordering::Equal
                {
                    return Err(refusal(t, why));
                }
            }
        }
        Ok(match self.rows.binary_search_by(|r| key_cmp(&t.primary_key(r), pk)) {
            Ok(i) if self.name_only[i].is_some() => None,
            Ok(i) => match &self.poison[i] {
                Some(why) => return Err(refusal(t, why)),
                None => {
                    self.seen(i);
                    Some(self.rows[i].clone())
                }
            },
            Err(_) => None,
        })
    }

    /// The indexes of the rows that can pass `filters`: those between the bounds the filters on the leading
    /// primary key columns give (ADR-035, as in the store: a row of the band may still fail, a row outside it
    /// cannot pass). The whole listing when the filters give no bound, or when a row is poisoned (its key
    /// columns may be placeholders, which do not sort).
    fn band(&self, t: &TableDef, filters: &[FieldFilter]) -> std::ops::Range<usize> {
        let all = 0..self.rows.len();
        if !self.clean {
            return all;
        }
        let Some(seek) = al_data::memory::Seek::of(t, 0, filters, self.rows.first()) else {
            return all;
        };
        let lo = self.rows.partition_point(|r| seek.before(r) == Some(true));
        let hi = self.rows.partition_point(|r| seek.after(r) != Some(true));
        lo..hi.max(lo)
    }

    /// The indexes of all rows that pass `filters`, once a big listing has been scanned twice with this set of
    /// filters; `None` for the first scan, for a small listing and for one with a poisoned row. The result is the
    /// same as testing each row: the listing never changes.
    fn passing(&self, t: &TableDef, filters: &[FieldFilter]) -> Option<std::sync::Arc<Vec<u32>>> {
        if !self.clean || self.rows.len() < MEMO_MIN_ROWS || filters.is_empty() || self.filters_unknown(filters) {
            return None;
        }
        // A row only a lookup by name finds is decided per scan (`found_by_name`), never from a memo.
        if !self.name_set.is_empty()
            || filters
                .iter()
                .any(|f| self.name_only.iter().flatten().any(|(c, _)| *c == f.field))
        {
            return None;
        }
        let key = format!("{filters:?}");
        {
            let mut memo = self.memo.lock().unwrap_or_else(|e| e.into_inner());
            if memo.seen.len() >= MEMO_MAX_SETS && !memo.seen.contains_key(&key) {
                return None;
            }
            let entry = memo.seen.entry(key.clone()).or_insert((0, None));
            if let Some(idx) = &entry.1 {
                return Some(idx.clone());
            }
            entry.0 += 1;
            if entry.0 < 2 {
                return None;
            }
        }
        let idx: std::sync::Arc<Vec<u32>> = std::sync::Arc::new(
            // Only the band the filters give is tested: a row outside it cannot pass (`band`), and the memo is
            // keyed by the filters, so every scan with them has the same band.
            self.band(t, filters)
                .filter(|&i| self.name_only[i].is_none() && passes(&self.rows[i], filters))
                .map(|i| i as u32)
                .collect(),
        );
        let mut memo = self.memo.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = memo.seen.get_mut(&key) {
            entry.1 = Some(idx.clone());
        }
        Some(idx)
    }

    /// The first row at or after index `from` (walking towards `ascending`) that passes the
    /// filters; a row that cannot be worked out and might pass refuses.
    fn walk(
        &self,
        t: &TableDef,
        range: std::ops::Range<usize>,
        ascending: bool,
        filters: &[FieldFilter],
    ) -> Result<Option<Row>, StoreError> {
        if let Some(idx) = self.passing(t, filters) {
            // The rows that pass are known: the first one inside the range, from the side the scan comes from.
            let pick = if ascending {
                let k = idx.partition_point(|&i| (i as usize) < range.start);
                idx.get(k).copied().filter(|&i| (i as usize) < range.end)
            } else {
                let k = idx.partition_point(|&i| (i as usize) < range.end);
                k.checked_sub(1)
                    .map(|k| idx[k])
                    .filter(|&i| (i as usize) >= range.start)
            };
            return Ok(pick.map(|i| {
                self.seen(i as usize);
                self.rows[i as usize].clone()
            }));
        }
        let check_unknown = self.filters_unknown(filters);
        let at = |i: usize| -> Result<Option<Row>, StoreError> {
            let r = &self.rows[i];
            if self.name_only[i].is_some() {
                return Ok(self.found_by_name(t, i, filters, 0)?.then(|| {
                    self.seen(i);
                    r.clone()
                }));
            }
            match &self.poison[i] {
                Some(why) if may_pass(t, r, self.known[i], &self.known_cols[i], filters) => Err(refusal(t, why)),
                Some(_) => Ok(None),
                None => {
                    if let Some(why) = unknown_filtered_if(check_unknown, r, filters) {
                        return Err(StoreError::Refused(why.to_string()));
                    }
                    Ok(passes(r, filters).then(|| {
                        self.seen(i);
                        r.clone()
                    }))
                }
            }
        };
        if ascending {
            for i in range {
                if let Some(r) = at(i)? {
                    return Ok(Some(r));
                }
            }
        } else {
            for i in range.rev() {
                if let Some(r) = at(i)? {
                    return Ok(Some(r));
                }
            }
        }
        Ok(None)
    }

    pub(crate) fn next(&self, t: &TableDef, scan: &Scan) -> Result<Option<Row>, StoreError> {
        super::refuse_unpinned(scan.filters)?;
        let later = |r: &Row| match scan.after {
            None => true,
            Some(a) => {
                let o = key_cmp(&position(t, scan.key, r), a);
                if scan.ascending {
                    o == Ordering::Greater
                } else {
                    o == Ordering::Less
                }
            }
        };
        if scan.key == 0 {
            // Sorted by this very key: start where the position says and walk.
            let start = match scan.after {
                None => 0,
                Some(a) => self
                    .rows
                    .partition_point(|r| key_cmp(&position(t, 0, r), a) != Ordering::Greater),
            };
            let band = self.band(t, scan.filters);
            if scan.ascending {
                return self.walk(
                    t,
                    start.max(band.start)..self.rows.len().min(band.end),
                    true,
                    scan.filters,
                );
            }
            let end = match scan.after {
                None => self.rows.len(),
                Some(a) => self
                    .rows
                    .partition_point(|r| key_cmp(&position(t, 0, r), a) == Ordering::Less),
            };
            return self.walk(t, band.start..end.min(band.end), false, scan.filters);
        }
        // An order by a column some row leaves unknown: that row's place is not known.
        let key_unknown = t
            .keys
            .get(scan.key)
            .is_some_and(|k| k.iter().any(|&c| self.unknown_cols.get(c).copied().unwrap_or(false)));
        let check_unknown = self.filters_unknown(scan.filters);
        let mut best: Option<(Vec<Value>, usize)> = None;
        for (i, r) in self.rows.iter().enumerate() {
            if self.name_only[i].is_some() {
                // Checked first: its place in the order is checked there.
                if !self.found_by_name(t, i, scan.filters, scan.key)? || !later(r) {
                    continue;
                }
            }
            // A row that passes the filters but whose place is not known: whether it comes later is not known either.
            else if key_unknown
                && self.poison[i].is_none()
                && unknown_filtered_if(check_unknown, r, scan.filters).is_none()
                && passes(r, scan.filters)
            {
                if let Some(why) = t.keys[scan.key].iter().find_map(|&c| r[c].unknown()) {
                    return Err(StoreError::Refused(why.to_string()));
                }
            }
            if !later(r) {
                continue;
            }
            if let Some(why) = &self.poison[i] {
                if may_pass(t, r, self.known[i], &self.known_cols[i], scan.filters) {
                    return Err(refusal(t, why));
                }
                continue;
            }
            if let Some(why) = unknown_filtered_if(check_unknown, r, scan.filters) {
                return Err(StoreError::Refused(why.to_string()));
            }
            if !passes(r, scan.filters) {
                continue;
            }
            let pos = position(t, scan.key, r);
            let better = best.as_ref().is_none_or(|(b, _)| {
                let o = key_cmp(&pos, b);
                if scan.ascending {
                    o == Ordering::Less
                } else {
                    o == Ordering::Greater
                }
            });
            if better {
                best = Some((pos, i));
            }
        }
        Ok(best.map(|(_, i)| {
            self.seen(i);
            self.rows[i].clone()
        }))
    }

    /// Whether the row at `i`, which only a lookup by name finds ([`Entry::by_name_only`]), passes `filters` in a
    /// scan ordered by key `key`.
    ///
    /// An equality on the name finds the row, with or without a filter on the table number, and next to filters on
    /// its other columns, which apply as on any row: ObsoleteState `No` and `<>Removed`, Type, Class, Enabled, a
    /// TableNo range of one table and an alternative of two tables (types/AjMisc.al:689-731
    /// SG_FieldTable_RowVersion_NameWithOtherFilters, :733-748 _NameAlone; Data Type Management.FindFieldByName
    /// :750-772). A case-insensitive (`@TIMESTAMP`), wildcard (`times*`) or alternative (`timestamp|No.`) filter on
    /// the name does not see it (:712-717), nor does a not-equal, an `&` of not-equals, a comparison or a range
    /// (metadata/FieldNameNotEqualFilter.al:57-120); another form of name filter, and a filter in the OR group, refuse.
    fn found_by_name(&self, t: &TableDef, i: usize, filters: &[FieldFilter], key: usize) -> Result<bool, StoreError> {
        super::refuse_unpinned(filters)?;
        use al_builtins::filter::FilterExpr;
        let r = &self.rows[i];
        let Some((col, refuse)) = &self.name_only[i] else {
            return Ok(false);
        };
        let col = *col;
        if self.name_set.get(i).copied().unwrap_or(false) {
            let known = |c: usize| c == col || t.keys[0].contains(&c) || self.known_cols[i].contains(&c);
            let left_out = filters.iter().any(|f| {
                f.group != al_builtins::filter::OR_GROUP && known(f.field) && !f.expr.matches_checked(&r[f.field])
            });
            return match (left_out, refuse) {
                (true, _) => Ok(false),
                (false, Some(why)) => Err(refusal(t, why)),
                (false, None) => Ok(false),
            };
        }
        let or_group = filters.iter().any(|f| f.group == al_builtins::filter::OR_GROUP);
        // No lookup by name: the row is not seen.
        if !filters.iter().any(|f| f.field == col) {
            return Ok(false);
        }
        if !or_group {
            for f in filters.iter().filter(|f| f.field == col) {
                let unseen = match &f.expr {
                    e if single_value(e) => !e.matches_checked(&r[col]),
                    FilterExpr::Like(..) => true,
                    FilterExpr::Or(alts) => alts.iter().all(|a| matches!(a, FilterExpr::Eq(_))),
                    // A not-equal, an `&` of not-equals, a comparison and a range of two names do not see it either, with or
                    // without the Class, Enabled and ObsoleteState filters (metadata/FieldNameNotEqualFilter.al:57-120, staging
                    // r6980; al-runner issue 6995).
                    FilterExpr::Ne(_)
                    | FilterExpr::Lt(_)
                    | FilterExpr::Le(_)
                    | FilterExpr::Gt(_)
                    | FilterExpr::Ge(_)
                    | FilterExpr::Range(..) => true,
                    FilterExpr::And(parts) if parts.iter().all(|a| matches!(a, FilterExpr::Ne(_))) => true,
                    _ => {
                        return Err(refusal(
                            t,
                            "a row only a lookup by name finds (the Field row of the rowversion, timestamp): only an equality, a case-insensitive, a wildcard and an alternative filter on the name are pinned (types/AjMisc.al:689-731)",
                        ));
                    }
                };
                if unseen {
                    return Ok(false);
                }
            }
            // A table number the filters leave out: the row is not seen.
            let lead = t.keys[0].first().copied();
            if filters.iter().any(|f| {
                Some(f.field) == lead && r[f.field].unknown().is_none() && !f.expr.matches_checked(&r[f.field])
            }) {
                return Ok(false);
            }
        }
        if let Some(why) = refuse {
            return Err(refusal(t, why));
        }
        if or_group {
            return Err(refusal(
                t,
                "a row only a lookup by name finds (the Field row of the rowversion, timestamp): a filter in the OR group is not pinned (types/AjMisc.al:689-731)",
            ));
        }
        if let Some(why) = unknown_filtered(r, filters) {
            return Err(StoreError::Refused(why.to_string()));
        }
        if let Some(why) = t.keys.get(key).and_then(|k| k.iter().find_map(|&c| r[c].unknown())) {
            return Err(StoreError::Refused(why.to_string()));
        }
        Ok(passes(r, filters))
    }

    pub(crate) fn count(&self, t: &TableDef, filters: &[FieldFilter]) -> Result<usize, StoreError> {
        super::refuse_unpinned(filters)?;
        let mut n = 0;
        let band = self.band(t, filters);
        let check_unknown = self.filters_unknown(filters);
        for (i, r) in self.rows.iter().enumerate().skip(band.start).take(band.len()) {
            if self.name_only[i].is_some() {
                let found = self.found_by_name(t, i, filters, 0)?;
                if found {
                    self.seen(i);
                }
                n += usize::from(found);
                continue;
            }
            match &self.poison[i] {
                Some(why) if may_pass(t, r, self.known[i], &self.known_cols[i], filters) => {
                    return Err(refusal(t, why));
                }
                Some(_) => {}
                None => {
                    if let Some(why) = unknown_filtered_if(check_unknown, r, filters) {
                        return Err(StoreError::Refused(why.to_string()));
                    }
                    let pass = passes(r, filters);
                    if pass {
                        self.seen(i);
                    }
                    n += usize::from(pass)
                }
            }
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_builtins::filter::FilterExpr;
    use al_builtins::value::RtType;
    use al_data::FieldDef;

    fn table() -> TableDef {
        let f = |id, name: &str, ty| FieldDef {
            id,
            name: name.into(),
            ty,
            stored: true,
            code_storage: Default::default(),
        };
        TableDef {
            object: 0,
            name: "T".into(),
            fields: vec![f(1, "ID", RtType::Integer), f(2, "Name", RtType::Text(None))],
            keys: vec![vec![0]],
            per_company: false,
        }
    }

    fn row(id: i32, name: &str) -> Row {
        vec![Value::Integer(id), Value::Text(name.into())]
    }

    fn mat() -> Mat {
        Mat::new(
            &table(),
            vec![
                Entry::ok(row(3, "c")),
                Entry::ok(row(1, "a")),
                Entry::poisoned(row(2, ""), "row 2 is not understood".into()),
            ],
        )
    }

    /// A row built on an assumption marks it when a read returns or counts it, and only then.
    #[test]
    fn a_read_that_returns_or_counts_an_assumed_row_marks_its_assumption() {
        use al_builtins::assumptions::{Assumption, take_hits};
        let a = Assumption::FeatureKeyEnabledModify;
        let m = Mat::new(
            &table(),
            vec![Entry::ok(row(1, "a")), Entry::ok(row(2, "b")).assuming(a)],
        );
        let t = table();
        take_hits();
        assert!(m.get(&t, &[Value::Integer(1)]).unwrap().is_some());
        assert_eq!(m.count(&t, &[id_filter(1, 1)]).unwrap(), 1);
        assert!(m.next(&t, &scan(&[id_filter(1, 1)], None, true)).unwrap().is_some());
        assert!(!take_hits().contains(&a));
        assert!(m.get(&t, &[Value::Integer(2)]).unwrap().is_some());
        assert!(take_hits().contains(&a));
        assert_eq!(m.count(&t, &[]).unwrap(), 2);
        assert!(take_hits().contains(&a));
        let f = [id_filter(2, 2)];
        assert!(m.next(&t, &scan(&f, None, true)).unwrap().is_some());
        assert!(take_hits().contains(&a));
    }

    fn scan<'a>(filters: &'a [FieldFilter], after: Option<&'a [Value]>, ascending: bool) -> Scan<'a> {
        Scan {
            key: 0,
            filters,
            ascending,
            after,
        }
    }

    fn id_filter(lo: i32, hi: i32) -> FieldFilter {
        FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field: 0,
            expr: FilterExpr::Range(Some(Value::Integer(lo)), Some(Value::Integer(hi))),
            group: 0,
        }
    }

    #[test]
    fn a_filter_or_an_order_on_a_column_a_row_leaves_unknown_refuses_and_the_rest_runs() {
        let mut t = table();
        t.keys.push(vec![1]);
        let unknown = Value::Unknown("the value of field 'Name' is not known".into());
        let m = Mat::new(
            &t,
            vec![Entry::ok(row(1, "a")), Entry::ok(vec![Value::Integer(2), unknown])],
        );
        // Without a filter on Name both rows count and come back, the unknown value with them.
        assert_eq!(m.count(&t, &[]).ok(), Some(2));
        let after_1 = [Value::Integer(1), Value::Integer(1)];
        assert!(matches!(m.next(&t, &scan(&[], Some(&after_1), true)), Ok(Some(r)) if r[1].unknown().is_some()));
        // A filter on ID that leaves row 2 out is decided.
        assert_eq!(m.count(&t, &[id_filter(1, 1)]).ok(), Some(1));
        // A filter on Name would read row 2's value: refused, with the reason.
        let name_is_a = FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field: 1,
            expr: FilterExpr::Eq(Value::Text("a".into())),
            group: 0,
        };
        let refused =
            |r: Result<_, StoreError>| matches!(r, Err(StoreError::Refused(w)) if w.contains("'Name' is not known"));
        assert!(refused(m.count(&t, std::slice::from_ref(&name_is_a)).map(|_| ())));
        // A scan from the end meets row 2 first; one from the start finds row 1 before it.
        assert!(refused(
            m.next(&t, &scan(std::slice::from_ref(&name_is_a), None, false))
                .map(|_| ())
        ));
        assert!(
            matches!(m.next(&t, &scan(std::slice::from_ref(&name_is_a), None, true)), Ok(Some(r)) if matches!(r[0], Value::Integer(1)))
        );
        // Unless the other filters leave the row out first.
        assert_eq!(m.count(&t, &[id_filter(1, 1), name_is_a]).ok(), Some(1));
        // An order by Name: the place of row 2 is not known.
        let by_name = Scan {
            key: 1,
            filters: &[],
            ascending: true,
            after: None,
        };
        assert!(refused(m.next(&t, &by_name).map(|_| ())));
    }

    #[test]
    fn a_row_a_decided_filter_leaves_out_is_not_refused_for_its_unknown_column() {
        let t = table();
        let unknown = Value::Unknown("the value of field 'Name' is not known".into());
        // The poisoned row makes the listing unclean: no band bounds the scan, every row is looked at.
        let m = Mat::new(
            &t,
            vec![
                Entry::ok(row(1, "a")),
                Entry::ok(vec![Value::Integer(2), unknown]),
                Entry::poisoned(row(3, ""), "not known".into()),
            ],
        );
        let name_is_a = FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field: 1,
            expr: FilterExpr::Eq(Value::Text("a".into())),
            group: 0,
        };
        assert_eq!(m.count(&t, &[id_filter(1, 1), name_is_a.clone()]).ok(), Some(1));
        assert!(
            matches!(m.next(&t, &scan(&[id_filter(1, 1), name_is_a.clone()], None, false)), Ok(Some(r)) if matches!(r[0], Value::Integer(1)))
        );
        // Row 2 passes the decided filter: still refused.
        assert!(matches!(
            m.count(&t, &[id_filter(1, 2), name_is_a]),
            Err(StoreError::Refused(_))
        ));
    }

    #[test]
    fn rows_come_back_in_key_order_and_a_poisoned_row_refuses_only_a_read_that_could_reach_it() {
        let (t, m) = (table(), mat());
        // The scan from the start reaches row 1, then would pass row 2.
        assert!(matches!(m.next(&t, &scan(&[], None, true)), Ok(Some(r)) if matches!(r[0], Value::Integer(1))));
        let after_1 = [Value::Integer(1), Value::Integer(1)];
        assert!(
            m.next(&t, &scan(&[], Some(&after_1), true)).is_err(),
            "row 2 might be the next row"
        );
        // A key filter that excludes row 2 is answered.
        let f = [id_filter(3, 9)];
        assert!(matches!(m.next(&t, &scan(&f, None, true)), Ok(Some(r)) if matches!(r[0], Value::Integer(3))));
        assert!(matches!(m.count(&t, &f), Ok(1)));
        // A keyed read of the poisoned row refuses; of a missing row answers none.
        assert!(m.get(&t, &[Value::Integer(2)]).is_err());
        assert!(matches!(m.get(&t, &[Value::Integer(9)]), Ok(None)));
        assert!(matches!(m.get(&t, &[Value::Integer(3)]), Ok(Some(_))));
        assert!(
            m.count(&t, &[]).is_err(),
            "a count over everything includes the poisoned row"
        );
    }

    #[test]
    fn a_descending_scan_starts_at_the_last_row() {
        let (t, m) = (table(), mat());
        let f = [id_filter(3, 9)];
        assert!(matches!(m.next(&t, &scan(&f, None, false)), Ok(Some(r)) if matches!(r[0], Value::Integer(3))));
    }

    #[test]
    fn a_row_known_by_a_key_prefix_stands_for_every_row_that_starts_with_it() {
        // Key (ID, Name): a row poisoned with only ID known must not be ruled out by a Name filter.
        let t = TableDef {
            keys: vec![vec![0, 1]],
            ..table()
        };
        let m = Mat::new(
            &t,
            vec![
                Entry::ok(row(1, "a")),
                Entry::poisoned_prefix(row(2, ""), 1, "rows of 2 are not known".into()),
            ],
        );
        let name_is_a = FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field: 1,
            expr: FilterExpr::Eq(Value::Text("a".into())),
            group: 0,
        };
        // The Name placeholder ("") does not match 'a', yet the row may hold an 'a': refused.
        assert!(m.count(&t, &[name_is_a]).is_err());
        // An ID filter that excludes 2 is decided by the known column.
        assert!(matches!(m.count(&t, &[id_filter(1, 1)]), Ok(1)));
        assert!(m.get(&t, &[Value::Integer(2), Value::Text("".into())]).is_err());
        // A Get of a key the row stands for, past its placeholder, is refused as well (al-runner issue 5790).
        assert!(m.get(&t, &[Value::Integer(2), Value::Text("b".into())]).is_err());
        assert!(
            m.get(&t, &[Value::Integer(1), Value::Text("a".into())])
                .unwrap()
                .is_some()
        );
        assert!(
            m.get(&t, &[Value::Integer(3), Value::Text("b".into())])
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn a_get_cannot_miss_a_row_whose_whole_key_is_unknown() {
        // Feature Key on a minor that was not measured: one row, no key column known (al-runner issue 5790).
        let t = table();
        let m = Mat::new(&t, vec![Entry::poisoned_prefix(row(0, ""), 0, "not measured".into())]);
        assert!(m.get(&t, &[Value::Integer(7)]).is_err());
    }

    #[test]
    fn a_poisoned_row_with_a_known_column_is_ruled_out_by_a_filter_on_that_column() {
        // Event Subscription: a subscriber to a page event is refused for its Published Function, but its publisher type is real.
        let t = table();
        let m = Mat::new(
            &t,
            vec![
                Entry::ok(row(1, "a")),
                Entry::poisoned(row(2, "page"), "not pinned".into()).with_known_cols(vec![1]),
            ],
        );
        let name = |n: &str| FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field: 1,
            expr: FilterExpr::Eq(Value::Text(n.into())),
            group: 0,
        };
        assert!(matches!(m.count(&t, &[name("a")]), Ok(1)));
        assert!(m.count(&t, &[name("page")]).is_err());
        assert!(m.count(&t, &[]).is_err());
    }

    // ---- the band of the primary key (the walk of a listing starts and ends where the key filters say) ----

    fn big() -> Mat {
        big_of(1000)
    }

    fn big_of(n: i32) -> Mat {
        Mat::new(
            &table(),
            (1..=n).map(|i| Entry::ok(row(i, &format!("n{}", i % 7)))).collect(),
        )
    }

    fn looked_at(f: impl FnOnce() -> R) -> (R, usize)
    where
        R: Sized,
    {
        ROWS_LOOKED_AT.with(|c| c.set(0));
        let r = f();
        (r, ROWS_LOOKED_AT.with(|c| c.get()))
    }

    type R = Result<Option<Row>, al_data::StoreError>;

    #[test]
    fn a_scan_with_a_filter_on_the_leading_key_field_reads_only_the_band() {
        let (t, m) = (table(), big());
        let f = [id_filter(500, 502)];
        let (r, n) = looked_at(|| m.next(&t, &scan(&f, None, true)));
        assert!(matches!(r, Ok(Some(r)) if matches!(r[0], Value::Integer(500))));
        assert!(n <= 3, "{n} rows looked at");
        // The step past the last row of the band reads nothing of the 498 rows after it.
        let last = position(&t, 0, &row(502, "n5"));
        let (r, n) = looked_at(|| m.next(&t, &scan(&f, Some(&last), true)));
        assert!(matches!(r, Ok(None)));
        assert!(n <= 1, "{n} rows looked at");
        // Descending starts at the top of the band.
        let (r, n) = looked_at(|| m.next(&t, &scan(&f, None, false)));
        assert!(matches!(r, Ok(Some(r)) if matches!(r[0], Value::Integer(502))));
        assert!(n <= 3, "{n} rows looked at");
        let (c, n) = looked_at(|| m.count(&t, &f).map(|c| Some(vec![Value::Integer(c as i32)])));
        assert!(matches!(c, Ok(Some(r)) if matches!(r[0], Value::Integer(3))));
        assert!(n <= 3, "{n} rows looked at");
    }

    #[test]
    fn a_big_listing_scanned_again_with_the_same_filters_does_not_look_at_every_row_again() {
        let (t, m) = (table(), big_of(5000));
        let name3 = [FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field: 1,
            expr: FilterExpr::Eq(Value::Text("n3".into())),
            group: 0,
        }];
        // The first scan walks; the second builds the list of passing rows; the third and later read it.
        let first = looked_at(|| m.next(&t, &scan(&name3, None, true)));
        assert!(matches!(first.0, Ok(Some(r)) if matches!(r[0], Value::Integer(3))));
        let _ = m.next(&t, &scan(&name3, None, true));
        let (r, n) = looked_at(|| m.next(&t, &scan(&name3, None, true)));
        assert!(matches!(r, Ok(Some(r)) if matches!(r[0], Value::Integer(3))));
        assert_eq!(n, 0, "answered from the list of passing rows");
        // The step past the last row that passes reads nothing either.
        let last = position(&t, 0, &row(4994, "n3"));
        let (r, n) = looked_at(|| m.next(&t, &scan(&name3, Some(&last), true)));
        assert!(matches!(r, Ok(None)) && n == 0, "{n}");
        let (r, n) = looked_at(|| m.next(&t, &scan(&name3, None, false)));
        assert!(
            matches!(r, Ok(Some(r)) if matches!(r[0], Value::Integer(4994))) && n == 0,
            "{n}"
        );
    }

    #[test]
    fn the_list_of_passing_rows_is_built_from_the_band_only() {
        let (t, m) = (table(), big_of(5000));
        let f = [
            id_filter(1000, 1099),
            FieldFilter {
                lang: al_builtins::filter::filter_language(),
                field: 1,
                expr: FilterExpr::Eq(Value::Text("n3".into())),
                group: 0,
            },
        ];
        let _ = m.next(&t, &scan(&f, None, true));
        // The second scan builds the list: it tests the 100 rows of the band, not the 5000 of the listing.
        let (r, n) = looked_at(|| m.next(&t, &scan(&f, None, true)));
        assert!(matches!(r, Ok(Some(r)) if matches!(r[0], Value::Integer(1004))));
        assert!(n <= 100, "{n} rows looked at");
        let (r, n) = looked_at(|| m.next(&t, &scan(&f, None, false)));
        assert!(matches!(r, Ok(Some(r)) if matches!(r[0], Value::Integer(1095))));
        assert_eq!(n, 0, "answered from the list");
    }

    #[test]
    fn the_band_changes_no_answer() {
        use FilterExpr as E;
        let (t, m) = (table(), big());
        the_band_changes_no_answer_on(&t, &m);
        // A listing big enough to remember: every chain of Next runs three times, the last two from the list.
        let remembering = big_of(3000);
        for _ in 0..3 {
            the_band_changes_no_answer_on(&t, &remembering);
        }
        let _ = E::Eq(Value::Null);
    }

    fn the_band_changes_no_answer_on(t: &TableDef, m: &Mat) {
        use FilterExpr as E;
        let flt = |field, expr| FieldFilter {
            lang: al_builtins::filter::filter_language(),
            field,
            expr,
            group: 0,
        };
        let int = |i| Value::Integer(i);
        let cases: Vec<Vec<FieldFilter>> = vec![
            vec![flt(0, E::Eq(int(7)))],
            vec![flt(0, E::Gt(int(990)))],
            vec![flt(0, E::Lt(int(4)))],
            vec![flt(0, E::Range(None, Some(int(2))))],
            vec![flt(0, E::Range(Some(int(1001)), None))],
            vec![
                flt(0, E::Range(Some(int(10)), Some(int(40)))),
                flt(1, E::Eq(Value::Text("n3".into()))),
            ],
            vec![flt(1, E::Eq(Value::Text("n3".into())))],
            vec![flt(0, E::Ne(int(1)))],
            vec![flt(0, E::Range(Some(int(30)), Some(int(20))))],
        ];
        for f in &cases {
            let want: Vec<i32> = m
                .rows
                .iter()
                .filter(|r| passes(r, f))
                .map(|r| match r[0] {
                    Value::Integer(i) => i,
                    _ => 0,
                })
                .collect();
            let mut got = Vec::new();
            let mut after: Option<Vec<Value>> = None;
            while let Ok(Some(r)) = m.next(t, &scan(f, after.as_deref(), true)) {
                if let Value::Integer(i) = r[0] {
                    got.push(i);
                }
                after = Some(position(t, 0, &r));
            }
            assert_eq!(got, want, "ascending, {f:?}");
            let mut back = Vec::new();
            let mut after: Option<Vec<Value>> = None;
            while let Ok(Some(r)) = m.next(t, &scan(f, after.as_deref(), false)) {
                if let Value::Integer(i) = r[0] {
                    back.push(i);
                }
                after = Some(position(t, 0, &r));
            }
            back.reverse();
            assert_eq!(back, want, "descending, {f:?}");
            assert!(matches!(m.count(t, f), Ok(n) if n == want.len()), "count, {f:?}");
        }
    }
}

#[cfg(test)]
mod shared_tests {
    use super::*;
    use al_builtins::value::RtType;
    use al_data::FieldDef;

    fn def(ty: RtType) -> TableDef {
        TableDef {
            object: 0,
            name: "T".into(),
            fields: vec![FieldDef {
                id: 1,
                name: "F".into(),
                ty,
                stored: true,
                code_storage: Default::default(),
            }],
            keys: vec![vec![0]],
            per_company: false,
        }
    }

    #[test]
    fn a_listing_of_plain_values_can_be_shared() {
        let t = def(RtType::Integer);
        let m = Mat::new(&t, vec![Entry::ok(vec![Value::Integer(1)])]);
        fn is_send_sync<T: Send + Sync>(_: &T) {}
        is_send_sync(&SharedMat::try_new(m).ok().expect("plain"));
    }

    #[test]
    fn a_listing_holding_a_handle_cannot_be_shared() {
        let t = def(RtType::Integer);
        let handle = Value::List(Default::default());
        assert!(SharedMat::try_new(Mat::new(&t, vec![Entry::ok(vec![handle])])).is_err());
    }
}
