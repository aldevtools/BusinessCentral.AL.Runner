//! In-memory store on persistent maps: a snapshot is a cheap clone (ADR-004).
//! Scans are linear in the table size; fine for test-sized tables.

use crate::*;
use al_builtins::filter::{FieldFilter, key_cmp};
use std::cmp::Ordering;

/// A primary-key tuple ordered field by field.
#[derive(Clone, Debug)]
struct Key(Vec<Value>);

impl PartialEq for Key {
    fn eq(&self, o: &Self) -> bool {
        key_cmp(&self.0, &o.0) == Ordering::Equal
    }
}
impl Eq for Key {}
impl PartialOrd for Key {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Key {
    fn cmp(&self, o: &Self) -> Ordering {
        key_cmp(&self.0, &o.0)
    }
}

type Table = crate::pmap::PMap<Key, Row>;
/// A table's place in the store: its company (empty for a table that is not per company) and its object. The company
/// is a shared name, so building a slot for an operation copies a pointer, not a string.
type Slot = (std::sync::Arc<str>, u32);
type Tables = crate::pmap::PMap<Slot, Table>;

/// The rows of one table, built to be handed to a [`MemStore`] by a [`Backing`].
#[derive(Clone)]
pub struct Rows(Table, i64);

impl Rows {
    /// The largest value of the table's AutoIncrement field among the rows (0 when it has none):
    /// the counter continues above it, as an identity column does after a restore.
    pub fn with_auto_increment_floor(mut self, floor: i64) -> Rows {
        self.1 = floor;
        self
    }

    pub fn auto_increment_floor(&self) -> i64 {
        self.1
    }

    /// The table `rows` make. Two rows with the same primary key are an error carrying how many
    /// rows were lost to it: a table is never built from rows that overwrite each other.
    pub fn from_rows(t: &TableDef, rows: Vec<Row>) -> Result<Rows, usize> {
        let n = rows.len();
        let map: Table = rows.into_iter().map(|r| (Key(t.primary_key(&r)), r)).collect();
        if map.len() == n {
            Ok(Rows(map, 0))
        } else {
            Err(n - map.len())
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Rows a table holds before any write to it, read when the table is first touched (ADR-020: test
/// data). A table the store has no entry for reads as the backing's rows; the first write copies
/// them into the store, and from then on the store holds the table. A rollback to a state without
/// the entry reads the backing again: its rows are committed data.
pub trait Backing {
    /// The company the rows belong to.
    fn company(&self) -> &str;
    /// The rows of table `object` for `company` (the empty name for a table that is not per
    /// company); `None` when the backing has none to offer. Answering is cheap after the first
    /// time: a clone of a shared map.
    fn rows(&self, company: &str, object: u32) -> Option<Rows>;
    /// Whether the table is served at all (a table with rows to offer in some company).
    fn has_rows(&self, object: u32) -> bool {
        self.rows(self.company(), object)
            .or_else(|| self.rows("", object))
            .is_some_and(|r| !r.is_empty())
    }
}

/// A table's rows in the order of one key, to step through them without a pass over the table per step.
#[derive(Clone)]
struct OrderEntry {
    shape: u64,
    rows: usize,
    /// The fields of the key the order is of: an ad-hoc key can take the slot of an earlier one.
    fields: Vec<usize>,
    order: std::rc::Rc<Vec<Key>>,
}

/// What `alt_next` knows about one scan (a table, a key and a set of filters) as of a table version.
#[derive(Clone)]
struct AltEntry {
    version: u64,
    /// How many scan steps came at this version: the first is answered by a pass over the band, the second builds
    /// `keys`, the later ones search it.
    calls: u32,
    /// No other key narrows this scan: the caller walks.
    not_applicable: bool,
    /// The primary keys of the rows that pass the filters, in the order of the scan's key.
    keys: Option<std::rc::Rc<Vec<Key>>>,
}

/// Where the band of one key is: the primary key's rows between two keys, or a slice of the cached order.
enum Band {
    Pk,
    Order(std::rc::Rc<Vec<Key>>, std::ops::Range<usize>),
}

/// Tables with fewer rows than this are walked in a secondary key by looking at every row.
const INDEX_MIN_ROWS: usize = 64;

#[derive(Clone, Default)]
pub struct MemStore {
    tables: Tables,
    committed: Tables,
    /// The value of `counter` when each table last changed. It follows `tables` through commit and
    /// rollback, so a table has the same stamp exactly when it has the same content history.
    versions: crate::pmap::PMap<Slot, u64>,
    committed_versions: crate::pmap::PMap<Slot, u64>,
    /// The value of `counter` when the *shape* of each table last changed: a row was inserted, deleted or
    /// renamed, or a Modify changed a field some key is made of. It follows `tables` like `versions`, so
    /// equal stamps mean the same rows in the same places in every key.
    shapes: crate::pmap::PMap<Slot, u64>,
    committed_shapes: crate::pmap::PMap<Slot, u64>,
    /// The primary keys of a big table in the order of one of its other keys, as of a shape stamp.
    order_cache: std::cell::RefCell<std::collections::HashMap<(Slot, usize), OrderEntry>>,
    /// The rows that pass one set of filters, in the order of one key, for scans whose filters bound the leading
    /// fields of another key (see `alt_next`).
    alt_cache: std::cell::RefCell<std::collections::HashMap<(Slot, usize, String), AltEntry>>,
    /// Handed out by writes; never goes back, so a stamp is never reused.
    counter: u64,
    /// Tables that have no stored rows (ADR-019).
    virtuals: Option<std::rc::Rc<dyn VirtualTables>>,
    /// Rows a table holds before anything wrote to it (ADR-020).
    backing: Option<std::rc::Rc<dyn Backing>>,
    /// While set, a change to a row of the backing (a modify, delete or rename) is written down
    /// here and allowed (ADR-020 amendment: install code may change restored rows). Shared by the
    /// clones of the store, so a snapshot or a rollback does not lose it.
    restored_log: Option<std::rc::Rc<std::cell::RefCell<Vec<RestoredChange>>>>,
    /// Per table (object index), the columns in which a stored row held a value a platform row left unknown
    /// (`Value::Unknown`, a record copied from one). Never cleared: a filter or an order on such a column checks the
    /// rows (`unknown_read`).
    unknown_cols: std::collections::HashMap<u32, Vec<usize>>,
    /// A virtual table took a change (a Feature Key's Enabled) since the last commit: whether a rollback undoes it is
    /// not settled (platform/lt-aw-fresh-tenant/AwTests.al:557-576, a Linux suspect), so the session refuses that rollback.
    virtual_write_pending: bool,
}

/// One table of an exported store ([`MemStore::export_committed`]): its company (empty for a table that is not per
/// company), its object, and its rows with their primary keys, in key order.
#[derive(Debug, Clone)]
pub struct ExportedTable {
    pub company: String,
    pub object: u32,
    pub rows: Vec<(Vec<Value>, Row)>,
}

/// A change to a row the restored company holds, made while recording was on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoredChange {
    pub table: String,
    /// `Modify`, `Delete`, `Rename` or `DeleteAll`.
    pub op: &'static str,
    /// A Modify that writes the values the row already has, system fields apart. Every other change is `false`.
    pub same: bool,
    /// For a Modify with other values: the names of the fields whose values differ.
    pub fields: Vec<String>,
}

thread_local! {
    /// The company of a table that is not per company; one per thread, so threads do not share its count.
    static NO_COMPANY: std::sync::Arc<str> = std::sync::Arc::from("");
}

fn slot(cx: &AccessContext, t: &TableDef) -> Slot {
    (
        if t.per_company {
            cx.company.clone()
        } else {
            NO_COMPANY.with(|e| e.clone())
        },
        t.object,
    )
}

/// Whether `new` differs from `old` in a field some key of `t` is made of (the primary key included).
fn key_fields_differ(t: &TableDef, old: &[Value], new: &[Value]) -> bool {
    t.keys
        .iter()
        .flatten()
        .any(|&i| al_builtins::ops::compare_key_value(&old[i], &new[i]).map_or(true, |o| o != Ordering::Equal))
}

/// `key_cmp(position(t, key, row), other)` without building the position.
fn cmp_row_to_position(t: &TableDef, key: usize, row: &[Value], other: &[Value]) -> Ordering {
    for (&i, x) in t.keys[key].iter().chain(&t.keys[0]).zip(other) {
        match t.cmp_key_field(i, &row[i], x) {
            Ordering::Equal => continue,
            o => return o,
        }
    }
    (t.keys[key].len() + t.keys[0].len()).cmp(&other.len())
}

/// `key_cmp(position(t, key, a), position(t, key, b))` without building the positions.
fn cmp_rows(t: &TableDef, key: usize, a: &[Value], b: &[Value]) -> Ordering {
    for &i in t.keys[key].iter().chain(&t.keys[0]) {
        match t.cmp_key_field(i, &a[i], &b[i]) {
            Ordering::Equal => continue,
            o => return o,
        }
    }
    Ordering::Equal
}

/// The part of the order of a key that the filters narrow down (ADR-035): the position tuple of the first row a scan
/// can return, and of the last, as far as filters on the leading fields of the key say. Either may be empty (open),
/// and both are prefixes of a position. The full `passes` check still runs on every row between them, so a bound
/// that is too wide only costs time; a bound is only taken from what `key_cmp` order can answer exactly.
#[derive(Default)]
pub struct Seek {
    lo: Vec<Value>,
    hi: Vec<Value>,
    /// The fields of the position, as far as the bounds reach.
    fields: Vec<usize>,
}

/// A filter value a bound can be made of: not a blank text (the filter reads a text of spaces as '', which the key
/// order does not), and not a value that does not compare.
fn usable_bound(v: &Value) -> bool {
    match v {
        Value::Text(_) | Value::Code(_) => al_builtins::native::text_of(v).is_some_and(|t| !t.trim().is_empty()),
        Value::Integer(_)
        | Value::BigInteger(_)
        | Value::Date(..)
        | Value::Boolean(_)
        | Value::Enum(..)
        | Value::Option(..)
        | Value::Decimal(_) => true,
        _ => false,
    }
}

fn same_kind(a: &Value, b: &Value) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

impl Seek {
    /// The bounds the filters give for key `key` of `t`; `None` when they give none.
    pub fn of(t: &TableDef, key: usize, filters: &[FieldFilter], sample: Option<&Row>) -> Option<Seek> {
        use al_builtins::filter::{FilterExpr as E, OR_GROUP};
        if filters.is_empty() {
            return None;
        }
        let fields: Vec<usize> = if key == 0 {
            t.keys[0].clone()
        } else {
            t.keys[key].iter().chain(&t.keys[0]).copied().collect()
        };
        let mut seek = Seek::default();
        let (mut lo_open, mut hi_open) = (false, false);
        for &fld in &fields {
            // A Code field stored as a number sorts in another order than its filters compare: no bound from it on.
            if t.fields[fld].code_storage != CodeStorage::Text {
                break;
            }
            // The first usable lower and upper value of the filters on this field (all of them apply at once).
            let (mut lo, mut hi): (Option<&Value>, Option<&Value>) = (None, None);
            for f in filters.iter().filter(|f| f.field == fld && f.group != OR_GROUP) {
                let (l, h) = match &f.expr {
                    E::Eq(x) => (Some(x), Some(x)),
                    E::Range(l, h) => (l.as_ref(), h.as_ref()),
                    // A strict bound is taken as a closed one: wider, so never wrong.
                    E::Ge(x) | E::Gt(x) => (Some(x), None),
                    E::Le(x) | E::Lt(x) => (None, Some(x)),
                    _ => (None, None),
                };
                // Only a value of the kind the field holds in the rows: the order is the order of that kind.
                let fits = |v: &Value| usable_bound(v) && sample.is_none_or(|r| same_kind(v, &r[fld]));
                lo = lo.or(l.filter(|v| fits(v)));
                hi = hi.or(h.filter(|v| fits(v)));
            }
            let point = match (lo, hi) {
                (Some(l), Some(h)) => same_kind(l, h) && al_builtins::ops::compare(l, h).ok() == Some(Ordering::Equal),
                _ => false,
            };
            if !lo_open {
                match lo {
                    Some(l) => seek.lo.push(l.clone()),
                    None => lo_open = true,
                }
                lo_open |= !point;
            }
            if !hi_open {
                match hi {
                    Some(h) => seek.hi.push(h.clone()),
                    None => hi_open = true,
                }
                hi_open |= !point;
            }
            if lo_open && hi_open {
                break;
            }
        }
        seek.fields = fields;
        (!seek.lo.is_empty() || !seek.hi.is_empty()).then_some(seek)
    }

    /// How `row` stands to the bound `bound` (a prefix of a position): `None` when a value does not compare, in
    /// which case the caller does not trust the order and looks at the row.
    fn cmp_to(&self, row: &[Value], bound: &[Value]) -> Option<Ordering> {
        for (&i, b) in self.fields.iter().zip(bound) {
            if !same_kind(&row[i], b) {
                return None;
            }
            // The store orders text keys by `text_key_order`; the filters compare text by code points. The two agree
            // for upper-case letters and digits only: any other text is looked at row by row.
            if let (Value::Text(x) | Value::Code(x), Value::Text(y) | Value::Code(y)) = (&row[i], b) {
                let neutral = |t: &str| t.bytes().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
                if !neutral(x) || !neutral(y) {
                    return None;
                }
            }
            match al_builtins::ops::compare(&row[i], b) {
                Ok(Ordering::Equal) => {}
                Ok(o) => return Some(o),
                Err(_) => return None,
            }
        }
        Some(Ordering::Equal)
    }

    /// The row is before the first row the filters allow.
    pub fn before(&self, row: &[Value]) -> Option<bool> {
        if self.lo.is_empty() {
            return Some(false);
        }
        self.cmp_to(row, &self.lo).map(|o| o == Ordering::Less)
    }

    /// The row is after the last row the filters allow.
    pub fn after(&self, row: &[Value]) -> Option<bool> {
        if self.hi.is_empty() {
            return Some(false);
        }
        self.cmp_to(row, &self.hi).map(|o| o == Ordering::Greater)
    }
}

#[cfg(test)]
thread_local! {
    /// How many rows `passes` looked at: the tests of the key seek count what a scan reads.
    static ROWS_LOOKED_AT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The rows of `table` (in primary key order) that can pass `filters`: those between the bounds the filters on the
/// leading primary key fields give. A row of the band may still fail them, so the caller checks `passes`.
fn pk_band<'a>(
    table: &'a Table,
    t: &TableDef,
    filters: &[FieldFilter],
) -> Box<dyn Iterator<Item = (&'a Key, &'a Row)> + 'a> {
    use std::ops::Bound::{Included, Unbounded};
    let Some(seek) = Seek::of(t, 0, filters, table.values().next()) else {
        return Box::new(table.iter());
    };
    let lower = if seek.lo.is_empty() {
        Unbounded
    } else {
        Included(Key(seek.lo.clone()))
    };
    Box::new(
        table
            .range((lower, Unbounded))
            .take_while(move |(_, r)| seek.after(r) != Some(true)),
    )
}

fn passes(row: &Row, filters: &[FieldFilter]) -> bool {
    #[cfg(test)]
    ROWS_LOOKED_AT.with(|c| c.set(c.get() + 1));
    al_builtins::filter::row_passes(row, filters)
}

impl MemStore {
    /// A read that filters on, or orders by, a column a stored row holds unknown (a copy of a platform row) is refused:
    /// whether that row passes, or where it sorts, is not known. Costs nothing until such a row was stored.
    fn unknown_read(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        filters: &[FieldFilter],
        order: &[usize],
    ) -> Result<(), StoreError> {
        if self.unknown_cols.is_empty() {
            return Ok(());
        }
        let Some(held) = self.unknown_cols.get(&t.object) else {
            return Ok(());
        };
        let cols: Vec<usize> = filters
            .iter()
            .map(|f| f.field)
            .chain(order.iter().copied())
            .filter(|c| held.contains(c))
            .collect();
        if cols.is_empty() {
            return Ok(());
        }
        let Some(table) = self.table(cx, t) else {
            return Ok(());
        };
        for row in table.values() {
            if let Some(why) = cols.iter().find_map(|&c| row.get(c).and_then(Value::unknown)) {
                return Err(StoreError::Refused(why.to_string()));
            }
        }
        Ok(())
    }

    /// A read that filters a Code field stored as a number by anything but one value, or sorts by a Code field stored as a
    /// BigInteger, is refused: neither is pinned (see [`CodeStorage`]).
    fn code_storage_read(t: &TableDef, filters: &[FieldFilter], order: &[usize]) -> Result<(), StoreError> {
        use al_builtins::filter::FilterExpr as E;
        let stored_as = |i: usize| t.fields.get(i).map_or(CodeStorage::Text, |f| f.code_storage);
        if let Some(f) = filters.iter().find(|f| {
            stored_as(f.field) != CodeStorage::Text
                && !match &f.expr {
                    E::Eq(_) | E::Ne(_) | E::RangeSame(_) => true,
                    E::Range(Some(a), Some(b)) => al_builtins::ops::compare(a, b).ok() == Some(Ordering::Equal),
                    _ => false,
                }
        }) {
            return Err(StoreError::Refused(format!(
                "a filter other than one value on the Code field {}, which the database stores as a number (SQLDataType): its order is not pinned",
                t.fields[f.field].name
            )));
        }
        if let Some(&i) = order.iter().find(|&&i| stored_as(i) == CodeStorage::BigInteger) {
            return Err(StoreError::Refused(format!(
                "sorting by the Code field {}, which the database stores as a BigInteger (SQLDataType): its order is not pinned",
                t.fields[i].name
            )));
        }
        Ok(())
    }

    /// Writes down the columns of `row` that hold an unknown value (see `unknown_cols`).
    fn note_unknown(&mut self, t: &TableDef, row: &Row) {
        for (i, v) in row.iter().enumerate() {
            if v.unknown().is_some() {
                let held = self.unknown_cols.entry(t.object).or_default();
                if !held.contains(&i) {
                    held.push(i);
                }
            }
        }
    }

    /// A store whose virtual tables are answered by `v`.
    pub fn with_virtual_tables(v: std::rc::Rc<dyn VirtualTables>) -> MemStore {
        MemStore {
            virtuals: Some(v),
            ..Default::default()
        }
    }

    /// This store with `b` as the rows its untouched tables read.
    pub fn with_backing(mut self, b: std::rc::Rc<dyn Backing>) -> MemStore {
        self.backing = Some(b);
        self.alt_cache.borrow_mut().clear();
        self.order_cache.borrow_mut().clear();
        self
    }

    /// Starts (`true`) or stops (`false`) writing down the changes made to rows the backing holds.
    /// Changes are never refused (ADR-020 amendment). Starting clears the record.
    pub fn record_restored_changes(&mut self, on: bool) {
        self.restored_log = on.then(Default::default);
    }

    /// The changes written down since recording started.
    pub fn restored_changes(&self) -> Vec<RestoredChange> {
        self.restored_log
            .as_ref()
            .map(|l| l.borrow().clone())
            .unwrap_or_default()
    }

    /// Writes down a change to the row with this key if recording is on and the backing holds the
    /// row. `new` is the row a Modify writes.
    fn note_restored(&self, cx: &AccessContext, t: &TableDef, key: &Key, op: &'static str, new: Option<&Row>) {
        let Some(log) = &self.restored_log else { return };
        let slot = slot(cx, t);
        let Some(rows) = self.backing.as_ref().and_then(|b| b.rows(&slot.0, slot.1)) else {
            return;
        };
        let Some(old) = rows.0.get(key) else { return };
        let fields: Vec<String> = new
            .map(|n| {
                (0..old.len().min(n.len()))
                    .filter(|&i| format!("{:?}", n[i]) != format!("{:?}", old[i]))
                    // The system fields (id from 2000000000: SystemModifiedAt, SystemRowVersion ...) change
                    // with every write and do not count as a different value.
                    .filter_map(|i| t.fields.get(i).filter(|f| f.id < 2_000_000_000).map(|f| f.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let same = new.is_some() && fields.is_empty();
        log.borrow_mut().push(RestoredChange {
            table: t.name.clone(),
            op,
            same,
            fields,
        });
    }

    /// The rows table `object` has to offer `company`, if the store has a backing.
    pub fn backed_rows(&self, company: &str, object: u32) -> Option<Rows> {
        self.backing.as_ref()?.rows(company, object)
    }

    /// The store without its provider: the data only. A captured snapshot (ADR-004) holds no
    /// provider, so it cannot keep the session that made it alive.
    pub fn detached(mut self) -> MemStore {
        self.virtuals = None;
        self
    }

    /// The provider of the computed tables.
    pub fn virtual_tables(&self) -> Option<std::rc::Rc<dyn VirtualTables>> {
        self.virtuals.clone()
    }

    /// This store's data with `v` as its provider.
    pub fn attached(mut self, v: Option<std::rc::Rc<dyn VirtualTables>>) -> MemStore {
        self.virtuals = v;
        self
    }

    /// The rows a Modify stored for the computed table `t` (a provider answers them over its own).
    pub fn stored_rows(&self, cx: &AccessContext, t: &TableDef) -> Vec<Row> {
        bump();
        self.table(cx, t)
            .map(|tb| tb.values().cloned().collect())
            .unwrap_or_default()
    }

    /// The provider of table `t`, if its rows are computed.
    fn virtual_of(&self, t: &TableDef) -> Option<&dyn VirtualTables> {
        self.virtuals.as_deref().filter(|v| v.serves(t))
    }

    fn read_only(&self, t: &TableDef, what: &str) -> StoreError {
        if let Some(why) = self.virtuals.as_deref().and_then(|v| v.write_refusal(t)) {
            return StoreError::Refused(why);
        }
        StoreError::Refused(format!(
            "{what} on virtual table '{}': its rows are computed, not stored",
            t.name
        ))
    }

    /// Whether any write since the last commit or rollback is pending.
    pub fn has_uncommitted(&self) -> bool {
        !(self.tables.ptr_eq(&self.committed) || (self.tables.is_empty() && self.committed.is_empty()))
    }

    /// Whether table object `object` holds a row now (committed or not) in any company.
    pub fn has_rows(&self, object: u32) -> bool {
        if self.tables.iter().any(|((_, o), t)| *o == object && !t.is_empty()) {
            return true;
        }
        // A table the store holds is what its rows are now; only an absent one reads the backing.
        let held = self.tables.keys().any(|(_, o)| *o == object);
        !held && self.backing.as_ref().is_some_and(|b| b.has_rows(object))
    }

    /// The table objects that hold a committed row in any company.
    /// The committed rows of every table the store holds an entry for, for a snapshot on disk or for
    /// another thread (the rows are plain values; [`MemStore::from_exported`] is the way back).
    /// A store with a backing (test data) exports the tables it holds: those a write reached, which
    /// carry the backing's rows with the changes, and an emptied one stays empty. The tables it has
    /// no entry for are the backing's and are not exported; the importer puts the same backing back
    /// (`with_backing`). The record of changes to restored rows is a log and is not exported.
    /// Changes that are not committed are not exported.
    pub fn export_committed(&self) -> Option<Vec<ExportedTable>> {
        Some(
            self.committed
                .iter()
                .map(|((company, object), table)| ExportedTable {
                    company: company.to_string(),
                    object: *object,
                    rows: table.iter().map(|(k, r)| (k.0.clone(), r.clone())).collect(),
                })
                .collect(),
        )
    }

    /// A store whose committed (and current) data is `tables`, with no provider and no backing. Its stamps are new
    /// ones: a stamp only says whether a table changed, and no cache of this store knows an older one.
    pub fn from_exported(tables: Vec<ExportedTable>) -> MemStore {
        let mut s = MemStore::default();
        for t in tables {
            let slot: Slot = (std::sync::Arc::from(t.company.as_str()), t.object);
            let table: Table = t.rows.into_iter().map(|(k, r)| (Key(k), r)).collect();
            s.counter += 1;
            s.versions.insert(slot.clone(), s.counter);
            s.shapes.insert(slot.clone(), s.counter);
            s.tables.insert(slot, table);
        }
        s.committed = s.tables.clone();
        s.committed_versions = s.versions.clone();
        s.committed_shapes = s.shapes.clone();
        s
    }

    pub fn nonempty_tables(&self) -> std::collections::BTreeSet<u32> {
        self.committed
            .iter()
            .filter(|(_, t)| !t.is_empty())
            .map(|((_, object), _)| *object)
            .collect()
    }

    /// Whether `company` holds a row in any per-company table, committed or not.
    pub fn has_company_rows(&self, company: &str) -> bool {
        self.tables
            .iter()
            .any(|((c, _), t)| !c.is_empty() && &**c == company && !t.is_empty())
            || self
                .backing
                .as_ref()
                .is_some_and(|b| !company.is_empty() && b.company().eq_ignore_ascii_case(company))
    }

    /// The companies that hold a row in per-company table `object`, committed or not.
    pub fn companies_with_rows(&self, object: u32) -> Vec<String> {
        let mut out: Vec<String> = self
            .tables
            .iter()
            .filter(|((c, o), t)| *o == object && !c.is_empty() && !t.is_empty())
            .map(|((c, _), _)| c.to_string())
            .collect();
        if let Some(b) = &self.backing {
            let company = b.company();
            let held = self.tables.contains_key(&(std::sync::Arc::from(company), object));
            if !held && b.rows(company, object).is_some_and(|r| !r.is_empty()) {
                out.push(company.to_string());
            }
        }
        out
    }

    /// Removes every row of `company`'s tables, as deleting the company does. Part of the open
    /// transaction: a rollback puts the rows back.
    pub fn drop_company(&mut self, company: &str) {
        let doomed: Vec<Slot> = self
            .tables
            .keys()
            .filter(|(c, _)| !c.is_empty() && &**c == company)
            .cloned()
            .collect();
        for slot in doomed {
            self.tables.remove(&slot);
            self.counter += 1;
            self.versions.insert(slot.clone(), self.counter);
            self.shapes.insert(slot, self.counter);
        }
    }

    /// Moves every row of company `old` to company `new`, as renaming the company does
    /// (record/TestChangeCompanyFiltersCopyDeleteContracts.al:170-212). Part of the open transaction.
    pub fn rename_company(&mut self, old: &str, new: &str) {
        let moved: Vec<Slot> = self
            .tables
            .keys()
            .filter(|(c, _)| !c.is_empty() && &**c == old)
            .cloned()
            .collect();
        for slot in moved {
            if let Some(table) = self.tables.remove(&slot) {
                let target: Slot = (std::sync::Arc::from(new), slot.1);
                self.tables.insert(target.clone(), table);
                self.counter += 1;
                self.versions.insert(slot.clone(), self.counter);
                self.shapes.insert(slot, self.counter);
                self.counter += 1;
                self.versions.insert(target.clone(), self.counter);
                self.shapes.insert(target, self.counter);
            }
        }
    }

    /// A number that changes whenever the rows of table `t` change (a rollback included). Queries
    /// compare it to tell whether their result set is stale (corpus query/TestQueryReadAfterWrite.al).
    pub fn table_stamp(&self, cx: &AccessContext, t: &TableDef) -> u64 {
        if self.virtual_of(t).is_some() {
            return 0;
        }
        self.versions.get(&slot(cx, t)).copied().unwrap_or(0)
    }

    fn touch(&mut self, cx: &AccessContext, t: &TableDef) {
        self.counter += 1;
        self.versions.insert(slot(cx, t), self.counter);
    }

    /// `touch` for a change of the table's shape (see `shapes`).
    fn touch_shape(&mut self, cx: &AccessContext, t: &TableDef) {
        self.touch(cx, t);
        self.shapes.insert(slot(cx, t), self.counter);
    }

    fn table(&self, cx: &AccessContext, t: &TableDef) -> Option<std::borrow::Cow<'_, Table>> {
        let slot = slot(cx, t);
        match self.tables.get(&slot) {
            Some(table) => Some(std::borrow::Cow::Borrowed(table)),
            None => self
                .backing
                .as_ref()?
                .rows(&slot.0, slot.1)
                .map(|r| std::borrow::Cow::Owned(r.0)),
        }
    }

    /// The primary keys of `table` in the order of key `key`, kept while the shape of the table stays.
    fn order_of(&self, slot: &Slot, key: usize, t: &TableDef, table: &Table) -> std::rc::Rc<Vec<Key>> {
        let shape = self.shapes.get(slot).copied().unwrap_or(0);
        let ck = (slot.clone(), key);
        if let Some(e) = self.order_cache.borrow().get(&ck) {
            if e.shape == shape && e.rows == table.len() && e.fields == t.keys[key] {
                return e.order.clone();
            }
        }
        let mut rows: Vec<(&Key, &Row)> = table.iter().collect();
        rows.sort_by(|a, b| cmp_rows(t, key, a.1, b.1));
        let order = std::rc::Rc::new(rows.into_iter().map(|(k, _)| k.clone()).collect::<Vec<_>>());
        self.order_cache.borrow_mut().insert(
            ck,
            OrderEntry {
                shape,
                rows: table.len(),
                fields: t.keys[key].clone(),
                order: order.clone(),
            },
        );
        order
    }

    /// The band of key `j` that the filters leave, and how many rows it holds; `None` when the filters bound no
    /// leading field of the key or the order cannot be trusted.
    fn band_of(
        &self,
        slot: &Slot,
        j: usize,
        t: &TableDef,
        table: &Table,
        filters: &[FieldFilter],
    ) -> Option<(Band, usize)> {
        let seek = Seek::of(t, j, filters, table.values().next())?;
        if j == 0 {
            let n = pk_band(table, t, filters).count();
            return Some((Band::Pk, n));
        }
        let order = self.order_of(slot, j, t, table);
        if order.len() != table.len() || !order.iter().take(1).all(|k| table.get(k).is_some()) {
            return None;
        }
        let trusted = std::cell::Cell::new(true);
        let probe = |k: &Key, f: &dyn Fn(&[Value]) -> Option<bool>| match table.get(k).map(|r| f(r)) {
            Some(Some(b)) => b,
            _ => {
                trusted.set(false);
                false
            }
        };
        let start = order.partition_point(|k| probe(k, &|r| seek.before(r)));
        let end = order.partition_point(|k| !probe(k, &|r| seek.after(r)));
        (trusted.get() && start <= end).then(|| {
            let n = end - start;
            (Band::Order(order, start..end), n)
        })
    }

    /// A scan whose own key is not bounded by the filters, but another key is (an index the filters narrow, as SQL
    /// picks one): the rows that pass are found in that key's band and put in the order of the scan's key, once
    /// per table version and set of filters (the cache `alt_cache`). `None`: the caller walks.
    ///
    /// The first step of a scan reads the band; the second builds the sorted list; later steps search it. The step
    /// right after a write walks (a loop that writes in every step must not pay for a band in every step).
    fn alt_next(&self, cx: &AccessContext, t: &TableDef, scan: &Scan, table: &Table) -> Option<Option<Row>> {
        if scan.filters.is_empty() || Seek::of(t, scan.key, scan.filters, table.values().next()).is_some() {
            return None;
        }
        let sample = table.values().next();
        if !(0..t.keys.len()).any(|j| j != scan.key && Seek::of(t, j, scan.filters, sample).is_some()) {
            return None;
        }
        let slot = slot(cx, t);
        let version = self.versions.get(&slot).copied().unwrap_or(0);
        let ck = (slot.clone(), scan.key, format!("{:?}", scan.filters));
        let fields: Vec<usize> = if scan.key == 0 {
            t.keys[0].clone()
        } else {
            t.keys[scan.key].iter().chain(&t.keys[0]).copied().collect()
        };
        // How a row stands to the position `a`.
        let cmp_after = |r: &[Value], a: &[Value]| -> Ordering {
            for (&i, x) in fields.iter().zip(a) {
                match t.cmp_key_field(i, &r[i], x) {
                    Ordering::Equal => continue,
                    o => return o,
                }
            }
            Ordering::Equal
        };
        let after_ok = |r: &[Value]| {
            scan.after.is_none_or(|a| {
                let o = cmp_after(r, a);
                if scan.ascending {
                    o == Ordering::Greater
                } else {
                    o == Ordering::Less
                }
            })
        };
        let calls;
        {
            let mut cache = self.alt_cache.borrow_mut();
            if cache.len() > 256 {
                cache.clear();
            }
            match cache.get_mut(&ck) {
                Some(e) if e.version == version => {
                    if e.not_applicable {
                        return None;
                    }
                    if let Some(keys) = e.keys.clone() {
                        drop(cache);
                        return Some(self.search_sorted(table, &keys, scan, &cmp_after));
                    }
                    e.calls += 1;
                    calls = e.calls;
                }
                Some(e) => {
                    // The first step after a write: walk.
                    *e = AltEntry {
                        version,
                        calls: 1,
                        not_applicable: false,
                        keys: None,
                    };
                    return None;
                }
                None => {
                    cache.insert(
                        ck.clone(),
                        AltEntry {
                            version,
                            calls: 1,
                            not_applicable: false,
                            keys: None,
                        },
                    );
                    calls = 1;
                }
            }
        }
        // The narrowest band of any key, if it is a clear gain over the walk.
        let mut best: Option<(Band, usize)> = None;
        for j in 0..t.keys.len() {
            if let Some((b, n)) = self.band_of(&slot, j, t, table, scan.filters) {
                if best.as_ref().is_none_or(|(_, m)| n < *m) {
                    best = Some((b, n));
                }
            }
        }
        let Some((band, _)) = best.filter(|(_, n)| *n * 4 <= table.len()) else {
            if let Some(e) = self.alt_cache.borrow_mut().get_mut(&ck) {
                e.not_applicable = true;
            }
            return None;
        };
        let band_keys: Vec<Key> = match &band {
            Band::Pk => pk_band(table, t, scan.filters).map(|(k, _)| k.clone()).collect(),
            Band::Order(order, range) => order[range.clone()].to_vec(),
        };
        if calls < 2 {
            // The first step: the best row of the band.
            let mut best: Option<&Row> = None;
            for k in &band_keys {
                let Some(r) = table.get(k) else { continue };
                if !passes(r, scan.filters) || !after_ok(r) {
                    continue;
                }
                let better = best.is_none_or(|b| {
                    let o = cmp_rows(t, scan.key, r, b);
                    if scan.ascending {
                        o == Ordering::Less
                    } else {
                        o == Ordering::Greater
                    }
                });
                if better {
                    best = Some(r);
                }
            }
            return Some(best.cloned());
        }
        let mut rows: Vec<(&Key, &Row)> = band_keys
            .iter()
            .filter_map(|k| table.get_key_value(k))
            .filter(|(_, r)| passes(r, scan.filters))
            .collect();
        rows.sort_by(|a, b| cmp_rows(t, scan.key, a.1, b.1));
        let keys = std::rc::Rc::new(rows.into_iter().map(|(k, _)| k.clone()).collect::<Vec<_>>());
        if let Some(e) = self.alt_cache.borrow_mut().get_mut(&ck) {
            e.keys = Some(keys.clone());
        }
        Some(self.search_sorted(table, &keys, scan, &cmp_after))
    }

    /// The row after the position of `scan` in `keys` (the rows that pass, in the order of the scan's key).
    fn search_sorted(
        &self,
        table: &Table,
        keys: &[Key],
        scan: &Scan,
        cmp_after: &dyn Fn(&[Value], &[Value]) -> Ordering,
    ) -> Option<Row> {
        let at = |i: usize| table.get(&keys[i]);
        if scan.ascending {
            let start = scan.after.map_or(0, |a| {
                keys.partition_point(|k| table.get(k).is_some_and(|r| cmp_after(r, a) != Ordering::Greater))
            });
            (start < keys.len()).then(|| at(start).cloned()).flatten()
        } else {
            let end = scan.after.map_or(keys.len(), |a| {
                keys.partition_point(|k| table.get(k).is_some_and(|r| cmp_after(r, a) == Ordering::Less))
            });
            (end > 0).then(|| at(end - 1).cloned()).flatten()
        }
    }

    fn table_mut(&mut self, cx: &AccessContext, t: &TableDef) -> &mut Table {
        let slot = slot(cx, t);
        if !self.tables.contains_key(&slot) {
            let start = self
                .backing
                .as_ref()
                .and_then(|b| b.rows(&slot.0, slot.1))
                .map_or_else(Table::new, |r| r.0);
            self.tables.insert(slot.clone(), start);
        }
        self.tables.get_mut(&slot).expect("the slot was just inserted")
    }
}

thread_local! {
    static STORE_OPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// One more operation on a store of this thread.
#[inline(always)]
fn bump() {
    STORE_OPS.with(|c| c.set(c.get().wrapping_add(1)));
}

/// How many operations the stores of this thread have done (reads, writes, commits, rollbacks, restores). The VM's SQL
/// counters compare two of these to see store access that no record method it logged made (al-vm `sqlcount.rs`).
pub fn store_ops() -> u64 {
    STORE_OPS.with(|c| c.get())
}

impl Store for MemStore {
    fn insert(&mut self, cx: &AccessContext, t: &TableDef, row: Row) -> Result<InsertOutcome, StoreError> {
        bump();
        if self.virtual_of(t).is_some() {
            return Err(self.read_only(t, "Insert"));
        }
        self.note_unknown(t, &row);
        let k = Key(t.primary_key(&row));
        let table = self.table_mut(cx, t);
        if table.contains_key(&k) {
            return Ok(InsertOutcome::AlreadyExists);
        }
        table.insert(k, row);
        self.touch_shape(cx, t);
        Ok(InsertOutcome::Inserted)
    }

    fn get(&self, cx: &AccessContext, t: &TableDef, pk: &[Value]) -> Result<Option<Row>, StoreError> {
        bump();
        if let Some(v) = self.virtual_of(t) {
            return v.get(self, cx, t, pk);
        }
        Ok(self.table(cx, t).and_then(|m| m.get(&Key(pk.to_vec())).cloned()))
    }

    fn modify(&mut self, cx: &AccessContext, t: &TableDef, row: Row) -> Result<WriteOutcome, StoreError> {
        bump();
        if let Some(v) = self.virtual_of(t) {
            let writable = v.writable_columns(t);
            if !writable.is_empty() {
                let old = v.get(self, cx, t, &t.primary_key(&row))?;
                // The columns outside the writable ones (the system fields are the record's, not the row's) must be as served.
                let differs = |old: &Row, only_writable: bool| {
                    old.iter()
                        .zip(&row)
                        .zip(&t.fields)
                        .filter(|(_, f)| f.id < 2_000_000_000 && writable.contains(&f.id) == only_writable)
                        .any(|((a, b), _)| !al_builtins::ops::compare(a, b).is_ok_and(|o| o == Ordering::Equal))
                };
                if let Some(old) = old.filter(|old| !differs(old, false)) {
                    if differs(&old, true) {
                        let k = Key(t.primary_key(&row));
                        self.table_mut(cx, t).insert(k, row);
                        self.touch(cx, t);
                        self.virtual_write_pending = true;
                        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::FeatureKeyEnabledModify);
                    } else {
                        al_builtins::assumptions::assume(
                            al_builtins::assumptions::Assumption::FeatureKeyUnchangedModify,
                        );
                    }
                    return Ok(WriteOutcome::Done);
                }
            }
            return Err(self.read_only(t, "Modify"));
        }
        self.note_unknown(t, &row);
        let k = Key(t.primary_key(&row));
        self.note_restored(cx, t, &k, "Modify", Some(&row));
        let table = self.table_mut(cx, t);
        if !table.contains_key(&k) {
            return Ok(WriteOutcome::NotFound);
        }
        // A key is made of fields; when a Modify changes one the row stands elsewhere in that key.
        let moved = table.get(&k).is_some_and(|old| key_fields_differ(t, old, &row));
        table.insert(k, row);
        if moved {
            self.touch_shape(cx, t);
        } else {
            self.touch(cx, t);
        }
        Ok(WriteOutcome::Done)
    }

    fn delete(&mut self, cx: &AccessContext, t: &TableDef, pk: &[Value]) -> Result<WriteOutcome, StoreError> {
        bump();
        if self.virtual_of(t).is_some() {
            return Err(self.read_only(t, "Delete"));
        }
        self.note_restored(cx, t, &Key(pk.to_vec()), "Delete", None);
        Ok(match self.table_mut(cx, t).remove(&Key(pk.to_vec())) {
            Some(_) => {
                self.touch_shape(cx, t);
                WriteOutcome::Done
            }
            None => WriteOutcome::NotFound,
        })
    }

    fn rename(
        &mut self,
        cx: &AccessContext,
        t: &TableDef,
        old: &[Value],
        row: Row,
    ) -> Result<RenameOutcome, StoreError> {
        bump();
        if self.virtual_of(t).is_some() {
            return Err(self.read_only(t, "Rename"));
        }
        let new_k = Key(t.primary_key(&row));
        self.note_restored(cx, t, &Key(old.to_vec()), "Rename", None);
        let table = self.table_mut(cx, t);
        if !table.contains_key(&Key(old.to_vec())) {
            return Ok(RenameOutcome::NotFound);
        }
        if table.contains_key(&new_k) {
            return Ok(RenameOutcome::AlreadyExists);
        }
        table.remove(&Key(old.to_vec()));
        table.insert(new_k, row);
        self.touch_shape(cx, t);
        Ok(RenameOutcome::Renamed)
    }

    fn next(&self, cx: &AccessContext, t: &TableDef, scan: &Scan) -> Result<Option<Row>, StoreError> {
        bump();
        if let Some(v) = self.virtual_of(t) {
            return v.next(self, cx, t, scan);
        }
        self.unknown_read(cx, t, scan.filters, t.keys.get(scan.key).map_or(&[], |k| k.as_slice()))?;
        Self::code_storage_read(t, scan.filters, t.keys.get(scan.key).map_or(&[], |k| k.as_slice()))?;
        let Some(table) = self.table(cx, t) else {
            return Ok(None);
        };
        if table.len() >= INDEX_MIN_ROWS {
            if let Some(found) = self.alt_next(cx, t, scan, &table) {
                return Ok(found);
            }
        }
        // In primary key order the table is already in the order of the scan: step to the first row past
        // the position and look at the rows from there on until one passes the filters.
        if scan.key == 0 {
            use std::ops::Bound::{Excluded, Included, Unbounded};
            let k = t.keys[0].len();
            let after = scan.after.map(|a| Key(a[..k.min(a.len())].to_vec()));
            if scan.ascending {
                let seek = Seek::of(t, 0, scan.filters, table.values().next());
                let from = seek.as_ref().filter(|s| !s.lo.is_empty()).map(|s| Key(s.lo.clone()));
                let lower = match (after, from) {
                    (Some(a), Some(f)) if key_cmp(&a.0, &f.0) == Ordering::Less => Included(f),
                    (Some(a), _) => Excluded(a),
                    (None, Some(f)) => Included(f),
                    (None, None) => Unbounded,
                };
                return Ok(table
                    .range((lower, Unbounded))
                    .map(|(_, r)| r)
                    // Past the last row the filters allow (a row that does not compare is looked at).
                    .take_while(|r| seek.as_ref().is_none_or(|s| s.after(r) != Some(true)))
                    .find(|r| passes(r, scan.filters))
                    .cloned());
            }
            let upper = after.map_or(Unbounded, Excluded);
            return Ok(table
                .range((Unbounded, upper))
                .rev()
                .map(|(_, r)| r)
                .find(|r| passes(r, scan.filters))
                .cloned());
        }
        // A big table in another key: step through the table's rows in that key's order.
        if table.len() >= INDEX_MIN_ROWS {
            let slot = slot(cx, t);
            let order = self.order_of(&slot, scan.key, t, &table);
            let row_at = |k: &Key| table.get(k);
            if order.len() == table.len() && order.iter().take(1).all(|k| row_at(k).is_some()) {
                // The slice of the order the filters on the leading fields of the key leave (ADR-035).
                let seek = Seek::of(t, scan.key, scan.filters, table.values().next());
                let mut band = 0..order.len();
                if let Some(sk) = &seek {
                    let trusted = std::cell::Cell::new(true);
                    let probe = |k: &Key, f: &dyn Fn(&[Value]) -> Option<bool>| match row_at(k).map(|r| f(r)) {
                        Some(Some(b)) => b,
                        _ => {
                            trusted.set(false);
                            false
                        }
                    };
                    let start = order.partition_point(|k| probe(k, &|r| sk.before(r)));
                    let end = order.partition_point(|k| !probe(k, &|r| sk.after(r)));
                    // Rows that do not compare: the order cannot be trusted, so the whole of it is looked at.
                    if trusted.get() && start <= end {
                        band = start..end;
                    }
                }
                let found = if scan.ascending {
                    let start = scan.after.map_or(0, |a| {
                        order.partition_point(|k| {
                            row_at(k).is_some_and(|r| cmp_row_to_position(t, scan.key, r, a) != Ordering::Greater)
                        })
                    });
                    order[start.max(band.start).min(band.end)..band.end]
                        .iter()
                        .filter_map(row_at)
                        .find(|r| passes(r, scan.filters))
                } else {
                    let end = scan.after.map_or(order.len(), |a| {
                        order.partition_point(|k| {
                            row_at(k).is_some_and(|r| cmp_row_to_position(t, scan.key, r, a) == Ordering::Less)
                        })
                    });
                    order[band.start.min(end)..end.min(band.end)]
                        .iter()
                        .rev()
                        .filter_map(row_at)
                        .find(|r| passes(r, scan.filters))
                };
                return Ok(found.cloned());
            }
        }
        // A small table in another key: every row is looked at, without building its position (a copy of the key fields).
        let mut best: Option<&Row> = None;
        for row in table.values().filter(|r| passes(r, scan.filters)) {
            let after_ok = scan.after.is_none_or(|a| {
                let o = cmp_row_to_position(t, scan.key, row, a);
                if scan.ascending {
                    o == Ordering::Greater
                } else {
                    o == Ordering::Less
                }
            });
            if !after_ok {
                continue;
            }
            let better = best.is_none_or(|b| {
                let o = cmp_rows(t, scan.key, row, b);
                if scan.ascending {
                    o == Ordering::Less
                } else {
                    o == Ordering::Greater
                }
            });
            if better {
                best = Some(row);
            }
        }
        Ok(best.cloned())
    }

    fn count(&self, cx: &AccessContext, t: &TableDef, f: &[FieldFilter]) -> Result<usize, StoreError> {
        bump();
        if let Some(v) = self.virtual_of(t) {
            return v.count(self, cx, t, f);
        }
        self.unknown_read(cx, t, f, &[])?;
        Self::code_storage_read(t, f, &[])?;
        Ok(self
            .table(cx, t)
            .map_or(0, |m| pk_band(&m, t, f).filter(|(_, r)| passes(r, f)).count()))
    }

    fn delete_all(&mut self, cx: &AccessContext, t: &TableDef, f: &[FieldFilter]) -> Result<usize, StoreError> {
        bump();
        if self.virtual_of(t).is_some() {
            return Err(self.read_only(t, "DeleteAll"));
        }
        self.unknown_read(cx, t, f, &[])?;
        Self::code_storage_read(t, f, &[])?;
        let doomed: Vec<Key> = self
            .table(cx, t)
            .map(|table| {
                pk_band(&table, t, f)
                    .filter(|(_, r)| passes(r, f))
                    .map(|(k, _)| k.clone())
                    .collect()
            })
            .unwrap_or_default();
        for k in &doomed {
            self.note_restored(cx, t, k, "DeleteAll", None);
        }
        let table = self.table_mut(cx, t);
        for k in &doomed {
            table.remove(k);
        }
        if !doomed.is_empty() {
            self.touch_shape(cx, t);
        }
        Ok(doomed.len())
    }

    fn commit(&mut self) {
        bump();
        self.virtual_write_pending = false;
        self.committed = self.tables.clone();
        self.committed_versions = self.versions.clone();
        self.committed_shapes = self.shapes.clone();
    }

    fn rollback(&mut self) {
        bump();
        self.virtual_write_pending = false;
        self.tables = self.committed.clone();
        self.versions = self.committed_versions.clone();
        self.shapes = self.committed_shapes.clone();
    }
}

impl MemStore {
    /// A virtual table took a change since the last commit (see the field).
    pub fn virtual_write_pending(&self) -> bool {
        self.virtual_write_pending
    }
}

impl SnapshotStore for MemStore {
    type Snapshot = MemStore;
    fn snapshot(&self) -> MemStore {
        self.clone()
    }
    fn restore(&mut self, s: MemStore) {
        bump();
        let counter = self.counter.max(s.counter);
        *self = s;
        self.counter = counter;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- the key seek (ADR-035): a scan starts where the filters on the leading key fields start ----

    use al_builtins::filter::{FieldFilter, FilterExpr};
    use al_builtins::value::RtType;

    fn seek_table(keys: Vec<Vec<usize>>) -> TableDef {
        let f = |id, name: &str, ty| FieldDef {
            id,
            name: name.into(),
            ty,
            stored: true,
            code_storage: Default::default(),
        };
        TableDef {
            object: 9,
            name: "Seek".into(),
            fields: vec![
                f(1, "A", RtType::Integer),
                f(2, "B", RtType::Integer),
                f(3, "C", RtType::Code(Some(10))),
                f(4, "D", RtType::Integer),
            ],
            keys,
            per_company: true,
        }
    }

    fn cx() -> AccessContext {
        AccessContext {
            company: "C".into(),
            user: "U".into(),
        }
    }

    fn range(field: usize, lo: Option<Value>, hi: Option<Value>) -> FieldFilter {
        FieldFilter {
            field,
            expr: FilterExpr::Range(lo, hi),
            group: 0,
        }
    }

    /// The rows a scan returns, one `next` after the other.
    fn scan_all(s: &MemStore, t: &TableDef, key: usize, filters: &[FieldFilter], ascending: bool) -> Vec<Row> {
        let mut out: Vec<Row> = Vec::new();
        let mut after: Option<Vec<Value>> = None;
        while let Some(r) = s
            .next(
                &cx(),
                t,
                &Scan {
                    key,
                    filters,
                    ascending,
                    after: after.as_deref(),
                },
            )
            .unwrap()
        {
            after = Some(crate::position(t, key, &r));
            out.push(r);
        }
        out
    }

    /// What the scan has to return, without any seek: every row, in the order of the key, that passes.
    fn reference(s: &MemStore, t: &TableDef, key: usize, filters: &[FieldFilter], ascending: bool) -> Vec<Row> {
        let mut rows: Vec<Row> = s.table(&cx(), t).unwrap().values().cloned().collect();
        rows.sort_by(|a, b| cmp_rows(t, key, a, b));
        if !ascending {
            rows.reverse();
        }
        rows.retain(|r| al_builtins::filter::row_passes(r, filters));
        rows
    }

    #[test]
    fn a_filter_on_the_leading_key_fields_makes_the_scan_read_only_the_rows_near_it() {
        let t = seek_table(vec![vec![0, 1], vec![3, 0, 1]]);
        let mut s = MemStore::default();
        for a in 0..200 {
            for b in 0..100 {
                let row = vec![
                    Value::Integer(a),
                    Value::Integer(b),
                    Value::Code("X".into()),
                    Value::Integer(b % 7),
                ];
                s.insert(&cx(), &t, row).unwrap();
            }
        }
        let looked = || ROWS_LOOKED_AT.with(|c| c.get());
        // Primary key: A = 150 is the 150th hundred of 20000 rows.
        let f = [range(0, Some(Value::Integer(150)), Some(Value::Integer(150)))];
        let before = looked();
        let first = s
            .next(
                &cx(),
                &t,
                &Scan {
                    key: 0,
                    filters: &f,
                    ascending: true,
                    after: None,
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(format!("{:?}", first[0]), "Integer(150)");
        assert!(
            looked() - before <= 5,
            "primary key: {} rows looked at",
            looked() - before
        );
        // Another key (D, A, B): D = 6 holds one row in seven, and the scan has to find the first of them.
        let f = [
            range(3, Some(Value::Integer(6)), Some(Value::Integer(6))),
            range(0, Some(Value::Integer(190)), None),
        ];
        let before = looked();
        let first = s
            .next(
                &cx(),
                &t,
                &Scan {
                    key: 1,
                    filters: &f,
                    ascending: true,
                    after: None,
                },
            )
            .unwrap()
            .unwrap();
        assert_eq!(format!("{:?} {:?}", first[3], first[0]), "Integer(6) Integer(190)");
        assert!(
            looked() - before <= 5,
            "other key: {} rows looked at",
            looked() - before
        );
    }

    #[test]
    fn a_key_slot_that_names_another_key_later_is_ordered_again() {
        // An ad-hoc key (SetCurrentKey on other fields) takes a slot of the key list; the same slot number then
        // stands for different fields, and the cached order of the first has to be left (found in Tests-ERM,
        // codeunit 138025: a seek on a stale order found no Value Entry).
        let by_b = seek_table(vec![vec![0], vec![1, 0]]);
        let by_d = seek_table(vec![vec![0], vec![3, 0]]);
        let mut s = MemStore::default();
        for a in 0..100 {
            let row = vec![
                Value::Integer(a),
                Value::Integer(a % 10),
                Value::Code("X".into()),
                Value::Integer(99 - a),
            ];
            s.insert(&cx(), &by_b, row).unwrap();
        }
        let f = [range(3, Some(Value::Integer(5)), Some(Value::Integer(5)))];
        assert_eq!(scan_all(&s, &by_b, 1, &[], true).len(), 100);
        assert_eq!(
            format!("{:?}", scan_all(&s, &by_d, 1, &f, true)),
            format!("{:?}", reference(&s, &by_d, 1, &f, true))
        );
        assert_eq!(scan_all(&s, &by_d, 1, &f, true).len(), 1);
        assert_eq!(
            format!("{:?}", scan_all(&s, &by_d, 1, &f, false)),
            format!("{:?}", reference(&s, &by_d, 1, &f, false))
        );
    }

    #[test]
    fn a_scan_in_one_key_uses_the_band_of_another_key_the_filters_bound() {
        // Filters on D, the leading field of key 1, while the scan runs in the primary key (A, B): the walk reads
        // every row of the table for each step; the band of key 1 holds the 400 rows that can pass.
        let t = seek_table(vec![vec![0, 1], vec![3, 0, 1]]);
        let mut s = MemStore::default();
        for a in 0..200 {
            for b in 0..100 {
                let row = vec![
                    Value::Integer(a),
                    Value::Integer(b),
                    Value::Code("X".into()),
                    Value::Integer(b % 50),
                ];
                s.insert(&cx(), &t, row).unwrap();
            }
        }
        let f = [range(3, Some(Value::Integer(6)), Some(Value::Integer(6)))];
        let before = ROWS_LOOKED_AT.with(|c| c.get());
        let got = scan_all(&s, &t, 0, &f, true);
        let looked = ROWS_LOOKED_AT.with(|c| c.get()) - before;
        assert_eq!(got.len(), 400);
        assert!(looked <= 1500, "{looked} rows looked at for a scan of 400 rows");
        assert_eq!(format!("{got:?}"), format!("{:?}", reference(&s, &t, 0, &f, true)));
    }

    #[test]
    fn a_scan_through_another_keys_band_follows_writes_between_the_steps() {
        let mut seed = 0xD1B5_4A32_D192_ED03u64;
        let mut rnd = move |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % n
        };
        for round in 0..30 {
            let t = seek_table(vec![vec![0, 1], vec![3, 0, 1], vec![2, 3, 0, 1]]);
            let mut s = MemStore::default();
            for _ in 0..400 {
                let row = vec![
                    Value::Integer(rnd(30) as i32),
                    Value::Integer(rnd(30) as i32),
                    Value::Code(["A", "B", "ab", ""][rnd(4) as usize].into()),
                    Value::Integer(rnd(40) as i32),
                ];
                let _ = s.insert(&cx(), &t, row);
            }
            let key = (round % 3) as usize;
            let ascending = round % 2 == 0;
            let lo = rnd(40) as i32;
            let filters = vec![
                range(3, Some(Value::Integer(lo)), Some(Value::Integer(lo + (round % 4)))),
                FieldFilter {
                    field: 1,
                    expr: FilterExpr::Ne(Value::Integer(7)),
                    group: 0,
                },
            ];
            let mut after: Option<Vec<Value>> = None;
            for step in 0..300 {
                // A write between some steps; several reads in a row between others.
                if rnd(4) == 0 {
                    let row = vec![
                        Value::Integer(rnd(30) as i32),
                        Value::Integer(rnd(30) as i32),
                        Value::Code(["A", "B", "ab", ""][rnd(4) as usize].into()),
                        Value::Integer(rnd(40) as i32),
                    ];
                    match rnd(3) {
                        0 => {
                            let _ = s.insert(&cx(), &t, row);
                        }
                        1 => {
                            let _ = s.modify(&cx(), &t, row);
                        }
                        _ => {
                            let _ = s.delete(&cx(), &t, &[row[0].clone(), row[1].clone()]);
                        }
                    }
                }
                let got = s
                    .next(
                        &cx(),
                        &t,
                        &Scan {
                            key,
                            filters: &filters,
                            ascending,
                            after: after.as_deref(),
                        },
                    )
                    .unwrap();
                let expect = reference(&s, &t, key, &filters, ascending).into_iter().find(|r| {
                    after.as_ref().is_none_or(|a| {
                        let o = key_cmp(&crate::position(&t, key, r), a);
                        if ascending {
                            o == Ordering::Greater
                        } else {
                            o == Ordering::Less
                        }
                    })
                });
                assert_eq!(
                    format!("{got:?}"),
                    format!("{expect:?}"),
                    "round {round} step {step} key {key} ascending {ascending}"
                );
                after = got.map(|r| crate::position(&t, key, &r));
            }
        }
    }

    #[test]
    fn a_seek_returns_the_rows_a_walk_returns_in_the_same_order() {
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        let mut rnd = move |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % n
        };
        let codes = ["", "A", "B", "ab", "C", " ", "  ", "Z"];
        for round in 0..60 {
            let t = seek_table(vec![vec![0, 2, 1], vec![3, 2, 0, 1]]);
            let mut s = MemStore::default();
            let n = if round % 3 == 0 { 20 } else { 300 };
            for _ in 0..n {
                let row = vec![
                    Value::Integer(rnd(6) as i32),
                    Value::Integer(rnd(6) as i32),
                    Value::Code(codes[rnd(codes.len() as u64) as usize].into()),
                    Value::Integer(rnd(4) as i32),
                ];
                let _ = s.insert(&cx(), &t, row);
            }
            for _ in 0..40 {
                let mut filters = Vec::new();
                for _ in 0..rnd(4) {
                    let field = rnd(4) as usize;
                    let v = |r: u64| -> Value {
                        if field == 2 {
                            Value::Code(codes[(r % codes.len() as u64) as usize].into())
                        } else {
                            Value::Integer((r % 7) as i32)
                        }
                    };
                    let expr = match rnd(9) {
                        0 => FilterExpr::Eq(v(rnd(100))),
                        1 => FilterExpr::Range(Some(v(rnd(100))), None),
                        2 => FilterExpr::Range(None, Some(v(rnd(100)))),
                        3 => FilterExpr::Range(Some(v(rnd(100))), Some(v(rnd(100)))),
                        4 => FilterExpr::Gt(v(rnd(100))),
                        5 => FilterExpr::Lt(v(rnd(100))),
                        6 => FilterExpr::Ne(v(rnd(100))),
                        7 => FilterExpr::Or(vec![FilterExpr::Eq(v(rnd(100))), FilterExpr::Eq(v(rnd(100)))]),
                        _ => FilterExpr::Le(v(rnd(100))),
                    };
                    let group = [0, 0, 0, 2, -1][rnd(5) as usize];
                    filters.push(FieldFilter { field, expr, group });
                }
                for key in 0..2 {
                    for ascending in [true, false] {
                        assert_eq!(
                            format!("{:?}", scan_all(&s, &t, key, &filters, ascending)),
                            format!("{:?}", reference(&s, &t, key, &filters, ascending)),
                            "key {key} ascending {ascending} filters {filters:?}"
                        );
                    }
                }
                assert_eq!(
                    s.count(&cx(), &t, &filters).unwrap(),
                    reference(&s, &t, 0, &filters, true).len()
                );
            }
        }
    }

    #[test]
    fn passes_the_store_contract() {
        crate::contract::run_all(MemStore::default);
    }

    #[test]
    fn dropping_a_company_removes_its_rows_and_a_rollback_restores_them() {
        let cx = |c: &str| AccessContext {
            company: c.into(),
            user: "U".into(),
        };
        let t = TableDef {
            object: 7,
            name: "T".into(),
            fields: vec![FieldDef {
                id: 1,
                name: "K".into(),
                ty: al_builtins::value::RtType::Integer,
                stored: true,
                code_storage: Default::default(),
            }],
            keys: vec![vec![0]],
            per_company: true,
        };
        let mut s = MemStore::default();
        s.insert(&cx("A"), &t, vec![Value::Integer(1)]).unwrap();
        s.insert(&cx("B"), &t, vec![Value::Integer(1)]).unwrap();
        s.commit();
        assert!(
            s.has_company_rows("A") && !s.has_company_rows("a"),
            "company names are case sensitive"
        );
        assert_eq!(s.companies_with_rows(7).len(), 2);
        let before = s.table_stamp(&cx("B"), &t);
        s.drop_company("B");
        assert!(!s.has_company_rows("B") && s.has_company_rows("A"));
        assert_eq!(s.count(&cx("B"), &t, &[]).unwrap(), 0);
        assert_eq!(s.count(&cx("A"), &t, &[]).unwrap(), 1);
        assert_ne!(s.table_stamp(&cx("B"), &t), before, "queries see the change");
        s.rollback();
        assert_eq!(s.count(&cx("B"), &t, &[]).unwrap(), 1);
        s.rename_company("B", "C");
        assert_eq!(s.count(&cx("B"), &t, &[]).unwrap(), 0);
        assert_eq!(s.count(&cx("C"), &t, &[]).unwrap(), 1);
        s.rollback();
        assert_eq!(s.count(&cx("B"), &t, &[]).unwrap(), 1);
    }

    #[test]
    fn reports_uncommitted_writes() {
        let cx = AccessContext {
            company: "C".into(),
            user: "U".into(),
        };
        let t = TableDef {
            object: 0,
            name: "T".into(),
            fields: vec![FieldDef {
                id: 1,
                name: "K".into(),
                ty: al_builtins::value::RtType::Integer,
                stored: true,
                code_storage: Default::default(),
            }],
            keys: vec![vec![0]],
            per_company: true,
        };
        let mut s = MemStore::default();
        assert!(!s.has_uncommitted());
        s.insert(&cx, &t, vec![Value::Integer(1)]).unwrap();
        assert!(s.has_uncommitted());
        s.commit();
        assert!(!s.has_uncommitted());
        s.insert(&cx, &t, vec![Value::Integer(2)]).unwrap();
        s.rollback();
        assert!(!s.has_uncommitted());
    }

    #[test]
    fn table_stamp_changes_with_every_write_and_rollback() {
        let cx = AccessContext {
            company: "C".into(),
            user: "U".into(),
        };
        let t = TableDef {
            object: 0,
            name: "T".into(),
            fields: vec![FieldDef {
                id: 1,
                name: "K".into(),
                ty: al_builtins::value::RtType::Integer,
                stored: true,
                code_storage: Default::default(),
            }],
            keys: vec![vec![0]],
            per_company: true,
        };
        let mut s = MemStore::default();
        let s0 = s.table_stamp(&cx, &t);
        s.insert(&cx, &t, vec![Value::Integer(1)]).unwrap();
        let s1 = s.table_stamp(&cx, &t);
        assert_ne!(s0, s1, "an insert changes the stamp");
        s.insert(&cx, &t, vec![Value::Integer(1)]).unwrap();
        assert_eq!(s1, s.table_stamp(&cx, &t), "a refused insert does not");
        s.commit();
        s.insert(&cx, &t, vec![Value::Integer(2)]).unwrap();
        let s2 = s.table_stamp(&cx, &t);
        s.rollback();
        assert_eq!(s1, s.table_stamp(&cx, &t), "a rollback restores the committed stamp");
        s.insert(&cx, &t, vec![Value::Integer(3)]).unwrap();
        assert_ne!(s2, s.table_stamp(&cx, &t), "a stamp is never reused for other content");
    }

    /// Rows a table holds before anything wrote to it: three rows for company `C`, table 7.
    struct ThreeRows {
        reads: std::cell::Cell<usize>,
    }

    impl Backing for ThreeRows {
        fn company(&self) -> &str {
            "C"
        }
        fn rows(&self, company: &str, object: u32) -> Option<Rows> {
            if company != "C" || object != 7 {
                return None;
            }
            self.reads.set(self.reads.get() + 1);
            let t = crate::contract::table();
            Rows::from_rows(
                &t,
                vec![
                    crate::contract::row("A", 1, 1),
                    crate::contract::row("B", 2, 1),
                    crate::contract::row("C", 3, 2),
                ],
            )
            .ok()
        }
    }

    fn backed() -> (MemStore, std::rc::Rc<ThreeRows>) {
        let b = std::rc::Rc::new(ThreeRows {
            reads: Default::default(),
        });
        (MemStore::default().with_backing(b.clone()), b)
    }

    #[test]
    fn a_table_nothing_wrote_to_reads_the_backing_rows() {
        let (s, _) = backed();
        let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
        assert_eq!(s.count(&cx, &t, &[]).unwrap(), 3);
        assert!(s.get(&cx, &t, &[Value::Code("B".into())]).unwrap().is_some());
        // another company and another table see nothing
        assert_eq!(s.count(&crate::contract::cx("D"), &t, &[]).unwrap(), 0);
        let mut other = t.clone();
        other.object = 8;
        assert_eq!(s.count(&cx, &other, &[]).unwrap(), 0);
    }

    #[test]
    fn a_write_starts_from_the_backing_rows() {
        let (mut s, _) = backed();
        let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
        assert_eq!(
            s.insert(&cx, &t, crate::contract::row("A", 9, 9)).unwrap(),
            InsertOutcome::AlreadyExists,
            "a restored key cannot be inserted again"
        );
        assert_eq!(
            s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap(),
            InsertOutcome::Inserted
        );
        assert_eq!(s.count(&cx, &t, &[]).unwrap(), 4);
        assert_eq!(
            s.delete(&cx, &t, &[Value::Code("A".into())]).unwrap(),
            WriteOutcome::Done
        );
        assert_eq!(s.count(&cx, &t, &[]).unwrap(), 3);
    }

    #[test]
    fn deleting_every_restored_row_leaves_the_table_empty_not_restored_again() {
        let (mut s, _) = backed();
        let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
        assert_eq!(s.delete_all(&cx, &t, &[]).unwrap(), 3);
        assert_eq!(s.count(&cx, &t, &[]).unwrap(), 0);
        s.commit();
        s.rollback();
        assert_eq!(s.count(&cx, &t, &[]).unwrap(), 0);
    }

    #[test]
    fn a_rollback_puts_the_backing_rows_back_and_a_commit_keeps_a_write() {
        let (mut s, _) = backed();
        let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
        s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
        s.rollback();
        assert_eq!(
            s.count(&cx, &t, &[]).unwrap(),
            3,
            "the restored rows are committed data"
        );
        s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
        s.commit();
        s.rollback();
        assert_eq!(s.count(&cx, &t, &[]).unwrap(), 4);
    }

    #[test]
    fn a_snapshot_taken_before_a_touch_still_reads_the_backing_rows() {
        let (s, _) = backed();
        let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
        let snap = s.snapshot();
        let mut s = s;
        s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
        s.restore(snap);
        assert_eq!(s.count(&cx, &t, &[]).unwrap(), 3);
    }

    #[test]
    fn the_company_and_table_queries_include_the_backing_rows() {
        let (s, _) = backed();
        assert!(s.has_rows(7) && !s.has_rows(8));
        assert!(s.has_company_rows("c") && !s.has_company_rows("D"));
        assert_eq!(s.companies_with_rows(7), vec!["C".to_string()]);
    }

    #[test]
    fn changes_to_backing_rows_are_allowed_and_written_down_when_recording() {
        let (mut s, _) = backed();
        let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
        s.record_restored_changes(true);
        // the same values: allowed, and said to be the same
        let same = s.get(&cx, &t, &[Value::Code("A".into())]).unwrap().unwrap();
        assert_eq!(s.modify(&cx, &t, same).unwrap(), WriteOutcome::Done);
        // other values, a delete, a rename, a DeleteAll: allowed
        assert_eq!(
            s.modify(&cx, &t, crate::contract::row("A", 9, 9)).unwrap(),
            WriteOutcome::Done
        );
        assert_eq!(
            s.rename(&cx, &t, &[Value::Code("B".into())], crate::contract::row("Z", 1, 1))
                .unwrap(),
            RenameOutcome::Renamed
        );
        // a row the code made itself is not a restored row
        s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
        s.modify(&cx, &t, crate::contract::row("D", 5, 3)).unwrap();
        s.delete(&cx, &t, &[Value::Code("D".into())]).unwrap();
        let got: Vec<(&str, bool)> = s.restored_changes().iter().map(|c| (c.op, c.same)).collect();
        assert_eq!(got, vec![("Modify", true), ("Modify", false), ("Rename", false)]);
        assert!(s.restored_changes().iter().all(|c| c.table == t.name));
        s.delete_all(&cx, &t, &[]).unwrap();
        assert_eq!(s.count(&cx, &t, &[]).unwrap(), 0);
        s.record_restored_changes(false);
        assert!(s.restored_changes().is_empty());
    }

    /// Walking a table row by row costs a few comparisons a row, not a pass over the table: 30,000 rows in a
    /// second, where a scan that looks at every row for each step needs minutes. Release only
    /// (`cargo test --release -p al-data -- --ignored`).
    #[test]
    #[ignore]
    fn walking_a_big_table_row_by_row_is_not_quadratic() {
        let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
        let mut s = MemStore::default();
        for i in 0..30_000 {
            s.insert(&cx, &t, crate::contract::row(&format!("{i:08}"), i, i % 7))
                .unwrap();
        }
        let started = std::time::Instant::now();
        for key in [0, 1] {
            let rows = crate::contract::scan_all(&s, &cx, &t, key, &[], true);
            assert_eq!(rows.len(), 30_000);
        }
        let took = started.elapsed();
        assert!(took < std::time::Duration::from_secs(5), "two walks took {took:?}");
    }

    #[test]
    fn the_backing_is_asked_again_only_while_the_slot_is_absent() {
        let (mut s, b) = backed();
        let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
        s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
        let after_write = b.reads.get();
        for _ in 0..5 {
            s.count(&cx, &t, &[]).unwrap();
        }
        assert_eq!(
            b.reads.get(),
            after_write,
            "a table the store holds is not read from the backing again"
        );
    }

    #[test]
    fn passes_the_snapshot_contract() {
        crate::contract::snapshot_restores_everything(MemStore::default());
    }

    // ---- export and import of the committed rows (the install baseline's snapshot on disk) ----

    #[test]
    fn exported_rows_import_into_a_store_that_reads_and_writes_like_the_original() {
        let t = seek_table(vec![vec![0, 1], vec![3, 0, 1]]);
        let mut s = MemStore::default();
        for a in 0..70 {
            s.insert(
                &cx(),
                &t,
                vec![
                    Value::Integer(a),
                    Value::Integer(a % 3),
                    Value::Code("X".into()),
                    Value::Integer(a % 5),
                ],
            )
            .unwrap();
        }
        s.commit();
        // A change that is not committed is not part of the export.
        s.insert(
            &cx(),
            &t,
            vec![
                Value::Integer(999),
                Value::Integer(0),
                Value::Code("X".into()),
                Value::Integer(0),
            ],
        )
        .unwrap();
        let exported = s.export_committed().expect("nothing backs this store");
        assert_eq!(exported.len(), 1);
        assert_eq!(
            (exported[0].company.as_str(), exported[0].object, exported[0].rows.len()),
            ("C", 9, 70)
        );
        let mut back = MemStore::from_exported(exported);
        s.rollback();
        for key in 0..2 {
            assert_eq!(
                format!("{:?}", scan_all(&back, &t, key, &[], true)),
                format!("{:?}", scan_all(&s, &t, key, &[], true))
            );
        }
        assert_eq!(back.count(&cx(), &t, &[]).unwrap(), 70);
        // It is a store like any other: a write is seen, a rollback takes it away, a commit keeps it.
        back.insert(
            &cx(),
            &t,
            vec![
                Value::Integer(500),
                Value::Integer(0),
                Value::Code("X".into()),
                Value::Integer(0),
            ],
        )
        .unwrap();
        assert_eq!(back.count(&cx(), &t, &[]).unwrap(), 71);
        back.rollback();
        assert_eq!(back.count(&cx(), &t, &[]).unwrap(), 70);
        let before = back.table_stamp(&cx(), &t);
        back.insert(
            &cx(),
            &t,
            vec![
                Value::Integer(501),
                Value::Integer(0),
                Value::Code("X".into()),
                Value::Integer(0),
            ],
        )
        .unwrap();
        assert_ne!(back.table_stamp(&cx(), &t), before, "a write changes the table's stamp");
    }

    #[test]
    fn a_backed_store_exports_the_tables_it_holds_and_not_the_backing() {
        let (mut s, b) = backed();
        let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
        let mut other = t.clone();
        other.object = 8;
        // Nothing wrote to table 9: it reads the backing and is not part of the export.
        assert_eq!(s.count(&cx, &t, &[]).unwrap(), 3);
        assert!(s.export_committed().expect("a backing is no obstacle").is_empty());
        // A write copies the backing's rows into the store; deleting them all leaves an empty held table.
        s.insert(&cx, &t, crate::contract::row("D", 4, 3)).unwrap();
        s.delete(&cx, &t, &[Value::Code("A".into())]).unwrap();
        s.commit();
        let exported = s.export_committed().unwrap();
        assert_eq!(exported.len(), 1);
        assert_eq!(exported[0].rows.len(), 3, "B, C and D");
        // The way back: the same backing, and the held table now says what it said.
        let back = MemStore::from_exported(exported).with_backing(b);
        assert_eq!(back.count(&cx, &t, &[]).unwrap(), 3);
        assert!(back.get(&cx, &t, &[Value::Code("A".into())]).unwrap().is_none());
        assert!(back.get(&cx, &t, &[Value::Code("D".into())]).unwrap().is_some());
        assert_eq!(back.count(&cx, &other, &[]).unwrap(), 0);
    }

    #[test]
    fn a_deleted_out_backed_table_stays_empty_after_the_way_back() {
        let (mut s, b) = backed();
        let (t, cx) = (crate::contract::table(), crate::contract::cx("C"));
        s.delete_all(&cx, &t, &[]).unwrap();
        s.commit();
        let back = MemStore::from_exported(s.export_committed().unwrap()).with_backing(b);
        assert_eq!(back.count(&cx, &t, &[]).unwrap(), 0, "not restored again");
    }
}

#[cfg(test)]
mod virtual_tests {
    use super::*;
    use al_builtins::value::RtType;
    use std::rc::Rc;

    /// A table that has one row, 7, and is served by a provider.
    struct One;
    impl VirtualTables for One {
        fn serves(&self, t: &TableDef) -> bool {
            t.name == "V"
        }
        fn get(&self, _: &MemStore, _: &AccessContext, _: &TableDef, pk: &[Value]) -> Result<Option<Row>, StoreError> {
            Ok(matches!(pk, [Value::Integer(7)]).then(|| vec![Value::Integer(7)]))
        }
        fn next(&self, _: &MemStore, _: &AccessContext, _: &TableDef, scan: &Scan) -> Result<Option<Row>, StoreError> {
            Ok(scan.after.is_none().then(|| vec![Value::Integer(7)]))
        }
        fn count(&self, _: &MemStore, _: &AccessContext, _: &TableDef, _: &[FieldFilter]) -> Result<usize, StoreError> {
            Ok(1)
        }
    }

    fn table(name: &str) -> TableDef {
        TableDef {
            object: 0,
            name: name.into(),
            fields: vec![FieldDef {
                id: 1,
                name: "K".into(),
                ty: RtType::Integer,
                stored: true,
                code_storage: Default::default(),
            }],
            keys: vec![vec![0]],
            per_company: false,
        }
    }

    #[test]
    fn a_served_table_is_read_from_the_provider_and_never_written() {
        let cx = AccessContext {
            company: "C".into(),
            user: "U".into(),
        };
        let mut s = MemStore::with_virtual_tables(Rc::new(One));
        let v = table("V");
        assert_eq!(s.count(&cx, &v, &[]), Ok(1));
        assert!(s.get(&cx, &v, &[Value::Integer(7)]).unwrap().is_some());
        assert!(s.get(&cx, &v, &[Value::Integer(8)]).unwrap().is_none());
        for r in [
            s.insert(&cx, &v, vec![Value::Integer(8)]).map(|_| ()),
            s.modify(&cx, &v, vec![Value::Integer(7)]).map(|_| ()),
            s.delete(&cx, &v, &[Value::Integer(7)]).map(|_| ()),
            s.delete_all(&cx, &v, &[]).map(|_| ()),
        ] {
            assert!(matches!(r, Err(StoreError::Refused(_))), "{r:?}");
        }
        assert!(!s.has_uncommitted(), "a refused write leaves nothing pending");
        // A table the provider does not serve is an ordinary stored table.
        let w = table("W");
        s.insert(&cx, &w, vec![Value::Integer(1)]).unwrap();
        assert_eq!(s.count(&cx, &w, &[]), Ok(1));
    }
}
