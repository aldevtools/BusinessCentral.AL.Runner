//! Runtime values (ADR-010).
//!
//! The state of a page is not here (ADR-072: al-vm `pages/state/mod.rs`). Entry points: `Value`,
//! `RtType`, `RecordState`, `Instance`, `DotNetObject`.

pub use crate::page_handle::{ControlHandle, PageHandle};
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::Rc;

/// The members of the system option `Action`, in docs order (none of the corpus reads an
/// ordinal: docs/probes-needed.md).
pub const ACTION_MEMBERS: [&str; 9] = [
    "None",
    "OK",
    "Cancel",
    "LookupOK",
    "LookupCancel",
    "Yes",
    "No",
    "RunObject",
    "RunSystem",
];

/// The members of the system option `PromptMode` of a PromptDialog page, in docs order.
pub const PROMPT_MODE_MEMBERS: [&str; 3] = ["Prompt", "Content", "Generate"];

/// Runtime type of a slot: what a variable, field or parameter holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RtType {
    Boolean,
    Integer,
    BigInteger,
    Decimal,
    Byte,
    Char,
    Text(Option<u32>),
    Code(Option<u32>),
    Date,
    Time,
    DateTime,
    Duration,
    Guid,
    DateFormula,
    Option(std::sync::Arc<[String]>),
    Enum(u32),
    Variant,
    List(Box<RtType>),
    Dictionary(Box<RtType>, Box<RtType>),
    Array(Vec<u32>, Box<RtType>),
    /// Record of the table with this object index; temporary flag.
    Record(u32, bool),
    /// Codeunit with this object index; also an xmlport variable, which is an instance with
    /// its own globals in the same way.
    Codeunit(u32),
    /// `TestPage` of the page with this object index (ADR-012).
    TestPage(u32),
    /// A `Page` variable of the page with this object index: a page instance that exists before
    /// it is run (ADR-012).
    Page(u32),
    /// A `Query` variable of the query with this object index.
    Query(u32),
    /// An `Interface` variable of the interface with this object index. It holds the
    /// codeunit instance it was assigned (or `Null` while unassigned).
    Interface(u32),
    /// Library types whose values are opaque handles (`JsonObject`, `TextBuilder`, ...).
    Library(crate::library_kind::LibraryKind),
    /// A `DotNet` variable declared through the alias at this index of `Program::dotnet_types`
    /// (ADR-014). Unassigned, it holds `Value::Null`.
    DotNet(u32),
}

/// The items of a List: a `Vec<Value>` that counts the times it was borrowed mutably, so that a `foreach` over it notices
/// a change (collections/RefSemantics.al:Foreach_ListAddWhileIterating, ...).
#[derive(Debug, Clone, Default)]
pub struct ListVec {
    items: Vec<Value>,
    pub version: u64,
    /// The list `Enum.Ordinals()` and `Enum.Names()` answer: every change raises NotSupportedOperation
    /// (collections/CsCollections.al:EnumOrdinals_Mutations, EnumNames_Mutations).
    pub read_only: bool,
}

impl From<Vec<Value>> for ListVec {
    fn from(items: Vec<Value>) -> Self {
        ListVec {
            items,
            version: 0,
            read_only: false,
        }
    }
}

impl FromIterator<Value> for ListVec {
    fn from_iter<I: IntoIterator<Item = Value>>(iter: I) -> Self {
        ListVec::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl std::ops::Deref for ListVec {
    type Target = Vec<Value>;
    fn deref(&self) -> &Vec<Value> {
        &self.items
    }
}

impl std::ops::DerefMut for ListVec {
    fn deref_mut(&mut self) -> &mut Vec<Value> {
        self.version += 1;
        &mut self.items
    }
}

/// The declaration a TextConstant in a Variant came from (`Value::TextConst`): the readers outside the pinned ones treat
/// the two apart (`conv::text_constant::read`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextConstFrom {
    /// A variable declared `Label`.
    Label,
    /// A variable declared `TextConst`.
    TextConst,
}

#[derive(Debug, Clone)]
pub enum Value {
    /// An unassigned Variant.
    Null,
    Boolean(bool),
    Integer(i32),
    BigInteger(i64),
    Decimal(Decimal),
    Byte(u8),
    Char(u16),
    Text(crate::text::Txt),
    /// A Label or TextConst held by a Variant: the text and the kind of its declaration. It answers IsTextConstant and no
    /// IsText (types/Y81LabelVariant.al; types/variant/VariantIsMatrix.al:224-243). Only a Variant ever holds it; what
    /// every other reader gets is `conv::text_constant::read`.
    TextConst(crate::text::Txt, TextConstFrom),
    Code(crate::text::Txt),
    /// Day number (0 = undefined `0D`), closing-date flag.
    Date(i32, bool),
    /// Milliseconds since midnight plus one (0 = undefined `0T`).
    Time(i64),
    /// Milliseconds since 0001-01-01 UTC plus one (0 = undefined `0DT`).
    DateTime(i64),
    /// Milliseconds.
    Duration(i64),
    Guid([u8; 16]),
    DateFormula(String),
    /// Ordinal and the option's member names (needed by Format).
    Option(i32, std::sync::Arc<[String]>),
    Enum(u32, i32),
    List(Rc<RefCell<ListVec>>),
    Dictionary(Rc<RefCell<Vec<(Value, Value)>>>),
    Array(Rc<RefCell<Vec<Value>>>),
    Codeunit(Rc<RefCell<Instance>>),
    Library(crate::library_kind::LibraryKind, Rc<RefCell<LibraryValue>>),
    /// A record variable (ADR-010: a handle; `:=` copies fields, see the VM).
    Record(Rc<RefCell<RecordState>>),
    /// A Blob field's content: a shared buffer that streams write to (see `stream`).
    Blob(crate::stream::BlobRef),
    /// A `TestPage` (or `TestPart`): a handle on a page and the test client
    /// driving it (ADR-012). Shared like a codeunit instance.
    TestPage(PageHandle),
    /// A control of a test page (`TestField`, `TestAction`, `TestFilter`).
    TestControl(ControlHandle),
    /// A query variable: its runtime filters and, once opened, its result set.
    Query(Rc<RefCell<QueryState>>),
    /// A `DotNet` object (ADR-014). `:=` shares the reference (assumed BC rule, probe pending).
    DotNet(Rc<DotNetObject>),
    /// A column of a platform table row whose value nothing pins (ADR-013 decision 7, hard rule 3): the
    /// provider leaves it unknown instead of giving it a default. It only ever sits in a record buffer or a
    /// listed row; every read of it is refused with the reason it holds, and writing the field replaces it.
    /// An event argument the platform leaves unknown (RequestPageXml of SubstituteReport for an Html SaveAs) is
    /// the one exception: it sits in a by-value parameter, can be passed on by value, and any other use is refused.
    Unknown(std::sync::Arc<str>),
    /// A value read from `SessionInformation.SqlStatementsExecuted()` or `SqlRowsRead()`, or computed from such reads
    /// (issue 5580): a BigInteger whose number is the runtime's model, so only the uses `sqlcount` lists are answered.
    SqlCount(SqlCount),
    /// The byte size of a placeholder report output (`Blob.Length`, `File.Len`), with the placeholder's mark and the number type of
    /// the slot that holds it: a number known only to be greater than 0, so only a comparison with 0 is answered
    /// (`crate::placeholder_output::size`).
    PlaceholderSize(std::sync::Arc<str>, SqlType),
}

/// A counter value (`Value::SqlCount`, see `crate::sqlcount`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqlCount {
    /// The model's number.
    pub n: i64,
    pub form: SqlForm,
    /// The type of the slot that holds it: a read is a BigInteger, and an Integer or a Decimal keeps it tainted (al-runner
    /// issue 6924, session-system/session/SqlCounters.al P6924 tests).
    pub ty: SqlType,
}

/// The number type a counter value (`SqlCount::ty`) or a placeholder's size (`Value::PlaceholderSize`) has: the type of the
/// slot that holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum SqlType {
    Integer,
    #[default]
    BigInteger,
    Decimal,
}

/// What a counter value is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlForm {
    /// A read of one counter. Two reads of the same epoch have only classified segments between them; two reads of the
    /// same epoch and chain have only segments between them whose size the corpus pins (al-vm `sqlcount.rs`).
    Read {
        counter: SqlCounter,
        epoch: u32,
        chain: u32,
        /// The chain this read's chain started from, after a change whose lower bound alone is pinned (0: none). A read
        /// minus a read of that chain is at least the model's difference ([`SqlForm::AtLeast`]).
        floor: u32,
    },
    /// The difference of two reads of one epoch, or a sum of such differences whose sign is certain.
    Delta,
    /// A difference known to be at least the number, its exact size not pinned.
    AtLeast,
    /// The difference of two reads of one counter of different epochs: work whose change no test pins ran between them, so
    /// neither its size nor its sign is known. It may be stored, passed on, formatted and logged; every use that reads it
    /// is refused (`al_builtins::sqlcount`).
    Unknown,
}

/// Which counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlCounter {
    Statements,
    RowsRead,
}

/// What a `Value::DotNet` refers to.
pub enum DotNetKind {
    /// An object held by the host (the sidecar), by handle, with its CLR type name.
    Host { handle: u64, clr_type: String },
    /// A .NET enum value, kept by value with its CLR type so it can go back as an argument.
    Enum {
        clr_type: String,
        name: String,
        ordinal: i64,
    },
    /// An object the runtime provides itself (the registry, ADR-014 section 1).
    Internal {
        type_name: String,
        state: RefCell<Vec<Value>>,
    },
}

/// A `DotNet` object. When the last `Rc` drops, a host handle goes onto the session's release
/// queue; `Drop` never calls the host (ADR-014 section 2).
pub struct DotNetObject {
    pub kind: DotNetKind,
    release: Option<Rc<RefCell<Vec<u64>>>>,
}

impl DotNetObject {
    /// A host object; its handle is queued on `release` when the object drops.
    pub fn host(handle: u64, clr_type: String, release: Rc<RefCell<Vec<u64>>>) -> DotNetObject {
        DotNetObject {
            kind: DotNetKind::Host { handle, clr_type },
            release: Some(release),
        }
    }

    pub fn new(kind: DotNetKind) -> DotNetObject {
        DotNetObject { kind, release: None }
    }
}

impl Drop for DotNetObject {
    fn drop(&mut self) {
        if let (DotNetKind::Host { handle, .. }, Some(queue)) = (&self.kind, &self.release) {
            queue.borrow_mut().push(*handle);
        }
    }
}

impl std::fmt::Debug for DotNetObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            DotNetKind::Host { handle, clr_type } => write!(f, "DotNet({clr_type} #{handle})"),
            DotNetKind::Enum { clr_type, name, .. } => write!(f, "DotNet({clr_type}.{name})"),
            DotNetKind::Internal { type_name, .. } => write!(f, "DotNet(internal {type_name})"),
        }
    }
}

/// What a query variable carries (corpus query/). The definition is in the IR; the VM
/// (`al-vm/src/queries.rs`) executes it.
#[derive(Debug, Clone)]
pub struct QueryState {
    /// Object index of the query.
    pub object: u32,
    /// `SetRange` / `SetFilter` per column (position in `ir::Query::columns`); a later call on the
    /// same column replaces the earlier one.
    pub filters: Vec<(usize, crate::filter::FilterExpr)>,
    /// A `SetRange` or `SetFilter` replaced or removed a filter set before, so `filters` no longer
    /// holds the order in which each filter was first set.
    pub filter_replaced: bool,
    /// `TopNumberOfRows(n)`; `None` leaves the property in force.
    pub top: Option<i32>,
    /// `SecurityFiltering(x)` as a member index of the platform option; `None` before any call.
    pub security_filtering: Option<i32>,
    /// The result set between `Open` and `Close`.
    pub run: Option<QueryRun>,
    /// The instance of a query with code: its variables live as long as the query variable
    /// (queries/TestQueryOnBeforeOpenVariablesContracts.al:104-140). Made on first use.
    pub instance: Option<Rc<RefCell<Instance>>>,
    /// The query's OnBeforeOpen trigger is running: `CurrQuery.Open` and `Close` raise
    /// (streams/ChQueryLeads.al:259-290).
    pub in_before_open: bool,
    /// `Close` ran since the last `Open`: a filter element read then is not pinned (only before `Open` is,
    /// assumption query-filter-element-read-without-row).
    pub closed: bool,
    /// Where the variable stands in the pinned sequence of a second `Open` while rows are left (`al-vm` `query_reopen`).
    pub reopen: crate::query_reopen_step::ReopenStep,
}

impl QueryState {
    /// A new or cleared query variable of query object `object`: no filters, no row cap, not open.
    pub fn new(object: u32) -> QueryState {
        QueryState {
            object,
            filters: Vec::new(),
            filter_replaced: false,
            top: None,
            security_filtering: None,
            run: None,
            instance: None,
            in_before_open: false,
            closed: false,
            reopen: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryRun {
    /// Whether the query has run. `Open` does not run it: the first `Read` does, so a row written
    /// in between is read (queries/TestQueryOuterJoinDescendingSaveAsContracts.al).
    pub started: bool,
    /// A SetRange or SetFilter came after `Open` and before the first `Read`: `Read` raises until the query is opened again
    /// (queries/read-lifecycle/PnAQueryOpenRead.al:SetRangeBetweenOpenAndFirstRead, SetRangeBeforeOpen_ThenChangedBeforeFirstRead).
    pub filters_changed: bool,
    /// The rows still to read, in the query's order.
    pub rows: Vec<QueryRow>,
    pub next: usize,
    /// The column values of the row `Read` returned last (`None` before the first and after the last).
    pub current: Option<Vec<Value>>,
    /// Sort key of the last row returned: after a write the result is read again from there.
    pub last_key: Option<Vec<Value>>,
    /// Rows returned since `Open`, against `TopNumberOfRows`.
    pub returned: usize,
    /// Version of each table the query reads (table object, stamp) when `rows` were read.
    pub stamps: Vec<(u32, u64)>,
}

impl QueryRun {
    /// What `Open` leaves: opened, not run yet.
    pub fn opened() -> QueryRun {
        QueryRun {
            started: false,
            filters_changed: false,
            rows: Vec::new(),
            next: 0,
            current: None,
            last_key: None,
            returned: 0,
            stamps: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueryRow {
    /// One value per query column (`Null` for filter elements).
    pub columns: Vec<Value>,
    /// The sort key: the `OrderBy` columns, then the primary key of each dataitem.
    pub key: Vec<Value>,
}

/// The identity of the private table of a temporary record. Records that share the table (a record
/// state clone, `Copy(x, true)`) hold clones of one `TempId`; when the last is dropped the id goes onto
/// the session's release queue and the session frees the table, as `DotNetObject` hands its handle back
/// (`Drop` never reaches into the session).
#[derive(Clone)]
pub struct TempId(Rc<TempToken>);

struct TempToken {
    id: u64,
    release: Rc<RefCell<Vec<u64>>>,
    /// The table was made for a record handed by value (al-vm `record_by_value`): the record it was copied from. The copy's Blob
    /// fields may still share their buffers with that record and its rows.
    by_value_of: Option<std::rc::Weak<RefCell<RecordState>>>,
    /// The primary key of the row the table's last Insert stored while a Blob of the record was empty: a row that may share a
    /// record's Blob buffer once the record moves on (al-vm `temp_blob_share.rs`). Where to look first, never more: the row is
    /// looked up, and the other rows are still searched while holders stay unaccounted for.
    empty_blob_insert_key: RefCell<Option<Vec<Value>>>,
    /// The record state whose Insert noted [`TempToken::empty_blob_insert_key`]; dropped when `RecordRef.SetTable` writes into a
    /// record of the table (al-vm `temp_blob_share.rs`, `measured_detach`).
    empty_blob_inserter: RefCell<Option<std::rc::Weak<RefCell<RecordState>>>>,
    /// The table is a Variant's by-value copy whose rows the corpus does not measure (al-vm `variant_copy.rs`, al-runner issue
    /// 8070): every read of its rows and every write refuses (al-vm `Session::store_for`). Set once, never cleared: it ends with
    /// the table.
    rows_unmeasured: std::cell::Cell<bool>,
    /// The records that got the table as the target of `Record.Copy(x, true)`, each with the record it copied (al-vm
    /// `clear_shared_codeunit.rs` counts them apart from the table's other sharers).
    copy_targets: RefCell<Vec<CopyShare>>,
}

/// A live `target.Copy(source, true)` of a temporary table: the target, and the source while it lives.
pub type CopyPair = (Rc<RefCell<RecordState>>, Option<Rc<RefCell<RecordState>>>);

/// What [`TempId::image_parts`] reads of a temporary table's token.
pub struct TempTokenParts {
    pub id: u64,
    /// The table was made for a record handed by value ([`TempId::is_by_value_copy`]).
    pub by_value_copy: bool,
    /// [`TempId::with_empty_blob_insert_key`].
    pub empty_blob_insert_key: Option<Vec<Value>>,
    /// [`TempId::empty_blob_inserter`].
    pub empty_blob_inserter: Option<Rc<RefCell<RecordState>>>,
    /// [`TempId::rows_unmeasured`].
    pub rows_unmeasured: bool,
    /// The live copy targets that still hold the table ([`TempId::copy_targets`]), as records.
    pub copy_targets: Vec<CopyPair>,
}

/// One `target.Copy(source, true)` of a temporary table ([`TempId::note_copy_target`]).
struct CopyShare {
    target: std::rc::Weak<RefCell<RecordState>>,
    source: std::rc::Weak<RefCell<RecordState>>,
}

impl Drop for TempToken {
    fn drop(&mut self) {
        self.release.borrow_mut().push(self.id);
    }
}

impl TempId {
    pub fn new(id: u64, release: Rc<RefCell<Vec<u64>>>) -> TempId {
        TempId(Rc::new(TempToken {
            id,
            release,
            by_value_of: None,
            empty_blob_insert_key: RefCell::new(None),
            empty_blob_inserter: RefCell::new(None),
            rows_unmeasured: Default::default(),
            copy_targets: Default::default(),
        }))
    }

    /// The table of a temporary record's copy handed by value; `source` is the record it was copied from.
    pub fn new_by_value_copy(id: u64, release: Rc<RefCell<Vec<u64>>>, source: &Rc<RefCell<RecordState>>) -> TempId {
        TempId(Rc::new(TempToken {
            id,
            release,
            by_value_of: Some(Rc::downgrade(source)),
            empty_blob_insert_key: RefCell::new(None),
            empty_blob_inserter: RefCell::new(None),
            rows_unmeasured: Default::default(),
            copy_targets: Default::default(),
        }))
    }

    pub fn id(&self) -> u64 {
        self.0.id
    }

    /// How many record states share the table (`Copy(x, true)`, a record state clone), this one included.
    pub fn sharers(&self) -> usize {
        Rc::strong_count(&self.0)
    }

    /// Marks the rows of the table as not measured (al-vm `variant_copy.rs`), for every record that shares it.
    pub fn mark_rows_unmeasured(&self) {
        self.0.rows_unmeasured.set(true);
    }

    /// The record state whose Insert [`TempId::note_empty_blob_insert`] last noted, while it lives (al-vm `login_image`
    /// keeps the note).
    pub fn empty_blob_inserter(&self) -> Option<Rc<RefCell<RecordState>>> {
        self.0
            .empty_blob_inserter
            .borrow()
            .as_ref()
            .and_then(std::rc::Weak::upgrade)
    }

    /// Whether [`TempId::mark_rows_unmeasured`] marked the table.
    pub fn rows_unmeasured(&self) -> bool {
        self.0.rows_unmeasured.get()
    }

    /// Whether the table was made for a record handed by value ([`TempId::new_by_value_copy`]).
    pub fn is_by_value_copy(&self) -> bool {
        self.0.by_value_of.is_some()
    }

    /// The record a by-value copy was made from, while it lives ([`TempId::new_by_value_copy`]).
    pub fn by_value_source(&self) -> Option<Rc<RefCell<RecordState>>> {
        self.0.by_value_of.as_ref().and_then(std::rc::Weak::upgrade)
    }

    /// The Insert of `inserter` into the table stored a row while a Blob of the record was empty, under the primary key `key`
    /// yields.
    pub fn note_empty_blob_insert<'a>(
        &self,
        inserter: &Rc<RefCell<RecordState>>,
        key: impl Iterator<Item = &'a Value>,
    ) {
        *self.0.empty_blob_inserter.borrow_mut() = Some(Rc::downgrade(inserter));
        let mut slot = self.0.empty_blob_insert_key.borrow_mut();
        let kept = slot.get_or_insert_with(Vec::new);
        kept.clear();
        kept.extend(key.cloned());
    }

    /// Whether `rec` is the record state whose Insert [`TempId::note_empty_blob_insert`] last noted.
    pub fn empty_blob_inserted_by(&self, rec: &Rc<RefCell<RecordState>>) -> bool {
        let inserter = self.0.empty_blob_inserter.borrow();
        inserter
            .as_ref()
            .and_then(std::rc::Weak::upgrade)
            .is_some_and(|r| Rc::ptr_eq(&r, rec))
    }

    /// `RecordRef.SetTable` wrote into a record of the table: no record state counts as the inserter any more.
    pub fn forget_empty_blob_inserter(&self) {
        *self.0.empty_blob_inserter.borrow_mut() = None;
    }

    /// `target.Copy(source, true)` gave `target` the table.
    pub fn note_copy_target(&self, target: &Rc<RefCell<RecordState>>, source: &Rc<RefCell<RecordState>>) {
        self.note_copy_target_of(target, Some(source));
    }

    /// [`TempId::note_copy_target`], also for a share whose source is gone (`None`): a session image (al-vm
    /// `login_image`) notes again on its thaw what [`TempId::image_parts`] read.
    pub fn note_copy_target_of(&self, target: &Rc<RefCell<RecordState>>, source: Option<&Rc<RefCell<RecordState>>>) {
        let mut targets = self.0.copy_targets.borrow_mut();
        targets.retain(|c| c.target.strong_count() > 0 && !std::ptr::eq(c.target.as_ptr(), Rc::as_ptr(target)));
        targets.push(CopyShare {
            target: Rc::downgrade(target),
            source: source.map_or_else(std::rc::Weak::new, Rc::downgrade),
        });
    }

    /// The live records [`TempId::note_copy_target`] noted that still hold this table, by address, each with the address
    /// of the record it copied while that record lives.
    pub fn copy_targets(&self) -> Vec<(usize, Option<usize>)> {
        (self.live_copy_targets().iter())
            .map(|(t, s)| (Rc::as_ptr(t) as usize, s.as_ref().map(|s| Rc::as_ptr(s) as usize)))
            .collect()
    }

    /// The live records [`TempId::note_copy_target`] noted that still hold this table, each with the record it copied while
    /// that record lives.
    fn live_copy_targets(&self) -> Vec<CopyPair> {
        let id = self.0.id;
        let holds = |r: &Rc<RefCell<RecordState>>| {
            r.try_borrow()
                .is_ok_and(|r| r.temp_id.as_ref().is_some_and(|t| t.id() == id))
        };
        (self.0.copy_targets.borrow().iter())
            .filter_map(|c| c.target.upgrade().filter(holds).map(|t| (t, c.source.upgrade())))
            .collect()
    }

    /// Everything the token holds, read through one destructure with no `..`, for a session image (al-vm `login_image`):
    /// a field added to the token fails that image's compile here.
    pub fn image_parts(&self) -> TempTokenParts {
        let TempToken {
            id,
            // The session's release queue: the thaw makes the table in the thawn session, with that session's queue.
            release: _,
            by_value_of,
            empty_blob_insert_key,
            empty_blob_inserter,
            rows_unmeasured,
            copy_targets: _,
        } = &*self.0;
        TempTokenParts {
            id: *id,
            by_value_copy: by_value_of.is_some(),
            empty_blob_insert_key: empty_blob_insert_key.borrow().clone(),
            empty_blob_inserter: empty_blob_inserter.borrow().as_ref().and_then(std::rc::Weak::upgrade),
            rows_unmeasured: rows_unmeasured.get(),
            copy_targets: self.live_copy_targets(),
        }
    }

    /// `f` of the key [`TempId::note_empty_blob_insert`] last noted.
    pub fn with_empty_blob_insert_key<R>(&self, f: impl FnOnce(Option<&[Value]>) -> R) -> R {
        f(self.0.empty_blob_insert_key.borrow().as_deref())
    }
}

impl std::fmt::Debug for TempId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TempId({})", self.0.id)
    }
}

/// Everything a record variable carries.
#[derive(Debug, Clone)]
pub struct RecordState {
    /// Object index of the table.
    pub table: u32,
    pub temporary: bool,
    /// Identity of the private table behind a temporary record (shared by `Copy(x, true)`).
    pub temp_id: Option<TempId>,
    /// The company set by `ChangeCompany` or `RecordRef.Open(.., Company)`, as the Company table
    /// names it; `None` is the session company.
    pub company: Option<String>,
    /// A `ChangeCompany` call succeeded on this variable, also to the session's own company or '' (which leave `company`
    /// at `None`), or on a record `Copy` took this one from. No method clears it. It only marks state the corpus does not
    /// measure: al-vm `modify_all_audit_field` refuses on it.
    pub change_company_called: bool,
    /// The table is `TableType = CRM`: any use needs a registered table connection
    /// (session-system/TestCrmTableNoConnectionContracts.al).
    pub crm: bool,
    /// Field values in the table's field order.
    pub fields: crate::record_fields::Fields,
    pub filters: Vec<crate::filter::FieldFilter>,
    /// Copies of the filters in `filters` that a measured setter set on a Decimal field whose SQL column cuts their values
    /// (al-vm `decimal_column::measured_setter`): a read compares such a filter cut only while the identical filter is here.
    /// `SetRange` and `SetFilter` drop the entry of their field and group; every copy of the record copies it with `filters`.
    pub decimal_cut_filters: Vec<crate::filter::FieldFilter>,
    /// The filter group `SetRange` and `SetFilter` write to and `GetFilter` reads.
    pub filter_group: i32,
    /// Primary keys marked with `Mark(true)`, and whether `MarkedOnly` restricts the set to them.
    pub marks: Vec<Vec<Value>>,
    pub marked_only: bool,
    /// `ReadIsolation` and `SecurityFiltering` as member indexes of their platform option types.
    pub read_isolation: i32,
    pub security_filtering: i32,
    /// The SecurityFiltering value the variable was declared with (`[SecurityFiltering(x)]`, else Validated): `Clear` sets
    /// `security_filtering` back to it. An assignment and `Copy` change neither (al-vm `Session::slot_default`). A copy that is a
    /// variable of its own takes the original's ([`RecordState::copy_as_new_variable`]).
    pub declared_security_filtering: i32,
    /// Whether `declared_security_filtering` came with a copy the corpus does not measure
    /// ([`RecordState::security_filtering_unmeasured_copy`]): `Clear` then gives a value that is not pinned either.
    pub declared_security_filtering_unpinned: bool,
    /// Whether the value `SecurityFiltering()` reads now is not pinned: the read is then refused. The corpus pins Clear,
    /// ClearAll, Reset, Init, assignment, Copy and parameters (al-language/reports/runtime/ACAttributes.al:355-414), a
    /// TransferFields or a RecordRef.SetTable into the record, a by-value and a temporary Record parameter, and a RecordRef
    /// taken from a record (al-language/reports/runtime/SFSecurityFilteringFlow.al); an event argument, a record handed to
    /// StartSession, a Variant's record and an array element passed by value, each of a record whose current value is not
    /// its declared one, are not measured.
    pub security_filtering_unpinned: bool,
    /// Whether the record is an element of a local array declared with `[SecurityFiltering(x)]`, whose direct reads,
    /// Clear of the element and Clear of the array are pinned (al-language/reports/runtime/SFSecurityFilteringFlow.al:429-459).
    /// A by-value copy of such an element is not measured ([`RecordState::copy_as_new_variable`]), nor is the array passed
    /// whole by value (al-vm `call_inner`).
    pub security_filtering_array: bool,
    /// Index of the current key in the table's keys.
    pub key: usize,
    /// `SetCurrentKey` on fields no key of the table starts with: the key index it stands for (one
    /// past the table's keys) and the field positions. `Session::table` appends it to the table's
    /// keys while `key` is that index, so a scan orders by these fields, then by the primary key.
    pub adhoc: Option<(usize, Vec<usize>)>,
    pub ascending: bool,
    /// `SetAscending(Field, false)`: bit i is set when the i-th field of the current key's position (the key's fields,
    /// then the primary key's) runs descending. Unlike `ascending` it does not survive a page's view round trip
    /// (testpage/TestPageViewRoundTripRoutes.al).
    pub field_descending: u32,
    /// The primary key of the row the record last read: `Find` still finds that row after the
    /// record moved it out of its own filter.
    pub position: Option<Rc<[Value]>>,
    /// Field positions of `SetAutoCalcFields`: calculated whenever a row is read.
    pub auto_calc: Vec<usize>,
    /// Partial records: `SetLoadFields` and which fields the buffer holds.
    pub load: LoadState,
    /// AL code has written a field of the buffer. BC builds the buffer of a record variable on
    /// the first write, and evaluates the `InitValue`s only then (record/TestDateTimeInitValue.Codeunit.al).
    pub touched: bool,
    /// AL code assigned a field since the last `Init` or `Clear`, even to its own default value; `AreFieldsLoaded` on a
    /// record nothing was fetched into reads it (record/load-fields).
    pub assigned: bool,
    /// This variable inserted its row since the last `Init` or `Clear`: a `CalcFields` of a Blob that finds the row gone
    /// (rolled back) empties the Blob (recordref-fieldref/PnE08RecordMisc.al:TestField_Blob).
    pub inserted: bool,
    /// What a RecordRef last read or wrote into the buffer: `RecordRef.IsDirty` compares with it, not with the stored row
    /// (recordref/lt-recordref-fieldref-members/RecordRefIsDirtyEdges.al:IsDirty_TwoRefs_OtherModifies). `None` when the
    /// buffer came from elsewhere (GetTable, Init).
    pub dirty_base: Option<Vec<Value>>,
    /// What `Record.IsDirty` compares the buffer with (record/ACRecordIsDirty.al). A clone of the state starts unknown.
    pub is_dirty_base: DirtyBase,
    /// The row this variable's last Modify wrote. A later Modify of the unchanged buffer, after a rollback undid that write,
    /// writes nothing (record/StaleBuffer.al:76-86).
    pub last_modify: Option<Vec<Value>>,
    /// The row version the buffer held when its Insert answered false (the key existed): a Modify through the buffer then raises
    /// DB:RecordChanged when the stored row's version is newer (record/crud/Y71FailedInsertModify.al).
    pub failed_insert: Option<i64>,
    /// Where the buffer's row version came from: the variable's own Insert or a Get (with that version and the number of
    /// commits of the session then). A stale buffer from the Insert of the transaction the last Commit closed may still be
    /// written (record/TestRecordStaleBufferWrite.al:395-690, `Session::stale_verdict`).
    pub row_origin: Option<RowOrigin>,
    /// Where the buffer's row version came from, for the Modify after a failed guarded Insert (see [`VersionEntry`]).
    pub version_entry: VersionEntry,
    /// The variable's own Insert and its own Get of that row after it (see [`OwnRead`]).
    pub own_read: OwnRead,
    /// The read of a row holding a Time stored past the day that the variable's buffer holds (see [`RolledRead`]).
    pub rolled_read: RolledRead,
    /// The record is a source the Blob copy probes measured (see [`ProbedSource`]).
    pub probed_source: ProbedSource,
    /// The table's and its extensions' global variables of this record variable (ADR-021).
    pub globals: TableGlobals,
    /// Every use of the record refuses with this reason: what its buffer holds is not pinned (the row a page closed on after
    /// `Page.RunModal(Id, Rec)` called as a statement, see `hand_back_selection`).
    pub unpinned_buffer: Option<&'static str>,
    /// A by-value copy whose filters were dropped on an assumption (`by-value-table-proc-other-table`,
    /// `by-value-report-procedure`, al-vm `records/by_value_view.rs`): a read of its view marks it. Clones of the state
    /// keep it; `Reset`, `SetView` and `Clear` end it.
    pub view_assumption: Option<crate::assumptions::Assumption>,
    /// A by-value copy that did not take its caller's SetAutoCalcFields (al-vm `records/by_value_view.rs`
    /// `measured_auto_calc`): a row read of it that calculates no field marks `by-value-record-no-auto-calc`
    /// (al-vm `flowfields.rs` `auto_calculate`). Clones of the state keep it.
    pub auto_calc_dropped: bool,
    /// The filters an action's `RunPageLink` put on this Rec of the page it opened: a copy of Rec passed by value does not
    /// carry them (testpage/TestActionRunPageLinkFilterGroup.al:292-369, `filters=` in OnOpenPage and OnAfterGetCurrRecord).
    pub run_page_link: Option<Rc<Vec<crate::filter::FieldFilter>>>,
}

/// The record is one whose Blobs the Blob copy probes measured as the source of a record-wide move (a record passed by
/// value, `Record.Copy`, `TransferFields` with flags, `SetTable(Rec, true)`, a Blob moved into a Variant that is not local):
/// al-vm `blob_transfer` sets it on a declared variable, keeps it through the record methods and builtins the probes ran, and
/// ends it at every other one (`blob_transfer::Holder`). The byte is the kind of measured record in al-vm `blob_transfer`'s
/// encoding, 0 for every other record; the number is the session's commit count when the byte was set (a Commit since ends
/// it).
///
/// `Clone` gives 0: a copy of a record state (a record passed by value, a Variant's record, xRec) is not that variable.
#[derive(Debug, Default)]
pub struct ProbedSource(pub u8, pub u64);

impl Clone for ProbedSource {
    fn clone(&self) -> Self {
        ProbedSource(0, 0)
    }
}

/// A record variable inserted a row with `Insert()` and then read it with `Get`, with nothing written to the record's store
/// in between: the buffer the assigned audit field cells measure (record/system-fields/AssignedAuditFields.al:140-148 `Seed`,
/// :650-655; the rules are in al-vm `assigned_audit.rs`). `None` for every other buffer: after any other read, an `Init`, a
/// `Modify` or a `Rename` through the variable.
///
/// `Clone` gives `None`: a copy of a record state (`:=`, a record passed by value, xRec) is not the variable that read the row.
#[derive(Debug, Default)]
pub struct OwnRead(pub Option<OwnReadAt>);

impl Clone for OwnRead {
    fn clone(&self) -> Self {
        OwnRead(None)
    }
}

/// The primary key of a database row holding a Time stored as 23:59:59.999 (al-vm `sqltime::is_rolled`) and which read put
/// it into the buffer: the variable's own `Get` from AL code, or ModifyAll's read of each row. A Modify keeps the stored value
/// of such a field only after that read (al-vm `records/time_rollover.rs`). Every other load, every record method but Modify,
/// `:=` into the variable and a builtin given the variable end it; a Modify uses it up.
///
/// `Clone` gives `None`: a copy of a record state (a record passed by value, xRec) is not the variable that read the row.
#[derive(Debug, Default)]
pub struct RolledRead(pub Option<(RolledReadBy, Vec<Value>)>);

impl Clone for RolledRead {
    fn clone(&self) -> Self {
        RolledRead(None)
    }
}

/// Which read made a [`RolledRead`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolledReadBy {
    /// `Get` that AL code called on a Record variable.
    Get,
    /// ModifyAll, for the row it modifies.
    ModifyAll,
}

/// See [`OwnRead`].
#[derive(Debug, Clone)]
pub struct OwnReadAt {
    /// The Get ran; before it only the Insert did.
    pub got: bool,
    /// A `Clear` of the variable ran between the Insert and the Get (AssignedAuditFields.al:652).
    pub cleared: bool,
    /// The row's SystemId.
    pub system_id: Value,
    /// The row's SystemRowVersion; `None` where the buffer holds none.
    pub version: Option<i64>,
    /// The session's commit count at the Insert.
    pub commits: u64,
    /// The change counter of the record's store after the Insert.
    pub store_changes: u64,
}

/// Where a buffer's row version came from, read only by the Modify after a failed guarded Insert (al-vm
/// `stale_own_modify.rs` `check_failed_insert_stale`, al-runner issue 8721). A clone of the record state marks it
/// [`VersionVia::Cloned`]; `:=`, Record.Copy, a by-value parameter and RecordRef.GetTable carry it as themselves
/// ([`VersionEntry::carried`]). Empty when no read or write of the buffer gave it its version (a TransferFields with
/// the key copies one: the buffer neither read nor wrote that row).
#[derive(Debug, Default)]
pub struct VersionEntry(pub Option<HeldVersion>);

impl Clone for VersionEntry {
    fn clone(&self) -> Self {
        self.carried(VersionVia::Cloned)
    }
}

impl VersionEntry {
    /// The entry of a version `source` just gave the variable itself.
    pub fn own(source: VersionSource) -> VersionEntry {
        VersionEntry(Some(HeldVersion {
            source: Rc::new(source),
            via: VersionVia::Own,
            failed_inserts: 0,
            deleted: false,
        }))
    }

    /// The entry a copy of the buffer takes, made by `via`: the same source, no failed Insert of its own yet.
    pub fn carried(&self, via: VersionVia) -> VersionEntry {
        VersionEntry(self.0.as_ref().map(|h| HeldVersion {
            source: Rc::clone(&h.source),
            via,
            failed_inserts: 0,
            deleted: h.deleted,
        }))
    }

    /// This copy came by `via`: what [`VersionEntry::carried`] gives a copy that is already a clone of the buffer.
    pub fn now_via(&mut self, via: VersionVia) {
        if let Some(h) = self.0.as_mut() {
            h.via = via;
        }
    }
}

/// One copy's hold of a [`VersionSource`]. The source is what the read or write stamped and every copy shares it; the
/// rest is the copy's own.
#[derive(Debug, Clone)]
pub struct HeldVersion {
    pub source: Rc<VersionSource>,
    /// Which copy holds it.
    pub via: VersionVia,
    /// The guarded Inserts of this copy at this version that answered false.
    pub failed_inserts: u32,
    /// The variable deleted its row since.
    pub deleted: bool,
}

/// One [`VersionEntry`]'s source: the version, how it reached the buffer, and the open transaction at that moment.
#[derive(Debug)]
pub struct VersionSource {
    /// The row version.
    pub version: i64,
    /// The primary key of the row it is the version of, at the read or write (the buffer's key may move since).
    pub key: Rc<[Value]>,
    /// The read or the write that gave it.
    pub how: VersionHow,
    /// What the open transaction had written to the table then (the buffer's own write included).
    pub table_written: TableWritten,
    /// The buffer's company then (`None`: the session's own).
    pub company: Option<String>,
    /// The session's test methods started, Commits and rollbacks then.
    pub tests: u32,
    pub commits: u64,
    pub rollbacks: u64,
    /// The session's noted writes then (al-vm `TxState::write_counts`): all of them, those no cell measures after the
    /// read, and those of this table and company.
    pub writes: [u64; 3],
    /// The length of the open transaction's write log then (al-vm `OpenTx::writes`).
    pub log: usize,
}

/// The read or write that gave a buffer its row version ([`VersionSource::how`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionHow {
    /// `Get`.
    Get,
    /// FindFirst, FindLast, FindSet, Find or Next.
    Find,
    /// Another read: GetBySystemId, a read through a RecordRef, any other load.
    OtherRead,
    /// The variable's own Insert, Modify or Rename (a plain Rename that rewrote no related row).
    OwnInsert,
    OwnModify,
    OwnRename,
    /// Another write of the variable: through a RecordRef, or a Rename that rewrote related rows.
    OtherWrite,
}

/// Which copy of the buffer holds a [`VersionSource`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionVia {
    /// The variable whose read or write gave the version.
    Own,
    /// A variable that took it by `:=`.
    Assigned,
    /// A variable that took it by Record.Copy.
    Copied,
    /// A by-value parameter of a procedure called by name.
    ByValue,
    /// A RecordRef that took it by GetTable.
    GetTable,
    /// Any other copy of the record state.
    Cloned,
}

/// What the open transaction had written or locked of a table, in one company, since the last Commit or rollback (al-vm
/// `OpenTx::table_writes`). `No` is also the answer for a temporary record, whose writes are not in the transaction,
/// and for a write the runtime did not note: both keep main's raise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum TableWritten {
    /// No noted write and no LockTable of the table.
    #[default]
    No,
    /// Only a DeleteAll or ModifyAll that matched no row (no cell measures it alone).
    EmptyBulk,
    /// Only writes of kinds a cell measures: a Record variable's Insert, Modify or Delete, a DeleteAll or ModifyAll
    /// that changed a row, and a Record variable's LockTable (corpus PR 1523,
    /// record/crud/TestRecordFailedInsertMovedKey.al).
    Written,
    /// Also another write: a Rename, a Truncate, a write through a RecordRef.
    Unmeasured,
}

/// See [`RecordState::row_origin`]. Two origins are equal when every field but `by_get` is: the stale-buffer rules compare
/// origins without it.
#[derive(Debug, Clone, Copy)]
pub struct RowOrigin {
    /// The variable's own write: its Insert, or its own Modify or Rename after its Insert or Get in the same transaction
    /// (record/crud/SbStaleOwnModify.al). Else a Get.
    pub own_write: bool,
    /// The row version the buffer got.
    pub version: i64,
    /// The session's commit count at that moment.
    pub commits: u64,
    /// `own_write` holds because the buffer read, by Get or GetBySystemId, a version written in the transaction that was open
    /// then, not because the variable wrote it (record/crud/SbStaleOwnModify.al:467-640, al-runner issue 5913).
    pub read_in_write_tx: bool,
    /// The buffer got its row version from a TransferFields out of a record, not from a read: only a LockTable and Get of the row
    /// in the new transaction lets its stale write through, a Modify of the row does not
    /// (record/crud/Y76StaleBuffers.al Y76_Part1_Copy_TransferFromRecord).
    pub lock_only: bool,
    /// The variable's own Modify followed its Insert or Get in the same transaction (a buffer with a Modify and one without are
    /// told apart by record/crud/Y76StaleBuffers.al Y76_Part2_*).
    pub own_modified: bool,
    /// A read by a Find, FindFirst, FindLast, FindSet or Next, not by Get, gave the buffer its committed version.
    pub from_find: bool,
    /// A `Get` gave the buffer its version (not GetBySystemId, not a Find), committed or written in the open transaction. Read
    /// only by the xRec rule of record/validate/XRecTableProcThroughCodeunit.al (al-vm `records/nested_validate.rs` `measured_buffer`).
    pub by_get: bool,
}

impl PartialEq for RowOrigin {
    fn eq(&self, other: &Self) -> bool {
        let RowOrigin {
            own_write,
            version,
            commits,
            read_in_write_tx,
            lock_only,
            own_modified,
            from_find,
            by_get: _,
        } = *self;
        (
            own_write,
            version,
            commits,
            read_in_write_tx,
            lock_only,
            own_modified,
            from_find,
        ) == (
            other.own_write,
            other.version,
            other.commits,
            other.read_in_write_tx,
            other.lock_only,
            other.own_modified,
            other.from_find,
        )
    }
}

impl Eq for RowOrigin {}

impl RowOrigin {
    /// An origin at `version` with the session's `commits`, no Get in a write transaction, no lock rule and no own Modify.
    pub fn new(own_write: bool, version: i64, commits: u64) -> RowOrigin {
        RowOrigin {
            own_write,
            version,
            commits,
            read_in_write_tx: false,
            lock_only: false,
            own_modified: false,
            from_find: false,
            by_get: false,
        }
    }
}

/// What an action's `RunPageLink` fills into Rec before OnOpenPage (field position, value) and the link's filters
/// (`PageState::open_rec_fill`).
pub type RunPageLinkFill = (Vec<(usize, Value)>, Rc<Vec<crate::filter::FieldFilter>>);

/// What `Record.IsDirty` compares a record variable's buffer with: a copy of the fields the variable last read or wrote
/// (record/ACRecordIsDirty.al). BC compares values, not writes: a field set back to what was read is not dirty
/// (ACRecordIsDirty.al:54-56, `after_value_restored`).
///
/// `Clone` gives `Unknown`: whether a copy of a record state (xRec, a row image, `RecordRef.GetTable`) keeps the snapshot is
/// not pinned. `:=` copies it with [`DirtyBase::copy`] (ACRecordIsDirty.al:63-64, `copy_dirty`), and so do Copy and a record
/// passed by value to a procedure (record/Y8RecordIsDirtyShapes.al:236-242, :321-324).
#[derive(Debug)]
pub enum DirtyBase {
    /// Nothing pins what IsDirty answers; the text says why.
    Unknown(&'static str),
    /// A declared variable nothing was read into: compared with a fresh buffer (ACRecordIsDirty.al:38-43).
    Fresh,
    /// The variable read or wrote its buffer and no field was written since: the snapshot is the buffer itself. A write
    /// copies it first ([`RecordState::before_field_write`]), so a read costs no copy.
    Synced,
    /// The fields the variable last read or wrote. `alt` is a second candidate where the corpus does not tell which one BC
    /// keeps (after TransferFields and Delete): IsDirty answers only where both give the same answer. `dirty_only`: the
    /// snapshot was kept by a read or write that answered false (or a `Delete(true)`), which the corpus pins only on a buffer
    /// that differs from it (record/Y8RecordIsDirtyShapes.al:278-297, :206-208): an answer "not dirty" is refused.
    /// `stale_store`: a ModifyAll or DeleteAll changed the stored row behind the variable (Y8RecordIsDirtyShapes.al:410-421),
    /// so IsDirty does not check its answer against the stored row.
    Row {
        base: Vec<Value>,
        alt: Option<Vec<Value>>,
        dirty_only: bool,
        stale_store: bool,
    },
}

impl Default for DirtyBase {
    fn default() -> Self {
        DirtyBase::Unknown("a record state that is not a declared variable")
    }
}

impl Clone for DirtyBase {
    fn clone(&self) -> Self {
        DirtyBase::Unknown("a copy of a record variable (passed by value, xRec, a row image, a RecordRef)")
    }
}

impl DirtyBase {
    /// A real copy, for `:=` between record variables.
    pub fn copy(&self) -> DirtyBase {
        match self {
            DirtyBase::Unknown(why) => DirtyBase::Unknown(why),
            DirtyBase::Fresh => DirtyBase::Fresh,
            DirtyBase::Synced => DirtyBase::Synced,
            DirtyBase::Row {
                base,
                alt,
                dirty_only,
                stale_store,
            } => DirtyBase::Row {
                base: base.clone(),
                alt: alt.clone(),
                dirty_only: *dirty_only,
                stale_store: *stale_store,
            },
        }
    }

    /// `Synced` becomes a copy of `fields`, before something writes them.
    pub fn materialize(&mut self, fields: &[Value]) {
        if matches!(self, DirtyBase::Synced) {
            *self = DirtyBase::Row {
                base: fields.to_vec(),
                alt: None,
                dirty_only: false,
                stale_store: false,
            };
        }
    }
}

impl RecordState {
    /// How many of the record's own copies of its fields hold Blob buffer `b` at position `pos`: the fields and the snapshots
    /// IsDirty, RecordRef.IsDirty and Modify compare with (`is_dirty_base`, `dirty_base`, `last_modify`).
    pub fn blob_copies_of(&self, pos: usize, b: &crate::stream::BlobRef) -> usize {
        let holds = |row: &[Value]| matches!(row.get(pos), Some(Value::Blob(o)) if std::sync::Arc::ptr_eq(o, b));
        // Without a vector of the rows: every write through an OutStream on a temporary record's Blob counts here.
        let (base, alt) = match &self.is_dirty_base {
            DirtyBase::Row { base, alt, .. } => (Some(base.as_slice()), alt.as_deref()),
            _ => (None, None),
        };
        [
            Some(&self.fields[..]),
            self.dirty_base.as_deref(),
            self.last_modify.as_deref(),
            base,
            alt,
        ]
        .into_iter()
        .flatten()
        .filter(|r| holds(r))
        .count()
    }

    /// A copy of the record that is a variable of its own: a by-value parameter (also a temporary one). It reads the
    /// original's DECLARED value, not its current one, and `Clear` on it goes back to that value; the original keeps its
    /// current value (al-language/reports/runtime/ACAttributes.al:397, :411-414; al-language/reports/runtime/
    /// SFSecurityFilteringFlow.al:279-306, :341-349, :371-373). A copy the corpus does not measure goes through
    /// [`RecordState::security_filtering_unmeasured_copy`] as well, and so does a copy of an element of an array declared with
    /// the attribute (the probe reads such elements only directly). Going back to the declared value is what `Clear` does, so a
    /// declared value that came with an unmeasured copy stays unpinned here too.
    pub fn copy_as_new_variable(&self) -> RecordState {
        let mut st = self.clone();
        st.security_filtering_clear();
        st.security_filtering_array = false;
        if self.security_filtering_array {
            st.security_filtering_unmeasured_copy(self);
        }
        st
    }

    /// What `SecurityFiltering()` reads and what `Clear` gives are not pinned at all: an element of an array declared with
    /// the attribute that went whole into a by-value parameter, and a variable declared with the attribute whose declared value
    /// the runtime does not keep (al-ir `Slot::security_filtering_not_kept`). Both reads are refused.
    pub fn security_filtering_not_measured(&mut self) {
        self.security_filtering_unpinned = true;
        self.declared_security_filtering_unpinned = true;
    }

    /// The copy of `original` is an event argument, a record handed to StartSession, a Variant's record or an array element
    /// passed by value: what SecurityFiltering() reads on it is pinned only when `original` holds its declared value
    /// (al-language/reports/runtime/ACAttributes.al:397, :411-414), and what Clear on it gives only when that value is
    /// Validated (the probe al-language/reports/runtime/SFSecurityFilteringFlow.al does not ask the rest). Otherwise the
    /// read is refused.
    pub fn security_filtering_unmeasured_copy(&mut self, original: &RecordState) {
        if original.security_filtering != original.declared_security_filtering {
            self.security_filtering_unpinned = true;
        }
        if original.declared_security_filtering != 0 {
            self.declared_security_filtering_unpinned = true;
        }
    }

    /// [`RecordState::copy_as_new_variable`] for a copy the corpus does not measure
    /// ([`RecordState::security_filtering_unmeasured_copy`]).
    pub fn copy_as_unmeasured_variable(&self) -> RecordState {
        let mut st = self.copy_as_new_variable();
        st.security_filtering_unmeasured_copy(self);
        st
    }

    /// The value `SecurityFiltering()` reads, refused when the corpus does not pin it.
    pub fn security_filtering_read(&self) -> Result<i32, crate::native::Signal> {
        if self.security_filtering_unpinned {
            return Err(crate::native::Signal::unsupported(
                "SecurityFiltering() of a record whose value the corpus does not pin: a copy (an event argument, a session's record, a Variant's record or an array element) whose value is not its declared one, Clear on such a copy of a record declared with another value than Validated, an element of an array declared with the attribute that was passed whole by value, or a variable declared with the attribute whose value the runtime does not keep (not pinned)",
            ));
        }
        Ok(self.security_filtering)
    }

    /// `Clear`: back to the declared value (al-language/reports/runtime/ACAttributes.al:373-374, :388-396; a codeunit's global:
    /// :434-435, :466-469), pinned unless that value came with a copy the corpus does not measure
    /// ([`RecordState::security_filtering_unmeasured_copy`]).
    pub fn security_filtering_clear(&mut self) {
        self.security_filtering = self.declared_security_filtering;
        self.security_filtering_unpinned = self.declared_security_filtering_unpinned;
    }

    /// A field of the buffer is about to be written: a snapshot that is the buffer itself becomes a copy.
    #[inline]
    pub fn before_field_write(&mut self) {
        if matches!(self.is_dirty_base, DirtyBase::Synced) {
            let RecordState {
                is_dirty_base, fields, ..
            } = self;
            is_dirty_base.materialize(fields);
        }
    }
}

/// The global variables of a table and of its table extensions, as one record variable holds
/// them (ADR-021). They live as long as the variable: every procedure and trigger call on it
/// runs on the same `Instance`, created on the first call. `Clone` gives an EMPTY set, because
/// a copy of a record state (a record passed by value, a row image, `RecordRef.GetTable`) starts
/// with fresh globals; use [`TableGlobals::share`] to keep two states on one set.
///
/// The instances hold `Rec` and `xRec` only while a call runs (the VM puts back what the slots
/// held before), so a state owns its globals and nothing points back: no `Rc` cycle.
#[derive(Default)]
pub struct TableGlobals {
    // Interior mutability: a handle is made through a shared borrow of the record state, which a
    // caller may hold (`xrec_handle(rec, rec.borrow().fields.clone())`).
    data: RefCell<Option<Rc<RefCell<GlobalsData>>>>,
}

/// Debug prints the number of instances only: while a call runs, an instance points at its record.
impl std::fmt::Debug for TableGlobals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.data.borrow().as_ref().map_or(0, |d| d.borrow().instances.len());
        write!(f, "TableGlobals({n})")
    }
}

#[derive(Default)]
struct GlobalsData {
    /// One instance per object (the table, each table extension) that has been called.
    instances: Vec<(u32, Rc<RefCell<Instance>>)>,
    /// `xRec`'s own set: it persists with the variable and survives `Clear` and `Reset` of `Rec`.
    xrec: TableGlobals,
}

impl Clone for TableGlobals {
    fn clone(&self) -> Self {
        TableGlobals::default()
    }
}

impl TableGlobals {
    /// An empty set that other states can [`share`](TableGlobals::share).
    pub fn new_shared() -> TableGlobals {
        TableGlobals {
            data: RefCell::new(Some(Rc::default())),
        }
    }

    /// Whether nothing has been called on this set yet.
    pub fn is_unset(&self) -> bool {
        self.data.borrow().is_none()
    }

    fn data(&self) -> Rc<RefCell<GlobalsData>> {
        self.data.borrow_mut().get_or_insert_with(Default::default).clone()
    }

    /// A second handle onto this very set (not a copy). Creates the set if there is none yet.
    pub fn share(&self) -> TableGlobals {
        TableGlobals {
            data: RefCell::new(Some(self.data())),
        }
    }

    /// The instance of `object`, if one was created.
    pub fn instance(&self, object: u32) -> Option<Rc<RefCell<Instance>>> {
        let data = self.data.borrow();
        let d = data.as_ref()?.borrow();
        d.instances.iter().find(|(o, _)| *o == object).map(|(_, i)| i.clone())
    }

    pub fn set_instance(&self, object: u32, inst: Rc<RefCell<Instance>>) {
        self.data().borrow_mut().instances.push((object, inst));
    }

    /// Every instance created so far.
    pub fn instances(&self) -> Vec<(u32, Rc<RefCell<Instance>>)> {
        match &*self.data.borrow() {
            Some(d) => d.borrow().instances.clone(),
            None => Vec::new(),
        }
    }

    /// The instances of `xRec`'s set, without making the set ([`TableGlobals::xrec`] makes it).
    pub fn xrec_instances(&self) -> Vec<(u32, Rc<RefCell<Instance>>)> {
        match &*self.data.borrow() {
            Some(d) => d.borrow().xrec.instances(),
            None => Vec::new(),
        }
    }

    /// The address of the shared set (`None` while unset) and how many strong owners it has: the identity a session image
    /// (al-vm `login_image`) keys it by, and the count it checks every owner against.
    pub fn set_address(&self) -> Option<(usize, usize)> {
        (self.data.borrow().as_ref()).map(|d| (Rc::as_ptr(d) as usize, Rc::strong_count(d)))
    }

    /// The xRec set of this set, when one was made ([`TableGlobals::xrec`] makes it); `None` makes nothing.
    pub fn made_xrec(&self) -> Option<TableGlobals> {
        let data = self.data.borrow();
        let d = data.as_ref()?.borrow();
        (!d.xrec.is_unset()).then(|| d.xrec.share())
    }

    /// A handle onto `xRec`'s set (created on first use).
    pub fn xrec(&self) -> TableGlobals {
        let data = self.data();
        let d = data.borrow();
        d.xrec.share()
    }

    /// Calls `f` for every instance of the set, and of `xRec`'s set, when no other handle shares them
    /// (`instance_cycles`).
    pub(crate) fn owned_instances(&self, f: &mut dyn FnMut(&Rc<RefCell<Instance>>)) {
        let Ok(data) = self.data.try_borrow() else { return };
        let Some(d) = data.as_ref().filter(|d| Rc::strong_count(d) == 1) else {
            return;
        };
        let Ok(d) = d.try_borrow() else { return };
        for (_, i) in &d.instances {
            f(i);
        }
        d.xrec.owned_instances(f);
    }
}

/// A set of field positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldSet {
    All,
    Of(Vec<usize>),
}

impl FieldSet {
    pub fn contains(&self, pos: usize) -> bool {
        match self {
            FieldSet::All => true,
            FieldSet::Of(v) => v.contains(&pos),
        }
    }

    pub fn add(&mut self, pos: usize) {
        if let FieldSet::Of(v) = self {
            if !v.contains(&pos) {
                v.push(pos);
            }
        }
    }

    pub fn remove(&mut self, pos: usize) {
        if let FieldSet::Of(v) = self {
            v.retain(|&p| p != pos);
        }
    }
}

/// What a record's buffer holds after a fetch (partial records, `Record.AreFieldsLoaded`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    /// The fields known to be loaded.
    pub have: FieldSet,
    /// Fields outside `have` whose state the corpus does not pin: asking about one is refused.
    pub unsure: FieldSet,
    /// The primary key of the row fetched, when the buffer is partial (`None` for a full one).
    pub row: Option<String>,
    /// `Clear` kept this buffer: it carries over to a fetch of the same row only; a fetch of another row starts afresh
    /// (record/TestRecordPartialLoadNarrowAfterFetch.al:121-185).
    pub cleared: bool,
    /// Fields assigned while not loaded: a load of the missing fields (a read of an unloaded field, a Modify) keeps their
    /// values (record/load-fields/P81LoadFields.al:1169-1188).
    pub assigned: Vec<usize>,
    /// A read of an unloaded field loaded every missing field: a fetch of another row then loads its load set and nothing
    /// carries over (record/load-fields/P81LoadFields.al:1047-1120 `key_after_next_loaded_Qty` No).
    pub missing_loaded: bool,
    /// The row as it was fetched: a field that is not loaded and no longer holds its fetched value was changed by AL code
    /// (an assignment, Validate, TransferFields, ...), and a load of the missing fields keeps it.
    pub fetched: FetchedRow,
    /// `Init` ran since the fetch: the fields that were not loaded hold their initial values, and neither a read nor a write
    /// loads them from the row (record/load-fields/R7000InitAfterLoadFields.al, corpus branch al-runtime-rs-staging-r6961).
    pub init: bool,
}

/// The fields of a row as it was fetched into a partial buffer (`Buffer::fetched`). Buffers compare equal whatever it holds.
#[derive(Debug, Clone, Default)]
pub struct FetchedRow(pub Option<Rc<Vec<Value>>>);

impl PartialEq for FetchedRow {
    fn eq(&self, _: &FetchedRow) -> bool {
        true
    }
}

impl Eq for FetchedRow {}

/// Partial-record state of a record variable. A default one is a full load that fetched nothing.
#[derive(Debug, Clone, Default)]
pub struct LoadState {
    /// `SetLoadFields`: the field positions requested; `None` is the table's default (all).
    pub request: Option<Vec<usize>>,
    /// What the buffer holds; `None` until a row was fetched into it (and after `Clear`).
    pub buffer: Option<Buffer>,
    /// The buffer is partial: a read of an omitted field loads it, so reads and writes of
    /// fields tell `touch` and `written`. Kept as its own flag so the full-load path pays one
    /// branch.
    pub partial: bool,
}

impl LoadState {
    /// A field of the buffer was read: an omitted one is loaded on demand
    /// (record/TestRecordPartialLoad.al:144-190).
    pub fn touch(&mut self, pos: usize) {
        if let Some(b) = &mut self.buffer {
            if !b.have.contains(pos) {
                b.have.add(pos);
                b.unsure.remove(pos);
            }
        }
    }

    /// A field of the buffer was assigned. Whether that counts as loading it is not pinned.
    pub fn written(&mut self, pos: usize) {
        if let Some(b) = &mut self.buffer {
            if !b.have.contains(pos) {
                b.unsure.add(pos);
                if !b.assigned.contains(&pos) {
                    b.assigned.push(pos);
                }
            }
        }
    }
}

/// What a `WebServiceActionContext` holds: the object an API action points the OData layer at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WebServiceActionState {
    pub object_id: i32,
    /// Ordinal in the platform's object type option; 0 is the unnamed default.
    pub object_type: i32,
    /// Index into the `WebServiceActionResultCode` members; 0 is `None`.
    pub result_code: i32,
    /// Field ids that have an entity key.
    pub keys: Vec<i32>,
}

/// One control of a `FilterPageBuilder`: a name and the record buffer whose filters it holds.
#[derive(Debug, Clone)]
pub struct FilterControl {
    pub name: String,
    /// Object index of the table.
    pub table: u32,
    /// Shared between copies of the builder (corpus pages/filterpagebuilder/TestFilterPageBuilder.al:
    /// FilterPageBuilder_Assignment_ViewOnCopyDoesReachOriginal).
    pub record: Rc<RefCell<RecordState>>,
}

/// What a `FilterPageBuilder` holds. An untouched one is `LibraryValue::Empty`.
#[derive(Debug, Clone, Default)]
pub struct FilterPageState {
    pub controls: Vec<FilterControl>,
    /// `None` until assigned: the getter then answers a default.
    pub caption: Option<String>,
}

/// What a `SessionSettings` holds. A property nobody wrote is `None`: the corpus does not say what
/// a pristine language id, locale id, time zone or profile id reads as, so reading one is refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionSettingsState {
    pub language_id: Option<i32>,
    pub locale_id: Option<i32>,
    pub time_zone: Option<String>,
    pub profile_id: Option<String>,
    pub profile_app_id: Option<[u8; 16]>,
    pub company: String,
}

/// Which collection of headers a bag is: they differ in the headers they accept (a request, response or default
/// header collection refuses the content headers; the content's own accepts them).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HeaderKind {
    #[default]
    Request,
    Content,
    Response,
    /// `HttpClient.DefaultRequestHeaders`.
    Default,
}

/// The headers of a request, response, content or client: names with the casing they were first added with,
/// each with its values in order. Secret values (added as `SecretText`) are kept apart.
#[derive(Debug, Default)]
pub struct HeaderBag {
    pub kind: HeaderKind,
    pub items: Vec<(String, Vec<String>)>,
    pub secret: Vec<(String, Vec<String>)>,
    /// The headers of a content written from a SecretText: they list one header that is not the Content-Type, which no test
    /// names (xml/xml/P97ContentSecret.al:Secret_Content_Headers `keys` 1, `has_type` No). Reading
    /// the list, or a header other than the Content-Type, is refused.
    pub unnamed_secret_header: bool,
    /// `HttpRequestMessage.SetCookie` wrote the `Cookie` header of these request headers (http::request_cookies).
    pub set_cookies: bool,
}

/// The headers of an `HttpContent`, request, response or client, shared with every `HttpHeaders` handed out.
pub type HttpHeaderList = Rc<RefCell<HeaderBag>>;

/// A header bag of `kind`.
pub fn new_header_list(kind: HeaderKind) -> HttpHeaderList {
    Rc::new(RefCell::new(HeaderBag {
        kind,
        ..HeaderBag::default()
    }))
}

/// What an `HttpRequestMessage` holds (corpus http/TestHttpRequestMessage.al,
/// http/HttpRequest.al). Nothing is sent but through a test handler.
#[derive(Debug, Clone)]
pub struct HttpRequestState {
    pub method: Option<String>,
    pub uri: Option<String>,
    /// The uri came from `SetSecretRequestUri`: `GetRequestUri` then reads empty
    /// (xml/xml/P97ReqSecretUri.al:79, :110).
    pub uri_secret: bool,
    /// The request's content: a copy of the content it was set from, or the uninitialized one its first read handed out
    /// (`http::request_content`).
    pub content: Option<Value>,
    pub headers: HttpHeaderList,
    /// The request was assigned, passed by value, put into a Variant or sent (`http::request_handed_on`). The variables
    /// share the request; the mark
    /// only says that no evidence for `ReadAs(SecretText)` covers its content (`http::SecretReadShape`).
    pub handed_on: bool,
}

impl Default for HttpRequestState {
    fn default() -> Self {
        HttpRequestState {
            method: None,
            uri: None,
            uri_secret: false,
            content: None,
            headers: new_header_list(HeaderKind::Request),
            handed_on: false,
        }
    }
}

/// What an `HttpContent` holds (corpus http/TestHttpRequestMessage.al:34-43, http/content/HttpContent.al).
#[derive(Debug, Clone)]
pub struct HttpContentState {
    /// The body as bytes (text is written as UTF-8).
    pub data: Vec<u8>,
    pub headers: HttpHeaderList,
    /// The content a request has when nobody set one: `ReadAs` answers false, `GetHeaders` raises.
    pub uninitialized: bool,
    /// The bytes were handed out as an `InStream`: a second `ReadAs(InStream)` gets an empty stream.
    pub stream_taken: bool,
    /// A `WriteFrom` has put a body here (a second one may add a Content-Length, see `write_body`).
    pub written: bool,
    /// The value is a copy an assignment or a by-value parameter made (`deep_copy`), not the content a request handed out.
    /// On an uninitialized content it decides what is not pinned for a copy: `ReadAs(InStream)` as a statement and
    /// `GetHeaders` with the result used refuse (`ReadAs(Text)` answers the same for both). It is also part of the rule
    /// for a body already handed out as a stream (`read_as`).
    pub copied: bool,
    /// The last `WriteFrom` took an `InStream`.
    pub from_stream: bool,
    /// The last `WriteFrom` took a `SecretText` (xml/xml/P97ContentSecret.al).
    pub secret: bool,
    /// The calls made on the content so far, as far as the evidence for `ReadAs(SecretText)` follows them
    /// (`http::SecretReadShape`).
    pub secret_read: crate::http::SecretReadShape,
    /// No method was called on this content yet; it was only assigned, passed by value or handed on otherwise
    /// (`http::content_handed_on`). It stands for the variable main keeps as `LibraryValue::Empty` (`http::content_unused`).
    pub untouched: bool,
}

impl Default for HttpContentState {
    fn default() -> Self {
        let headers = new_header_list(HeaderKind::Content);
        headers
            .borrow_mut()
            .items
            .push(("Content-Type".into(), vec!["text/plain; charset=utf-8".into()]));
        HttpContentState {
            data: Vec::new(),
            headers,
            uninitialized: false,
            stream_taken: false,
            written: false,
            copied: false,
            from_stream: false,
            secret: false,
            secret_read: crate::http::SecretReadShape::Other,
            untouched: false,
        }
    }
}

/// What a `Cookie` holds (metadata/session-system/P94Cookie.al).
#[derive(Debug, Clone, Default)]
pub struct CookieState {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    /// A `DateTime` value, 0 for none.
    pub expires: i64,
}

/// What an `HttpResponseMessage` holds (http/HttpResponse.al). A response nobody filled is
/// 200 OK with an empty body.
#[derive(Debug, Clone)]
pub struct HttpResponseState {
    pub status: i32,
    pub reason: String,
    /// A handler set a status whose reason phrase BC gives is not measured: reading `ReasonPhrase` is refused.
    pub reason_unpinned: bool,
    /// The `HttpContent` handle, shared with `Content`.
    pub content: Value,
    pub headers: HttpHeaderList,
    pub blocked: bool,
}

/// What an `HttpClient` holds (http/HttpClient.al).
#[derive(Debug, Clone)]
pub struct HttpClientState {
    pub base_address: String,
    pub timeout_ms: i64,
    pub default_headers: HttpHeaderList,
    /// `UseServerCertificateValidation` was called on this client (also before a `Clear`): a request of it that no handler
    /// answers is refused, since what the setting does to such a request is not pinned.
    pub cert_validation_set: bool,
    /// The value of the last `UseServerCertificateValidation` since the client was made or cleared.
    pub cert_validation_last: Option<bool>,
    /// A setting call left the defaults and rebuilt the client (`http::client_settings_change`).
    pub rebuilt: bool,
    /// The client was rebuilt and then cleared: whether a later setting call rebuilds it again is not pinned.
    pub cleared_after_rebuild: bool,
    /// `UseDefaultNetworkWindowsAuthentication` or `UseWindowsAuthentication` was called (also before a `Clear`): a request
    /// of the client that no handler answers is refused (http::windows_auth).
    pub windows_auth_set: bool,
}

impl Default for HttpClientState {
    fn default() -> Self {
        HttpClientState {
            base_address: String::new(),
            timeout_ms: 100_000,
            default_headers: new_header_list(HeaderKind::Default),
            cert_validation_set: false,
            cert_validation_last: None,
            rebuilt: false,
            cleared_after_rebuild: false,
            windows_auth_set: false,
        }
    }
}

/// What a `TestHttpRequestMessage` holds: what a `[HttpClientHandler]` sees of the request.
#[derive(Debug, Clone)]
pub struct TestHttpRequestState {
    /// Scheme, host and path without the query string.
    pub path: String,
    /// The `Dictionary of [Text, Text]` of the decoded query, shared with `QueryParameters`.
    pub query: Value,
    pub has_secret_uri: bool,
    /// The `HttpRequestType` enum value (`Null` for a request no client call made).
    pub request_type: Value,
}

/// What a `TestHttpResponseMessage` holds: what a `[HttpClientHandler]` answers.
#[derive(Debug, Clone)]
pub struct TestHttpResponseState {
    /// The request went out for real (no handler answered it) and failed before any connection
    /// (`http::failure_without_connection`): the response reads status 0 and keeps nothing of the handler's.
    pub request_failed: bool,
    /// The error a failed request leaves as the last error (text, code) when the call's result is used.
    pub failure: Option<(String, String)>,
    pub status: i32,
    pub reason: String,
    /// The handler set `ReasonPhrase`; when it did not, BC gives the reason phrase of the status.
    pub reason_set: bool,
    /// See [`HttpResponseState::reason_unpinned`].
    pub reason_unpinned: bool,
    pub success: bool,
    pub blocked: bool,
    pub content: Value,
    pub headers: HttpHeaderList,
}

/// What a `Notification` holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotificationState {
    pub message: Option<String>,
    /// `Id := guid` (ui-handlers/TestNotificationHandler.al:55-56); the default is not pinned.
    pub id: Option<[u8; 16]>,
    /// Ordinal in the `NotificationScope` members.
    pub scope: Option<i32>,
    pub data: Vec<(String, String)>,
    /// (Caption, codeunit id, method name, description): kept, never run.
    pub actions: Vec<(String, i32, String, Option<String>)>,
}

/// What an `ErrorInfo` holds (corpus errors/errorinfo/TestErrorInfoType.al).
#[derive(Debug, Clone)]
pub struct ErrorInfoState {
    pub message: String,
    pub title: String,
    pub detailed_message: String,
    pub control_name: String,
    /// Indexes into the `Verbosity` and `DataClassification` members.
    pub verbosity: i32,
    pub data_classification: i32,
    /// `None` until a test sets one: reads as Client.
    pub error_type: Option<i32>,
    pub collectible: bool,
    pub table_id: i32,
    /// The `RecordId` value of the record `Create` was given.
    pub record_id: Option<Value>,
    pub system_id: Option<Value>,
    pub field_no: i32,
    pub page_no: i32,
    /// The dictionary the test assigned, held by reference.
    pub custom_dimensions: Option<Value>,
    /// How the ErrorInfo was built, for `GetLastErrorText(true)` after `FieldError(Field, ErrorInfo)`.
    pub built: ErrorInfoBuilt,
}

/// How an `ErrorInfo` was built: `Create()` and nothing set after it, `Create('literal')` with one string literal and
/// nothing set after it, or any other way (a Label, a variable, more arguments, a property set, an action added, or the
/// `CustomDimensions` dictionary handed out). The setters of `errorinfo.rs` make it `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorInfoBuilt {
    CreateNoMessage,
    CreateLiteral,
    Other,
}

/// What a `ModuleInfo` reports: an app's manifest (corpus session-system/TestNavAppExtended.al,
/// session-system/TestBCPlatformContracts.al, session-system/TestMiscTypes.al).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInfoState {
    pub id: [u8; 16],
    pub name: String,
    pub publisher: String,
    /// `None` when the manifest gave none the runtime can read.
    pub version: Option<[i32; 4]>,
    pub dependencies: Vec<[u8; 16]>,
    /// The module BC makes up for the null app id: only its data version is pinned
    /// (session-system/TestEnvironmentUserTenantContracts.al:192-198); the other properties are refused.
    pub synthesized: bool,
}

/// The global variables of one codeunit instance.
#[derive(Debug, Default)]
pub struct Instance {
    pub object: u32,
    pub globals: Vec<Value>,
    pub initialized: bool,
    /// A report variable a `Run` just reset: its next method call other than `Run` runs `OnInitReport` again,
    /// a `Run` does not (reports/TestProbeReportContracts.al:ReportVariable_RunTwice_GlobalsReset against
    /// reports/TestReportRuntimeContracts.al:Report_Run_ResetsTheGlobals_NotOnInitReport_RunTwiceIsAllowed_ExecuteTwiceThrows).
    pub rerun: bool,
    /// For the instance of a page or report extension: the instance of the page or report it
    /// extends (ADR-023). `Weak`, so the link adds no reference cycle. `None` for every other
    /// instance.
    pub base: Option<std::rc::Weak<RefCell<Instance>>>,
    /// For the instance of a report variable: the instances of the reportextensions that change it, by object
    /// index, made at the first call of an extension procedure or run of the variable and kept with the variable's
    /// state (a `Run` and `Clear` drop them). Empty for every other instance.
    pub exts: Vec<(u32, Rc<RefCell<Instance>>)>,
    /// For the instance of an xmlport variable: the separators a program set (`FieldDelimiter(..)`, ...), as given, by
    /// kind (see `al-vm/src/xmlports.rs` `SEPARATORS`). Empty until one is set.
    pub xp_text: Vec<(u8, String)>,
    /// Whether the instance was made while the session's permissions were lowered (ADR-062 stage 3): BC checks Execute on an
    /// object when an instance made under restricted permissions is used, and an instance made before the restriction (a global
    /// of the test codeunit, the Permissions Mock) is not checked (Y24PermExecute.al: `Mock.ClearAssignments()` and `Observer`
    /// run under an exact set that grants them no Execute).
    pub born: Born,
    /// The session watches this single instance's globals (al-vm `single_track.rs`).
    pub single: bool,
    /// What a read of one of this single instance's globals does, when the run found its state somewhere no pin covers
    /// (al-vm `single_unmeasured.rs`). `None` for every other instance.
    pub unmeasured: Option<SingleMark>,
    /// Counts the instance as live while it exists (`instance_cycles::live`).
    pub live: crate::instance_cycles::LiveCount,
}

/// The mark of a single instance whose state the run found somewhere no pin covers ([`Instance::unmeasured`]).
#[derive(Debug, Clone)]
pub enum SingleMark {
    /// What it holds is not BC's value: every read of one of its globals is refused with this reason.
    Refused(Rc<str>),
    /// What it holds is BC's value by this assumption entry: every read of one of its globals marks it.
    Assumed(crate::assumptions::Assumption),
}

thread_local! {
    static BORN_RESTRICTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Set by the session whenever its permissions are lowered or restored; read when an [`Instance`] is made.
pub fn set_born_restricted(restricted: bool) {
    BORN_RESTRICTED.with(|c| c.set(restricted));
}

/// See [`Instance::born`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Born(pub bool);

impl Default for Born {
    fn default() -> Born {
        Born(BORN_RESTRICTED.with(std::cell::Cell::get))
    }
}

/// What a `TestHandlerContext` handed to an ITestHandler callback holds (corpus al-language-testlibraries/test-framework/
/// data-driven-tests-and-handlers/BiTestHandler.al:79-82, :125 and BiTestHandlerDefault.al:152-155, :286).
#[derive(Debug, Default, Clone)]
pub struct HandlerContext {
    pub codeunit_id: i32,
    /// The test codeunit's name, namespace first (BiTestHandlerBehaviorNs.al:26).
    pub codeunit_name: String,
    /// Empty in the codeunit callbacks.
    pub procedure: String,
    /// Empty except in the case callbacks.
    pub case: String,
    /// Whether the test passed, in the After callbacks; false in every Before callback
    /// (BiTestHandlerSuccess.al:116, al-runner issue 7181).
    pub success: bool,
    /// A Before callback of a default handler: its `success` (false) rests on assumption
    /// `test-handler-default-before-success` (the corpus pins it for a `TestHandlers` handler only).
    pub success_assumed: bool,
    /// The reason of a `Skip` call in this callback.
    pub skip: Option<String>,
}

/// How a RecordRef or a KeyRef came to hold nothing ([`LibraryValue::EmptiedRef`]): a probe measured some members in the first
/// three states, none in the others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptiedRef {
    /// A RecordRef that was open, after `Clear`.
    ClearedOpenRecordRef,
    /// A RecordRef never opened, after an `Open(0)` that raised nothing.
    ZeroOpenedRecordRef,
    /// A KeyRef that held a key, after `Clear`.
    ClearedAssignedKeyRef,
    /// A RecordRef that holds nothing after another sequence: `Clear` of one that was not open, or an `Open` or a `Get` that left
    /// one emptied (any state above) not open.
    OtherRecordRef,
    /// A KeyRef that held nothing, after `Clear`.
    OtherKeyRef,
}

/// State behind a library-type handle; grows with the builtins that need it.
#[derive(Debug, Default)]
pub enum LibraryValue {
    #[default]
    Empty,
    /// An InStream or OutStream after `Clear`: it holds no stream, as one never created, but the corpus measured fewer of its
    /// uses (streams/instream-outstream/Y62GoneInStream.al against StreamUncreatedUse.al; `stream_cleared`).
    ClearedStream,
    /// The XmlNode that `AsXmlNode` of an unassigned XmlElement returns (al-builtins `xml/unassigned_element.rs`): only
    /// `IsXmlElement` and `WriteTo(Text)` are admitted on it (`is_node_of_unassigned_element`); every other use refuses.
    XmlNodeOfUnassignedElement,
    /// A RecordRef after `Close`: not open, as one never opened (`Empty`), but the corpus measured fewer of its members
    /// (al-vm `unopened_refs.rs`). `recordref_closed::not_open` tells both apart from an open one.
    ClosedRecordRef,
    /// A RecordRef or a KeyRef that holds nothing after a call that emptied it: it answers as one never opened or never assigned,
    /// except the members the corpus measured only on a new variable (`recordref_closed`, al-vm `unopened_refs.rs`).
    EmptiedRef(EmptiedRef),
    /// A `DataSourceContext` or `TestHandlerContext` the test framework handed to a callback (ADR-059). Its members are
    /// refused there (`testcontexts.rs`).
    HandedContext,
    /// A `TestHandlerContext` the test runner hands to an ITestHandler callback (BC 29): what its members return there, and
    /// the reason of a `Skip` the callback made (`testcontexts.rs`, al-testrun `runtime/test_handlers.rs`).
    HandlerContext(Box<HandlerContext>),
    Text(String),
    /// A TextBuilder whose capacity was set or has to be kept: the text and the capacity in characters.
    TextCap(String, i32),
    /// An open `RecordRef`: it owns a record buffer exactly as a `Record`
    /// variable does (fields, filters, key), so record operations are shared.
    RecordRef(Rc<RefCell<RecordState>>),
    /// A `FieldRef`: the record buffer it points into and the field's position.
    FieldRef(Rc<RefCell<RecordState>>, usize),
    /// A `FieldRef` reached through `KeyRef.FieldIndex`: the same field, but the hidden key on SystemId names
    /// it `$systemId` (recordref-fieldref/recordref/TestRecordRefKeyIndexSetRecFilterContracts.al:93).
    KeyFieldRef(Rc<RefCell<RecordState>>, usize),
    /// A `KeyRef`: the record buffer and the key's index in the table's keys.
    KeyRef(Rc<RefCell<RecordState>>, usize),
    /// A `FieldRef` to one of the user-name system FlowFields (SystemCreatedByUserName 2000000005 to
    /// SystemModifiedByFullName 2000000008), which the record buffer does not hold: the record buffer, the field
    /// number, and the value `CalcField` calculated (`None` before it ran). See al-vm `refs.rs`.
    UserNameFieldRef {
        rec: Rc<RefCell<RecordState>>,
        no: u32,
        value: Option<String>,
    },
    /// A `RecordRef` opened on a system table the workspace has no schema for:
    /// only its number is known (corpus recordref-fieldref/recordref/TestRecordRefOpenCompilationTarget.al).
    SystemTable(i64),
    /// A `RecordId`: table object index and number, table name (for `Format`), primary key values.
    RecordId {
        table: u32,
        table_no: u32,
        name: String,
        key: Vec<Value>,
    },
    /// A `WebServiceActionContext` that has been written to (an untouched one is `Empty`).
    WebServiceAction(WebServiceActionState),
    /// A `SessionSettings` that has been written to (an untouched one is `Empty`).
    SessionSettings(SessionSettingsState),
    /// A `FilterPageBuilder` that has been written to (an untouched one is `Empty`).
    FilterPageBuilder(FilterPageState),
    /// A `Notification` that has been written to (an untouched one is `Empty`).
    Notification(NotificationState),
    /// A `ModuleInfo`: what one app reports about itself.
    ModuleInfo(ModuleInfoState),
    /// A `DataTransfer` that `SetTables` staged: (source table, destination table).
    DataTransfer(Option<(i32, i32)>),
    /// A `DataTransfer` staged inside install code: what its copy will do (al-vm `datatransfer`).
    DataTransferPlan(Box<crate::datatransfer::Plan>),
    /// A `Cookie`: an untouched one is `Empty`, which reads as no name, no value and no attributes.
    Cookie(CookieState),
    /// An `HttpRequestMessage` that was configured (an untouched one is `Empty`).
    HttpRequest(HttpRequestState),
    /// An `HttpContent` that was written to or asked for its headers (an untouched one is `Empty`).
    HttpContent(HttpContentState),
    /// An `HttpHeaders` handed out by `GetHeaders`, `Headers` or `DefaultRequestHeaders`: the owner's own list.
    HttpHeaders(HttpHeaderList),
    /// An `HttpResponseMessage` (an untouched one is `Empty`, which reads as 200 OK).
    HttpResponse(HttpResponseState),
    /// An `HttpClient` that was configured (an untouched one is `Empty`).
    HttpClient(HttpClientState),
    /// A `TestHttpRequestMessage` a handler was given (an untouched one is `Empty`).
    TestHttpRequest(TestHttpRequestState),
    /// A `TestHttpResponseMessage` (an untouched one is `Empty`).
    TestHttpResponse(TestHttpResponseState),
    /// A `Version`: major, minor, build, revision.
    Version([i32; 4]),
    /// An `ErrorInfo` (the zero-argument `Create` and `Create(Message, ...)` build one).
    ErrorInfo(ErrorInfoState),
    /// A JSON node behind a JsonObject / JsonArray / JsonToken / JsonValue handle.
    Json(crate::json::JsonCell),
    /// A node behind an Xml* handle.
    Xml(crate::xml::XmlCell),
    /// An XmlNodeList: a snapshot of nodes.
    XmlList(Vec<crate::json::Handle>),
    /// An XmlAttributeCollection: a view of one element's attributes.
    XmlAttrs(crate::json::Handle),
    /// An XmlNameTable (opaque).
    XmlNameTable,
    /// An `XmlWriteOptions` that has been set: `PreserveWhitespace`.
    /// An `XmlReadOptions` that has been set: `PreserveWhitespace` (what is read does not depend on it).
    XmlReadOptions {
        preserve_whitespace: bool,
    },
    XmlWriteOptions {
        preserve_whitespace: bool,
        /// Whether a document or element is written without indentation. A setter changes it only when it changes
        /// `preserve_whitespace`, and a new value starts unindented (see xml.rs `write_options_preserve_whitespace`).
        unindented: bool,
    },
    /// A `File` that was created or opened, or had a mode set (an untouched one is `Empty`).
    File(crate::file::FileState),
    /// An XmlNamespaceManager: its declarations and scopes.
    XmlNamespaces(crate::xml::NsScopes),
    /// An InStream: a snapshot being read.
    InStream(crate::stream::InState),
    /// An OutStream: appends to a Blob buffer. The last part is what the VM kept when the stream was opened on a Blob field of
    /// a temporary record ([`crate::stream::OutOpen`]).
    OutStream(
        crate::stream::BlobRef,
        crate::stream::Encoding,
        Option<Rc<crate::stream::OutOpen>>,
    ),
}

impl Value {
    /// Exact structural identity: the same variant with the same contents, down to a decimal's scale and sign, a
    /// list's mutation count and every flag of a Blob buffer. This is NOT AL equality (`=` coerces, ignores case
    /// and compares numbers by value); use it only to ask "is this exactly what it was". Handle-like variants
    /// (record, codeunit, library, test page, query, DotNet) are compared by what their `Debug` prints, as the
    /// page rows did before this function existed.
    pub fn identical(&self, other: &Value) -> bool {
        fn all(a: &[Value], b: &[Value]) -> bool {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.identical(y))
        }
        use Value::*;
        match (self, other) {
            (Null, Null) => true,
            (Boolean(a), Boolean(b)) => a == b,
            (Integer(a), Integer(b)) => a == b,
            (BigInteger(a), BigInteger(b)) => a == b,
            (Decimal(a), Decimal(b)) => a.serialize() == b.serialize(),
            (Byte(a), Byte(b)) => a == b,
            (Char(a), Char(b)) => a == b,
            (Text(a), Text(b)) | (Code(a), Code(b)) => a.as_str() == b.as_str(),
            (Date(a, f), Date(b, g)) => a == b && f == g,
            (Time(a), Time(b)) | (DateTime(a), DateTime(b)) | (Duration(a), Duration(b)) => a == b,
            (Guid(a), Guid(b)) => a == b,
            (DateFormula(a), DateFormula(b)) => a == b,
            (Option(a, n), Option(b, m)) => a == b && n[..] == m[..],
            (Enum(a, x), Enum(b, y)) => a == b && x == y,
            (List(a), List(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.version == b.version && all(&a, &b)
            }
            (Array(a), Array(b)) => all(&a.borrow(), &b.borrow()),
            (Dictionary(a), Dictionary(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.len() == b.len()
                    && a.iter()
                        .zip(b.iter())
                        .all(|((k, v), (l, w))| k.identical(l) && v.identical(w))
            }
            (Blob(a), Blob(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.bytes == b.bytes
                    && a.write_pos == b.write_pos
                    && a.loaded == b.loaded
                    && a.fetched == b.fetched
                    && a.refetch == b.refetch
                    && a.persisted == b.persisted
                    && a.written == b.written
                    && a.render_pending == b.render_pending
                    && a.first_write_truncates == b.first_write_truncates
            }
            (Codeunit(_), Codeunit(_))
            | (Library(..), Library(..))
            | (Record(_), Record(_))
            | (TestPage(_), TestPage(_))
            | (TestControl(_), TestControl(_))
            | (Query(_), Query(_))
            | (DotNet(_), DotNet(_)) => {
                struct Shown<'a>(&'a Value);
                impl std::fmt::Display for Shown<'_> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        std::fmt::Debug::fmt(self.0, f)
                    }
                }
                Shown(self).to_string() == Shown(other).to_string()
            }
            _ => false,
        }
    }

    /// `identical` over two rows of fields.
    pub fn all_identical(a: &[Value], b: &[Value]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.identical(y))
    }

    /// Whether the value holds no `Rc` handle: plain data (`Option` member lists and blob buffers
    /// are `Arc`s), so
    /// that rows made of such values can be read from several threads.
    pub fn is_plain(&self) -> bool {
        matches!(
            self,
            Value::Null
                | Value::Boolean(_)
                | Value::Integer(_)
                | Value::BigInteger(_)
                | Value::Decimal(_)
                | Value::Byte(_)
                | Value::Char(_)
                | Value::Text(_)
                | Value::Code(_)
                | Value::Date(..)
                | Value::Time(_)
                | Value::DateTime(_)
                | Value::Duration(_)
                | Value::Guid(_)
                | Value::DateFormula(_)
                | Value::Option(..)
                | Value::Enum(..)
                | Value::Blob(_)
                | Value::Unknown(_)
        )
    }

    /// Why the value cannot be read, for a column a platform provider left unknown.
    pub fn unknown(&self) -> Option<&str> {
        match self {
            Value::Unknown(why) => Some(why),
            _ => None,
        }
    }

    pub fn default_for(t: &RtType) -> Value {
        match t {
            RtType::Boolean => Value::Boolean(false),
            RtType::Integer => Value::Integer(0),
            RtType::BigInteger => Value::BigInteger(0),
            RtType::Decimal => Value::Decimal(Decimal::ZERO),
            RtType::Byte => Value::Byte(0),
            RtType::Char => Value::Char(0),
            RtType::Text(_) => Value::Text(String::new().into()),
            RtType::Code(_) => Value::Code(String::new().into()),
            RtType::Date => Value::Date(0, false),
            RtType::Time => Value::Time(0),
            RtType::DateTime => Value::DateTime(0),
            RtType::Duration => Value::Duration(0),
            RtType::Guid => Value::Guid([0; 16]),
            RtType::DateFormula => Value::DateFormula(String::new()),
            RtType::Option(m) => Value::Option(0, m.clone()),
            RtType::Enum(id) => Value::Enum(*id, 0),
            RtType::Variant => Value::Null,
            RtType::List(_) => Value::List(Rc::default()),
            RtType::Dictionary(..) => Value::Dictionary(Rc::default()),
            RtType::Array(dims, el) => {
                let n = dims.iter().map(|d| *d as usize).product();
                Value::Array(Rc::new(RefCell::new((0..n).map(|_| Value::default_for(el)).collect())))
            }
            RtType::Codeunit(id) => Value::Codeunit(crate::instance_cycles::shared(Instance {
                object: *id,
                ..Default::default()
            })),
            // An unassigned interface holds nothing.
            RtType::Interface(_) => Value::Null,
            RtType::TestPage(id) | RtType::Page(id) => Value::TestPage(PageHandle::closed(*id)),
            RtType::Query(id) => Value::Query(Rc::new(RefCell::new(QueryState::new(*id)))),
            // Records need the table's field types: the VM builds them.
            RtType::Record(..) => Value::Null,
            RtType::Library(crate::library_kind::LibraryKind::Blob) => Value::Blob(crate::stream::new_blob()),
            // A Media or MediaSet holds the id of its media (or set): a Guid, null while empty.
            RtType::Library(crate::library_kind::LibraryKind::Media | crate::library_kind::LibraryKind::MediaSet) => {
                Value::Guid([0; 16])
            }
            RtType::Library(name) => Value::Library(name.clone(), Rc::default()),
            // An unassigned DotNet variable is null (ADR-014 section 4).
            RtType::DotNet(_) => Value::Null,
        }
    }

    /// Length in UTF-16 code units, as AL's StrLen and MaxStrLen count it.
    pub fn text_len(s: &str) -> usize {
        s.encode_utf16().count()
    }

    /// AL's dynamic type tests (`Variant.IsInteger`, ...).
    pub fn is(&self, kind: &str) -> bool {
        let k = kind.to_ascii_lowercase();
        match self {
            Value::Null => false,
            Value::Boolean(_) => k == "boolean",
            Value::Integer(_) => k == "integer",
            Value::BigInteger(_) => k == "biginteger",
            Value::Decimal(_) => k == "decimal",
            Value::Byte(_) => k == "byte",
            Value::Char(_) => k == "char",
            Value::Text(_) => k == "text",
            Value::TextConst(..) => k == "textconstant",
            Value::Code(_) => k == "code",
            Value::Date(..) => k == "date",
            Value::Time(_) => k == "time",
            Value::DateTime(_) => k == "datetime",
            Value::Duration(_) => k == "duration",
            Value::Guid(_) => k == "guid",
            Value::DateFormula(_) => k == "dateformula",
            // An enum in a Variant reports IsOption: corpus types/TestVariantExtended.al:261.
            // A platform enum is its own type there, not an option (types/variant/LTP89IsEnumsA.al).
            Value::Option(_, members) => match crate::sysenum::kind_of(members) {
                Some(kind) => k == kind,
                None => k == "option",
            },
            Value::Enum(..) => k == "option",
            Value::List(_) => k == "list",
            Value::Dictionary(_) => k == "dictionary",
            Value::Array(_) => false,
            Value::Codeunit(_) => k == "codeunit",
            Value::Library(n, _) => n.eq_ignore_ascii_case(&k),
            Value::Record(_) => k == "record",
            Value::TestPage(_) | Value::TestControl(_) | Value::Query(_) => false,
            Value::Blob(_) => false,
            Value::DotNet(_) => k == "dotnet",
            // Never reached: a read of an unknown column is refused before its value reaches a Variant.
            Value::Unknown(_) => false,
            // The type of the slot that held it: an Integer as Blob.Length returns it, a BigInteger or Decimal field or variable
            // its own type. An Integer expression in a BigInteger variable answers BigInteger (types/decimal-integer/
            // Y66BigIntIntoInt.al:143-144, :163; types/decimal-integer/ScT70063.al:159-162, :196-197); a whole number of another
            // type in a Decimal variable answers Decimal (Y66BigIntIntoInt.al:139-140, :162).
            Value::PlaceholderSize(_, ty) => {
                k == match ty {
                    SqlType::Integer => "integer",
                    SqlType::BigInteger => "biginteger",
                    SqlType::Decimal => "decimal",
                }
            }
            // A BigInteger (session-system/session/SqlCounters.al:87-103).
            Value::SqlCount(c) => {
                k == match c.ty {
                    SqlType::Integer => "integer",
                    SqlType::BigInteger => "biginteger",
                    SqlType::Decimal => "decimal",
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        assert!(matches!(Value::default_for(&RtType::Integer), Value::Integer(0)));
        assert!(matches!(Value::default_for(&RtType::Boolean), Value::Boolean(false)));
        assert!(matches!(Value::default_for(&RtType::Text(Some(10))), Value::Text(ref s) if s.is_empty()));
        assert!(matches!(Value::default_for(&RtType::Code(None)), Value::Code(ref s) if s.is_empty()));
        assert!(matches!(Value::default_for(&RtType::Date), Value::Date(0, false)));
        assert!(matches!(Value::default_for(&RtType::Decimal), Value::Decimal(d) if d.is_zero()));
        assert!(matches!(Value::default_for(&RtType::Variant), Value::Null));
        assert!(matches!(
            Value::default_for(&RtType::Option(vec!["A".into()].into())),
            Value::Option(0, _)
        ));
        assert!(matches!(Value::default_for(&RtType::Guid), Value::Guid(g) if g == [0; 16]));
    }

    #[test]
    fn an_option_default_shares_the_members_of_its_type() {
        let members: std::sync::Arc<[String]> = vec!["A".to_string(), "B".to_string()].into();
        let ty = RtType::Option(members.clone());
        let Value::Option(0, held) = Value::default_for(&ty) else {
            panic!()
        };
        assert!(
            std::sync::Arc::ptr_eq(&held, &members),
            "the default copied the member names"
        );
    }

    #[test]
    fn arrays_default_to_their_element_default_per_cell() {
        let Value::Array(a) = Value::default_for(&RtType::Array(vec![2, 3], Box::new(RtType::Integer))) else {
            panic!()
        };
        assert_eq!(a.borrow().len(), 6);
        assert!(matches!(a.borrow()[5], Value::Integer(0)));
    }

    #[test]
    fn lists_share_on_clone_text_copies() {
        let l = Value::default_for(&RtType::List(Box::new(RtType::Integer)));
        let l2 = l.clone();
        if let Value::List(v) = &l {
            v.borrow_mut().push(Value::Integer(1));
        }
        let Value::List(v2) = &l2 else { panic!() };
        assert_eq!(v2.borrow().len(), 1, "a List is a reference type");
        let t = Value::Text("a".into());
        let mut t2 = t.clone();
        if let Value::Text(s) = &mut t2 {
            s.push('b');
        }
        assert!(matches!(t, Value::Text(ref s) if s == "a"), "Text is a value type");
    }

    #[test]
    fn each_default_list_is_a_fresh_handle() {
        let a = Value::default_for(&RtType::List(Box::new(RtType::Integer)));
        let b = Value::default_for(&RtType::List(Box::new(RtType::Integer)));
        let (Value::List(a), Value::List(b)) = (a, b) else {
            panic!()
        };
        assert!(!Rc::ptr_eq(&a, &b));
    }

    #[test]
    fn a_dropped_host_object_queues_its_handle_and_clones_share_it() {
        let queue = Rc::new(RefCell::new(Vec::new()));
        let a = Value::DotNet(Rc::new(DotNetObject::host(
            7,
            "System.IO.MemoryStream".into(),
            queue.clone(),
        )));
        let b = a.clone();
        drop(a);
        assert!(queue.borrow().is_empty(), "a second reference is alive");
        drop(b);
        assert_eq!(*queue.borrow(), vec![7]);
        // Enum and internal objects have nothing to release.
        drop(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Enum {
            clr_type: "E".into(),
            name: "A".into(),
            ordinal: 0,
        }))));
        assert_eq!(queue.borrow().len(), 1);
        assert!(matches!(Value::default_for(&RtType::DotNet(0)), Value::Null));
    }

    #[test]
    fn utf16_length() {
        assert_eq!(Value::text_len("abc"), 3);
        assert_eq!(Value::text_len("ä"), 1);
        assert_eq!(Value::text_len("😀"), 2, "a surrogate pair counts as two");
    }

    #[test]
    fn dynamic_type_tests() {
        assert!(Value::Integer(1).is("Integer"));
        assert!(!Value::Integer(1).is("Decimal"));
        assert!(Value::Code("A".into()).is("code"));
        assert!(!Value::Code("A".into()).is("Text"));
        assert!(Value::Option(1, std::sync::Arc::from(vec![])).is("Option"));
        assert!(Value::Enum(1, 0).is("Option"));
        assert!(!Value::Null.is("Integer"));
    }

    #[test]
    fn scalars_and_options_are_plain_and_handles_are_not() {
        let plain = [
            Value::Null,
            Value::Integer(1),
            Value::Text("a".into()),
            Value::Guid([1; 16]),
            Value::Enum(1, 2),
            Value::Option(1, std::sync::Arc::from(vec!["a".to_string()])),
        ];
        assert!(plain.iter().all(Value::is_plain));
        assert!(!Value::List(Default::default()).is_plain());
        assert!(!Value::Library("X".into(), Default::default()).is_plain());
    }
}

#[cfg(test)]
mod identical_tests {
    use super::*;
    use rust_decimal::Decimal;

    fn t(s: &str) -> Text {
        Text::new(s.to_string())
    }
    type Text = crate::text::Txt;

    #[test]
    fn same_contents_are_identical_and_different_are_not() {
        assert!(Value::Integer(1).identical(&Value::Integer(1)));
        assert!(!Value::Integer(1).identical(&Value::Integer(2)));
        assert!(!Value::Integer(1).identical(&Value::BigInteger(1)));
        assert!(Value::Null.identical(&Value::Null));
    }

    #[test]
    fn text_and_code_are_distinct() {
        assert!(Value::Text(t("A")).identical(&Value::Text(t("A"))));
        assert!(!Value::Text(t("A")).identical(&Value::Code(t("A"))));
        assert!(!Value::Text(t("A")).identical(&Value::Text(t("a"))));
    }

    #[test]
    fn enum_needs_id_and_ordinal() {
        assert!(Value::Enum(5, 1).identical(&Value::Enum(5, 1)));
        assert!(!Value::Enum(5, 1).identical(&Value::Enum(6, 1)));
        assert!(!Value::Enum(5, 1).identical(&Value::Enum(5, 2)));
    }

    #[test]
    fn decimals_by_exact_representation() {
        let z = Decimal::ZERO;
        let mut nz = Decimal::ZERO;
        nz.set_sign_negative(true);
        assert!(!Value::Decimal(z).identical(&Value::Decimal(nz)));
        assert!(!Value::Decimal(Decimal::new(10, 1)).identical(&Value::Decimal(Decimal::new(100, 2))));
        assert!(Value::Decimal(Decimal::new(10, 1)).identical(&Value::Decimal(Decimal::new(10, 1))));
    }

    #[test]
    fn blob_bytes_and_flags_are_compared() {
        let a = crate::stream::new_blob();
        let b = crate::stream::new_blob();
        assert!(Value::Blob(a.clone()).identical(&Value::Blob(b.clone())));
        crate::stream::write_bytes(&a, b"xy");
        assert!(!Value::Blob(a.clone()).identical(&Value::Blob(b.clone())));
        crate::stream::write_bytes(&b, b"xy");
        assert!(Value::Blob(a.clone()).identical(&Value::Blob(b.clone())));
        b.borrow_mut().loaded = false;
        assert!(!Value::Blob(a).identical(&Value::Blob(b)));
    }

    #[test]
    fn collections_compare_element_wise() {
        let l = |v: Vec<Value>| Value::List(Rc::new(RefCell::new(ListVec::from(v))));
        assert!(l(vec![Value::Integer(1)]).identical(&l(vec![Value::Integer(1)])));
        assert!(!l(vec![Value::Integer(1)]).identical(&l(vec![Value::Integer(2)])));
        assert!(!l(vec![Value::Integer(1)]).identical(&l(vec![])));
        let d = |v: Vec<(Value, Value)>| Value::Dictionary(Rc::new(RefCell::new(v)));
        let kv = |a: i32, b: i32| (Value::Integer(a), Value::Integer(b));
        assert!(d(vec![kv(1, 2)]).identical(&d(vec![kv(1, 2)])));
        assert!(!d(vec![kv(1, 2)]).identical(&d(vec![kv(1, 3)])));
        // A list nested in an array.
        let a = |v: Vec<Value>| Value::Array(Rc::new(RefCell::new(v)));
        assert!(a(vec![l(vec![Value::Byte(1)])]).identical(&a(vec![l(vec![Value::Byte(1)])])));
        assert!(!a(vec![l(vec![Value::Byte(1)])]).identical(&a(vec![l(vec![Value::Byte(2)])])));
    }

    #[test]
    fn rows_of_fields() {
        let x = vec![Value::Integer(1), Value::Text(t("a"))];
        assert!(Value::all_identical(&x, &x.clone()));
        assert!(!Value::all_identical(&x, &x[..1]));
    }
}

#[cfg(test)]
mod version_entry_tests {
    use super::*;

    /// The entry the variable's own Get gave it, with a failed Insert and a Delete of its own.
    fn own_entry(key: Vec<Value>) -> VersionEntry {
        let mut e = VersionEntry::own(VersionSource {
            version: 7,
            key: key.into(),
            how: VersionHow::Get,
            table_written: TableWritten::Written,
            company: Some("CRONUS".into()),
            tests: 1,
            commits: 2,
            rollbacks: 3,
            writes: [4, 5, 6],
            log: 8,
        });
        let h = e.0.as_mut().expect("an entry");
        (h.failed_inserts, h.deleted) = (1, true);
        e
    }

    /// Where the entry's key lives, and what the copy holding it has of its own: how it came, its failed Inserts and
    /// whether it deleted its row.
    fn key_at(e: &VersionEntry) -> *const Value {
        e.0.as_ref().expect("an entry").source.key.as_ptr()
    }
    fn per_copy(e: &VersionEntry) -> (VersionVia, u32, bool) {
        let s = e.0.as_ref().expect("an entry");
        (s.via, s.failed_inserts, s.deleted)
    }

    /// A copy of a record (a clone of its state, `:=`, Record.Copy, a by-value parameter, GetTable) shares the source of
    /// its row version instead of copying its key and company: a by-value Record argument made two copies of both per
    /// call. What a copy holds of its own is still its own: how it came, no failed Insert yet, and the Delete carried.
    #[test]
    fn a_copy_shares_the_row_version_source() {
        let own = own_entry(vec![Value::Integer(1), Value::Text("K".into())]);
        let cloned = own.clone();
        let by_value = own.carried(VersionVia::ByValue);
        assert_eq!(key_at(&cloned), key_at(&own));
        assert_eq!(key_at(&by_value), key_at(&own));
        assert_eq!(per_copy(&own), (VersionVia::Own, 1, true));
        assert_eq!(per_copy(&cloned), (VersionVia::Cloned, 0, true));
        assert_eq!(per_copy(&by_value), (VersionVia::ByValue, 0, true));
    }
}
