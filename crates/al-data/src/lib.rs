//! Storage for AL tables: the AL-shaped `Store` trait (ADR-003), the
//! `SnapshotStore` extension (ADR-004), an in-memory implementation, and the
//! contract suite every store must pass.

pub mod contract;
pub mod memory;
pub(crate) mod pmap;

use al_builtins::filter::FieldFilter;
use al_builtins::value::{RtType, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct FieldDef {
    pub id: u32,
    pub name: String,
    pub ty: RtType,
    /// FlowFields and FlowFilters are not stored.
    pub stored: bool,
    /// How the database stores a Code field (its `SQLDataType`); `Text` for every other field.
    pub code_storage: CodeStorage,
}

/// The column type of a Code field (`SQLDataType`). An `Integer` one sorts by number: the empty value first, then 9
/// before 10 (corpus record/lt-x-table-props/XTablePropsTests.al:322-325). The order of a `BigInteger` one is not
/// pinned: a read that sorts by it is refused, as is a filter on either other than one value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodeStorage {
    #[default]
    Text,
    Integer,
    BigInteger,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableDef {
    /// Object index of the table.
    pub object: u32,
    pub name: String,
    pub fields: Vec<FieldDef>,
    /// Keys as field positions; `keys[0]` is the primary key.
    pub keys: Vec<Vec<usize>>,
    /// `DataPerCompany = false` tables are shared by all companies (ADR-005).
    pub per_company: bool,
}

impl TableDef {
    /// The key order of two values of field `i`.
    #[inline]
    pub fn cmp_key_field(&self, i: usize, a: &Value, b: &Value) -> std::cmp::Ordering {
        if let (Value::Code(x), Value::Code(y)) = (a, b)
            && self.fields[i].code_storage == CodeStorage::Integer
        {
            // The values are written as plain numbers (the VM refuses any other), so length then text is the number order.
            let (x, y) = (x.as_str(), y.as_str());
            return x.len().cmp(&y.len()).then_with(|| x.cmp(y));
        }
        al_builtins::ops::compare_key_value(a, b).unwrap_or(std::cmp::Ordering::Equal)
    }

    pub fn primary_key(&self, row: &[Value]) -> Vec<Value> {
        self.keys[0].iter().map(|&i| row[i].clone()).collect()
    }
}

/// Who is asking, for which company (ADR-005). Permissions come later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessContext {
    pub company: std::sync::Arc<str>,
    pub user: std::sync::Arc<str>,
}

pub type Row = Vec<Value>;

#[derive(Clone, PartialEq, Eq)]
pub enum StoreError {
    Conflict,
    Deadlock,
    ChangedByAnotherUser,
    /// The store will not do this and says why (a write to a table whose rows are computed on
    /// read, a read it cannot answer without guessing). Callers report it as unsupported.
    Refused(String),
}

/// A refusal prints as its reason, so a message that formats the error reads the reason itself and
/// not `Refused("...")`.
impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Conflict => f.write_str("Conflict"),
            StoreError::Deadlock => f.write_str("Deadlock"),
            StoreError::ChangedByAnotherUser => f.write_str("ChangedByAnotherUser"),
            StoreError::Refused(why) => f.write_str(why),
        }
    }
}

/// Test failures and `unwrap` print the same text.
impl std::fmt::Debug for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertOutcome {
    Inserted,
    AlreadyExists,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    Done,
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameOutcome {
    Renamed,
    NotFound,
    AlreadyExists,
}

/// A key-ordered scan: rows passing `filters`, in the order of key `key`
/// (then primary key), ascending or not, strictly after `after` (a position
/// as returned by [`Store::position`]) or from the start.
#[derive(Debug, Clone)]
pub struct Scan<'a> {
    pub key: usize,
    pub filters: &'a [FieldFilter],
    pub ascending: bool,
    pub after: Option<&'a [Value]>,
}

/// Tables whose rows are computed on read instead of stored (ADR-019): the platform's virtual and
/// metadata tables. A [`memory::MemStore`] given one asks it first for every table it `serves`;
/// those tables have no rows of their own, cannot be written, and are not part of a snapshot,
/// commit or rollback.
pub trait VirtualTables {
    /// Whether table `t` is answered by this provider.
    fn serves(&self, t: &TableDef) -> bool;
    /// Why a write to table `t` is refused, when the provider has a reason of its own.
    fn write_refusal(&self, _t: &TableDef) -> Option<String> {
        None
    }
    /// Whether an Insert into table `t` adds a row the store keeps next to the provider's own: the provider answers the
    /// stored rows too ([`memory::MemStore::stored_rows`]), and a Modify or Delete of such a row writes the store. The
    /// provider's own rows stay read-only.
    fn takes_inserts(&self, _t: &TableDef) -> bool {
        false
    }
    /// The ids of the columns of table `t` a Modify may change; the store keeps the changed row and the provider
    /// answers it from then on (see [`memory::MemStore::stored_rows`]). Any other change is refused. A Modify
    /// that writes the row back unchanged is accepted when this is not empty.
    fn writable_columns(&self, _t: &TableDef) -> &'static [u32] {
        &[]
    }
    /// The reads take the store they are called through, so a provider can compose its rows from
    /// stored tables too (the Aggregate Permission Set lists the tenant's permission sets).
    fn get(
        &self,
        store: &memory::MemStore,
        cx: &AccessContext,
        t: &TableDef,
        pk: &[Value],
    ) -> Result<Option<Row>, StoreError>;
    /// The next row of a scan, as [`Store::next`].
    fn next(
        &self,
        store: &memory::MemStore,
        cx: &AccessContext,
        t: &TableDef,
        scan: &Scan,
    ) -> Result<Option<Row>, StoreError>;
    fn count(
        &self,
        store: &memory::MemStore,
        cx: &AccessContext,
        t: &TableDef,
        filters: &[FieldFilter],
    ) -> Result<usize, StoreError>;
}

pub trait Store {
    fn insert(&mut self, cx: &AccessContext, t: &TableDef, row: Row) -> Result<InsertOutcome, StoreError>;
    fn get(&self, cx: &AccessContext, t: &TableDef, pk: &[Value]) -> Result<Option<Row>, StoreError>;
    fn modify(&mut self, cx: &AccessContext, t: &TableDef, row: Row) -> Result<WriteOutcome, StoreError>;
    fn delete(&mut self, cx: &AccessContext, t: &TableDef, pk: &[Value]) -> Result<WriteOutcome, StoreError>;
    fn rename(
        &mut self,
        cx: &AccessContext,
        t: &TableDef,
        old_pk: &[Value],
        row: Row,
    ) -> Result<RenameOutcome, StoreError>;
    /// The next row of a scan, if any.
    fn next(&self, cx: &AccessContext, t: &TableDef, scan: &Scan) -> Result<Option<Row>, StoreError>;
    fn count(&self, cx: &AccessContext, t: &TableDef, filters: &[FieldFilter]) -> Result<usize, StoreError>;
    fn delete_all(&mut self, cx: &AccessContext, t: &TableDef, filters: &[FieldFilter]) -> Result<usize, StoreError>;
    /// Makes every change so far durable.
    fn commit(&mut self);
    /// Discards every change since the last commit.
    fn rollback(&mut self);
}

/// The position of `row` in key `key`: key fields then primary key fields.
pub fn position(t: &TableDef, key: usize, row: &[Value]) -> Vec<Value> {
    t.keys[key].iter().chain(&t.keys[0]).map(|&i| row[i].clone()).collect()
}

/// Stores whose whole state can be captured and restored in O(1) (ADR-004).
pub trait SnapshotStore: Store {
    type Snapshot: Clone;
    fn snapshot(&self) -> Self::Snapshot;
    fn restore(&mut self, s: Self::Snapshot);
}
