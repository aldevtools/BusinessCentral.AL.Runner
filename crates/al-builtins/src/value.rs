//! Runtime values (ADR-010).

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
    /// A Label (or TextConst) held by a Variant: the text, answering IsTextConstant and no IsText (types/
    /// Y81LabelVariant.al; needs corpus bump). Only a Variant ever holds it; anything that takes it out reads a Text.
    TextConst(crate::text::Txt),
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
    /// The byte size of a placeholder report output (`Blob.Length`, `File.Len`), with the placeholder's mark: an Integer known only
    /// to be greater than 0, so only a comparison with 0 is answered (`crate::placeholder_output::size`).
    PlaceholderSize(std::sync::Arc<str>),
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

/// The number type a counter value has (`SqlCount::ty`).
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
}

impl Drop for TempToken {
    fn drop(&mut self) {
        self.release.borrow_mut().push(self.id);
    }
}

impl TempId {
    pub fn new(id: u64, release: Rc<RefCell<Vec<u64>>>) -> TempId {
        TempId(Rc::new(TempToken { id, release }))
    }

    pub fn id(&self) -> u64 {
        self.0.id
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
    /// The table is `TableType = CRM`: any use needs a registered table connection
    /// (session-system/TestCrmTableNoConnectionContracts.al).
    pub crm: bool,
    /// Field values in the table's field order.
    pub fields: Vec<Value>,
    pub filters: Vec<crate::filter::FieldFilter>,
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
    pub position: Option<Vec<Value>>,
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
    /// The variable's own Insert and its own Get of that row after it (see [`OwnRead`]).
    pub own_read: OwnRead,
    /// The record is a source the Blob copy probes measured (see [`ProbedSource`]).
    pub probed_source: ProbedSource,
    /// The table's and its extensions' global variables of this record variable (ADR-021).
    pub globals: TableGlobals,
    /// Every use of the record refuses with this reason: what its buffer holds is not pinned (the row a page closed on after
    /// `Page.RunModal(Id, Rec)` called as a statement, see `hand_back_selection`).
    pub unpinned_buffer: Option<&'static str>,
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

/// See [`RecordState::row_origin`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

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

    /// A handle onto `xRec`'s set (created on first use).
    pub fn xrec(&self) -> TableGlobals {
        let data = self.data();
        let d = data.borrow();
        d.xrec.share()
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
    /// (session-system/TestEnvironmentUserTenantContracts.al:195-201); the other properties are refused.
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

/// State behind a library-type handle; grows with the builtins that need it.
#[derive(Debug, Default)]
pub enum LibraryValue {
    #[default]
    Empty,
    /// An InStream or OutStream after `Clear`: it holds no stream, as one never created, but the corpus measured fewer of its
    /// uses (streams/instream-outstream/Y62GoneInStream.al against StreamUncreatedUse.al; `stream_cleared`).
    ClearedStream,
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
    /// An OutStream: appends to a Blob buffer.
    OutStream(crate::stream::BlobRef, crate::stream::Encoding),
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
            RtType::Codeunit(id) => Value::Codeunit(Rc::new(RefCell::new(Instance {
                object: *id,
                ..Default::default()
            }))),
            // An unassigned interface holds nothing.
            RtType::Interface(_) => Value::Null,
            RtType::TestPage(id) | RtType::Page(id) => Value::TestPage(PageHandle::closed(*id)),
            RtType::Query(id) => Value::Query(Rc::new(RefCell::new(QueryState {
                object: *id,
                filters: Vec::new(),
                filter_replaced: false,
                top: None,
                security_filtering: None,
                run: None,
                instance: None,
                in_before_open: false,
                closed: false,
            }))),
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
            Value::TextConst(_) => k == "textconstant",
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
            // An Integer, as Blob.Length is.
            Value::PlaceholderSize(_) => k == "integer",
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
