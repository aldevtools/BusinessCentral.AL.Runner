//! Readers of the Field table's RelationFieldNo (column 22) other than a Record variable, for a row whose relation names no
//! field (`field_relation::relation_columns` writes 0 there, pinned for Record reads by
//! metadata/platform-tables/FieldRelationFieldNo.al:161-260).
//!
//! The readers corpus PR 1477 measured (FieldRelationFieldNoReaders.al, not yet pinned) run under the assumption
//! `relation-field-no-readers`; every other reader of that 0 is refused. A temporary Field record keeps what AL wrote into
//! it; a Record variable's reads are not touched here (pinned by FieldRelationFieldNo.al:161-260).
//!
//! the readers of that RelationFieldNo 0 other than a Record variable. `relation_field_no_read` is
//! the allowlist of the readers corpus PR 1477 measured (assumption `relation-field-no-readers`);
//! every other reader refuses. `RecordRefTraces` (a `Session` field) keeps how each RecordRef buffer
//! over the database Field table reached its row and which filters on RelationFieldNo
//! FieldRef.SetRange, FieldRef.SetFilter and SetView set, fed from `recordref_op`. Called from
//! FieldRef.Value (`refs.rs`), RecordRef finds (`recordref_record_op`), Format (also
//! `Format(FieldRef, n, 1 | 2 | 9)` through `Ctx::format_fieldref_read`), TestField and FieldError
//! (`recmeta.rs`), CalcSums (`flowfields.rs`), page controls, TestFilter and the page view at open
//! (`pages/`); `queries.rs` raises the measured Query.Open error.

use super::pos;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::record_op::RecordOp;
use al_builtins::value::{LibraryValue, RecordState, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::{Session, Signal};

type Rec = Rc<RefCell<RecordState>>;

/// The platform Field table.
pub(crate) const FIELD_TABLE: u32 = 2_000_000_041;
/// The Field table's TableNo, No., RelationTableNo and RelationFieldNo.
const NO: u32 = 2;
const RELATION_TABLE_NO: u32 = 21;
const RELATION_FIELD_NO: u32 = 22;
/// The first number of the platform's system fields (SystemId, SystemCreatedBy, ...).
const SYSTEM_FIELD_BASE: i32 = 2_000_000_000;

/// How a page over the Field table was opened, as the probe's pages were.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageOpen {
    /// A page of PageType = List opened with `TestPage.OpenView` (:355, :380, :412).
    ListOpenView,
    /// A page of PageType = Card opened with `TestPage.OpenView` (:434-441).
    CardOpenView,
    /// A page of PageType = Card that `TestPage.Trap` caught from `Page.Run(Page::X, FieldRec)` (:446-450).
    CardTrappedStaticRunWithRecord,
    /// Any other page type or open.
    Other,
}

/// How a page control is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControlRead {
    /// `TestField.Value`.
    Value,
    /// `TestField.AsInteger`.
    AsInteger,
    /// Any other read (AssertEquals, a lookup, a search, ...).
    Other,
}

/// How a RecordRef over the database Field table reached its current row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reach {
    /// `FindFirst()` after `Open(Database::Field)` (:148, through RefRow :134-137).
    FindFirst,
    /// `FindSet()` after `Open(Database::Field)`, then `Next()` while it moves (:235, :242-247).
    FindSetNext,
    /// `Get(RecordId)` after `Open(Database::Field)` (:198, :208-209).
    GetRecordId,
    /// `GetTable(FieldRec)` of a database Field record of the session's company into a RecordRef never opened (:198, :205-206).
    GetTable,
    /// Any other way, or the row changed since.
    Other,
}

/// The RecordRef read that runs filtered on RelationFieldNo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanOp {
    Count,
    FindSet,
    Next,
    /// Find, FindFirst, FindLast, IsEmpty, CountApprox and the rest.
    Other,
}

/// Where the filters on RelationFieldNo of a RecordRef came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterSource {
    /// Every one is in filter group 0, set by `FieldRef.SetRange(0)`, `FieldRef.SetFilter('<>0')` or a SetView WHERE with
    /// `CONST(0)` on that RecordRef (:283-325).
    MeasuredOnRecordRef,
    /// Another form, filter group or source (a Record's filter carried by GetTable, Copy, ...).
    Other,
}

/// A reader of RelationFieldNo other than a Record variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reader {
    /// `FieldRef.Value` of a RecordRef that reached its row this way.
    FieldRefValue(Reach),
    /// A RecordRef read whose filters name RelationFieldNo.
    RecordRefFilter(ScanOp, FilterSource),
    /// A RecordRef read in an order that has RelationFieldNo in it.
    RecordRefOrder,
    /// `Format` of a Record, RecordRef or FieldRef that shows RelationFieldNo.
    Format,
    /// `TestField` or `FieldError` on RelationFieldNo.
    TestFieldOrFieldError,
    /// `CalcSums` or `FieldRef.CalcSum` over RelationFieldNo.
    CalcSum,
    /// A page control bound to RelationFieldNo.
    PageControl(PageOpen, ControlRead),
    /// `TestFilter.SetFilter` on RelationFieldNo.
    PageFilter(PageOpen),
    /// A page whose open left its view filtered or ordered on RelationFieldNo.
    PageView,
}

/// The readers FieldRelationFieldNoReaders.al measured (corpus PR 1477) mark the assumption; every other one is refused.
/// - `FieldRef.Value` of a row reached as [`Reach`] names, unchanged since (:148, :174, :198, :235);
/// - a RecordRef Count, FindSet or Next whose filters on RelationFieldNo are all [`FilterSource::MeasuredOnRecordRef`]
///   (:283-325);
/// - a List page opened with OpenView: Value and AsInteger (:355, :380) and TestFilter.SetFilter (:412);
/// - a Card page opened with OpenView or caught by Trap from Page.Run(Page::X, FieldRec): AsInteger (:434-453).
pub(crate) fn relation_field_no_read(r: Reader) -> Result<(), String> {
    match r {
        Reader::FieldRefValue(Reach::FindFirst | Reach::FindSetNext | Reach::GetRecordId | Reach::GetTable)
        | Reader::RecordRefFilter(ScanOp::Count | ScanOp::FindSet | ScanOp::Next, FilterSource::MeasuredOnRecordRef)
        | Reader::PageControl(PageOpen::ListOpenView, ControlRead::Value | ControlRead::AsInteger)
        | Reader::PageControl(
            PageOpen::CardOpenView | PageOpen::CardTrappedStaticRunWithRecord,
            ControlRead::AsInteger,
        )
        | Reader::PageFilter(PageOpen::ListOpenView) => {
            assume(Assumption::RelationFieldNoReaders);
            Ok(())
        }
        other => Err(format!(
            "{} of the Field table's RelationFieldNo is not measured where a TableRelation names no field (docs/probes-needed.md, relation-field-no-readers)",
            other.describe()
        )),
    }
}

impl Reader {
    /// The reader in words, for the refusal.
    fn describe(self) -> String {
        let open = |o: PageOpen| match o {
            PageOpen::ListOpenView => "a List page opened with OpenView",
            PageOpen::CardOpenView => "a Card page opened with OpenView",
            PageOpen::CardTrappedStaticRunWithRecord => "a Card page trapped from Page.Run",
            PageOpen::Other => "a page of another type or open",
        };
        match self {
            Reader::FieldRefValue(Reach::Other) => {
                "FieldRef.Value after a positioning other than FindFirst, FindSet and Next, Get(RecordId) or GetTable".into()
            }
            Reader::FieldRefValue(_) => "FieldRef.Value".into(),
            Reader::RecordRefFilter(ScanOp::Other, _) => {
                "a RecordRef read other than Count, FindSet and Next filtered on it".into()
            }
            Reader::RecordRefFilter(_, FilterSource::Other) => {
                "a RecordRef read filtered on it other than by FieldRef.SetRange(0), FieldRef.SetFilter('<>0') or SetView in filter group 0".into()
            }
            Reader::RecordRefFilter(..) => "a RecordRef read filtered on it".into(),
            Reader::RecordRefOrder => "a RecordRef read sorted on it".into(),
            Reader::Format => "Format".into(),
            Reader::TestFieldOrFieldError => "TestField or FieldError".into(),
            Reader::CalcSum => "CalcSums".into(),
            Reader::PageControl(o, read) => {
                let read = match read {
                    ControlRead::Value => "Value",
                    ControlRead::AsInteger => "AsInteger",
                    ControlRead::Other => "a read other than Value or AsInteger",
                };
                format!("{read} of a control on {}", open(o))
            }
            Reader::PageFilter(o) => format!("TestFilter.SetFilter on {}", open(o)),
            Reader::PageView => "a page view filtered or sorted on it".into(),
        }
    }
}

/// Whether table `object_id` of a record (`temporary`) is the database Field table.
pub(crate) fn database_field_table(object_id: Option<u32>, temporary: bool) -> bool {
    object_id == Some(FIELD_TABLE) && !temporary
}

/// The position of RelationFieldNo in the Field table `t`.
pub(crate) fn relation_field_no_pos(t: &TableDef) -> usize {
    pos(t, RELATION_FIELD_NO)
}

/// Whether Field row `row` holds the RelationFieldNo 0 that `relation_columns` writes for a relation that names a table and
/// no field: RelationTableNo a table number, RelationFieldNo 0, No. a field below the system fields (SystemCreatedBy and
/// SystemModifiedBy relate to the User table's field 0 too, pinned by record/TestPlatformMetadataDeclarationContracts.al:240-241).
pub(crate) fn no_field_zero(t: &TableDef, row: &[Value]) -> bool {
    matches!(row[pos(t, RELATION_TABLE_NO)], Value::Integer(n) if n > 0)
        && matches!(row[pos(t, RELATION_FIELD_NO)], Value::Integer(0))
        && matches!(row[pos(t, NO)], Value::Integer(n) if (1..SYSTEM_FIELD_BASE).contains(&n))
}

/// A filter on RelationFieldNo in a form the probe set: `SetRange(0)`, `SetFilter('<>0')`, `CONST(0)` (:283-325).
fn measured_filter_form(e: &FilterExpr) -> bool {
    matches!(
        e,
        FilterExpr::Range(Some(Value::Integer(0)), Some(Value::Integer(0)))
            | FilterExpr::Eq(Value::Integer(0))
            | FilterExpr::Ne(Value::Integer(0))
    )
}

/// Whether a cell of a Field row still holds what it held: a column the platform leaves unknown holds the same reason.
fn same_cell(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Unknown(x), Value::Unknown(y)) => x == y,
        _ => a.identical(b),
    }
}

/// What the run saw of each RecordRef buffer over the database Field table: how it reached its row, and the filters on
/// RelationFieldNo that FieldRef.SetRange, FieldRef.SetFilter and SetView set on it. A buffer copied into another (Duplicate,
/// GetTable, Copy) starts with nothing.
#[derive(Default)]
pub(crate) struct RecordRefTraces(Vec<Trace>);

struct Trace {
    buffer: Weak<RefCell<RecordState>>,
    /// The buffer was opened by `Open(Database::Field)` with no other argument (:134, :208, :242), or came from a measured
    /// GetTable.
    plain_open: bool,
    /// How the buffer reached its row, and the row as it was then.
    reach: Option<(Reach, Vec<Value>)>,
    /// The filters on RelationFieldNo the measured entry points set, as they set them.
    filters: Vec<FieldFilter>,
}

impl RecordRefTraces {
    /// No buffer the run traced is alive any more: what a new session holds, as far as code can see.
    pub(crate) fn as_new(&self) -> bool {
        self.0.iter().all(|t| t.buffer.strong_count() == 0)
    }

    fn get(&self, rec: &Rec) -> Option<&Trace> {
        self.0.iter().find(|t| std::ptr::eq(t.buffer.as_ptr(), Rc::as_ptr(rec)))
    }

    fn entry(&mut self, rec: &Rec) -> &mut Trace {
        self.0.retain(|t| t.buffer.strong_count() > 0);
        let i = match self
            .0
            .iter()
            .position(|t| std::ptr::eq(t.buffer.as_ptr(), Rc::as_ptr(rec)))
        {
            Some(i) => i,
            None => {
                self.0.push(Trace {
                    buffer: Rc::downgrade(rec),
                    plain_open: false,
                    reach: None,
                    filters: Vec::new(),
                });
                self.0.len() - 1
            }
        };
        &mut self.0[i]
    }
}

/// What a RecordRef method does to the buffer's trace.
pub(crate) enum TraceStep {
    /// `Open`: with the table as its one argument (`true`) or with more.
    Open(bool),
    /// Reaches a row this way when it finds one.
    Reach(Reach),
    /// Leaves the row where it is (`Field`).
    Keep,
    /// Forgets how the row was reached.
    Forget,
    /// Forgets the row and the filters: Reset, Copy and GetTable write the filters.
    ForgetAll,
}

impl Session {
    /// What RecordRef method `method` with `args` does to the trace of the RecordRef `lib`, before it runs.
    pub(crate) fn relation_field_no_trace_step(&self, method: &str, lib: &LibraryValue, args: &[Value]) -> TraceStep {
        let opened = match lib {
            LibraryValue::RecordRef(rec) => Some(rec),
            _ => None,
        };
        let record_id = matches!(args, [Value::Library(n, _)] if n.eq_ignore_ascii_case("RecordId"));
        let field_record = |src: &Rec| {
            let s = src.borrow();
            database_field_table(self.engine.program.objects[s.table as usize].id, s.temporary)
                && s.company.is_none()
                && !s.change_company_called
        };
        match (method, opened) {
            ("Open", _) => TraceStep::Open(args.len() == 1),
            ("Field", _) => TraceStep::Keep,
            ("FindFirst", Some(_)) if args.is_empty() => TraceStep::Reach(Reach::FindFirst),
            ("FindSet", Some(_)) if args.is_empty() => TraceStep::Reach(Reach::FindSetNext),
            ("Next", Some(rec)) if args.is_empty() && self.relation_field_no_reach(rec) == Reach::FindSetNext => {
                TraceStep::Reach(Reach::FindSetNext)
            }
            ("Get", Some(_)) if record_id => TraceStep::Reach(Reach::GetRecordId),
            ("GetTable", None)
                if matches!(lib, LibraryValue::Empty)
                    && !self.variant_refs.get_table
                    && matches!(args, [Value::Record(src)] if field_record(src)) =>
            {
                TraceStep::Reach(Reach::GetTable)
            }
            ("Reset" | "Copy" | "GetTable", _) => TraceStep::ForgetAll,
            _ => TraceStep::Forget,
        }
    }

    /// After RecordRef method `method` ran on `lib` with result `r`: records how the buffer reached its row (`step`).
    pub(crate) fn relation_field_no_after_recordref_op(
        &mut self,
        step: TraceStep,
        method: &str,
        lib: &LibraryValue,
        r: &Result<Value, Signal>,
    ) {
        let LibraryValue::RecordRef(rec) = lib else {
            return;
        };
        let moved = match (method, r) {
            ("Next", Ok(Value::Integer(n))) => *n != 0,
            ("GetTable", Ok(_)) => true,
            (_, Ok(Value::Boolean(b))) => *b,
            _ => false,
        };
        match step {
            TraceStep::Keep => {}
            TraceStep::Open(plain) => {
                if r.is_ok() && self.session_company_field_ref(rec) {
                    let t = self.relation_field_no_refs.entry(rec);
                    *t = Trace {
                        buffer: Rc::downgrade(rec),
                        plain_open: plain,
                        reach: None,
                        filters: Vec::new(),
                    };
                }
            }
            TraceStep::Reach(Reach::GetTable) if moved && self.session_company_field_ref(rec) => {
                let row = rec.borrow().fields.to_vec();
                let t = self.relation_field_no_refs.entry(rec);
                t.plain_open = true;
                t.reach = Some((Reach::GetTable, row));
            }
            TraceStep::Reach(reach)
                if moved
                    && self.session_company_field_ref(rec)
                    && self.relation_field_no_refs.get(rec).is_some_and(|t| t.plain_open) =>
            {
                let row = rec.borrow().fields.to_vec();
                self.relation_field_no_refs.entry(rec).reach = Some((reach, row));
            }
            step => {
                if self.relation_field_no_refs.get(rec).is_some() {
                    let t = self.relation_field_no_refs.entry(rec);
                    t.reach = None;
                    if matches!(step, TraceStep::ForgetAll) {
                        t.filters.clear();
                    }
                }
            }
        }
    }

    /// Whether `rec` is a RecordRef over the database Field table of the session's company.
    fn session_company_field_ref(&self, rec: &Rec) -> bool {
        let r = rec.borrow();
        r.company.is_none()
            && !r.change_company_called
            && database_field_table(self.engine.program.objects[r.table as usize].id, r.temporary)
    }

    /// How RecordRef buffer `rec` reached the row it holds: `Other` once the row changed.
    fn relation_field_no_reach(&self, rec: &Rec) -> Reach {
        let r = rec.borrow();
        match self.relation_field_no_refs.get(rec).and_then(|t| t.reach.as_ref()) {
            Some((reach, row))
                if row.len() == r.fields.len() && row.iter().zip(&r.fields).all(|(a, b)| same_cell(a, b)) =>
            {
                *reach
            }
            _ => Reach::Other,
        }
    }

    /// `FieldRef.SetRange` or `FieldRef.SetFilter` of the field at `pos` (`None`: a SetView) ran on RecordRef buffer `rec`:
    /// keeps the filters on RelationFieldNo of the current filter group it left.
    pub(crate) fn relation_field_no_filter_set(&mut self, rec: &Rec, pos: Option<usize>) {
        let Some(t) = self.database_field_table_of(rec) else {
            return;
        };
        let p = relation_field_no_pos(&t);
        if pos.is_some_and(|x| x != p) {
            return;
        }
        let r = rec.borrow();
        let group = r.filter_group;
        let set: Vec<FieldFilter> = r
            .filters
            .iter()
            .filter(|f| f.field == p && f.group == group)
            .cloned()
            .collect();
        drop(r);
        let trace = self.relation_field_no_refs.entry(rec);
        trace.filters.retain(|f| f.group != group);
        trace.filters.extend(set);
    }

    /// The database Field table of `rec`, when it is one.
    fn database_field_table_of(&self, rec: &Rec) -> Option<std::sync::Arc<TableDef>> {
        let r = rec.borrow();
        let id = self.engine.program.objects[r.table as usize].id;
        if !database_field_table(id, r.temporary) {
            return None;
        }
        drop(r);
        self.table(rec).ok()
    }

    /// Reader `reader` reads the field at `pos` of `rec`'s current row, or every field (`None`): decided by
    /// [`relation_field_no_read`] when that reads the no-field RelationFieldNo 0 of the database Field table.
    pub(crate) fn relation_field_no_row_read(
        &self,
        rec: &Rec,
        pos: Option<usize>,
        reader: Reader,
    ) -> Result<(), Signal> {
        let Some(t) = self.database_field_table_of(rec) else {
            return Ok(());
        };
        let p = relation_field_no_pos(&t);
        if pos.is_none_or(|x| x == p) && no_field_zero(&t, &rec.borrow().fields) {
            return relation_field_no_read(reader).map_err(Signal::unsupported);
        }
        Ok(())
    }

    /// `FieldRef.Value` of the field at `pos` of RecordRef buffer `rec`: decided by [`relation_field_no_read`] with how the
    /// buffer reached its row, when that reads the no-field RelationFieldNo 0 of the database Field table.
    pub(crate) fn relation_field_no_fieldref_value(&self, rec: &Rec, pos: usize) -> Result<(), Signal> {
        let Some(t) = self.database_field_table_of(rec) else {
            return Ok(());
        };
        if pos == relation_field_no_pos(&t) && no_field_zero(&t, &rec.borrow().fields) {
            return relation_field_no_read(Reader::FieldRefValue(self.relation_field_no_reach(rec)))
                .map_err(Signal::unsupported);
        }
        Ok(())
    }

    /// RecordRef read `op` of the database Field table: a read filtered on RelationFieldNo is decided by
    /// [`relation_field_no_read`] with the read and where its filters came from; an order that has it is refused.
    pub(crate) fn relation_field_no_recordref_scan(&self, rec: &Rec, op: RecordOp) -> Result<(), Signal> {
        let Some(t) = self.database_field_table_of(rec) else {
            return Ok(());
        };
        let p = relation_field_no_pos(&t);
        let r = rec.borrow();
        if t.keys.get(r.key).is_some_and(|k| k.contains(&p)) {
            return relation_field_no_read(Reader::RecordRefOrder).map_err(Signal::unsupported);
        }
        if r.filters.iter().any(|f| f.field == p) {
            let set = self.relation_field_no_refs.get(rec).map_or(&[][..], |t| &t.filters[..]);
            let measured = r.filters.iter().filter(|f| f.field == p).all(|f| {
                f.group == 0
                    && measured_filter_form(&f.expr)
                    && set.iter().any(|s| s.group == f.group && s.expr.identical(&f.expr))
            });
            let source = if measured {
                FilterSource::MeasuredOnRecordRef
            } else {
                FilterSource::Other
            };
            let op = match op {
                RecordOp::Count => ScanOp::Count,
                RecordOp::FindSet => ScanOp::FindSet,
                RecordOp::Next => ScanOp::Next,
                _ => ScanOp::Other,
            };
            return relation_field_no_read(Reader::RecordRefFilter(op, source)).map_err(Signal::unsupported);
        }
        Ok(())
    }

    /// `CalcSums` or `FieldRef.CalcSum` of `positions` over the database Field table: refused when RelationFieldNo is
    /// among them.
    pub(crate) fn relation_field_no_calc_sum(&self, rec: &Rec, positions: &[usize]) -> Result<(), Signal> {
        match self.database_field_table_of(rec) {
            Some(t) if positions.contains(&relation_field_no_pos(&t)) => {
                relation_field_no_read(Reader::CalcSum).map_err(Signal::unsupported)
            }
            _ => Ok(()),
        }
    }

    /// The view of page record `rec` after its open: refused when it filters or orders the database Field table on
    /// RelationFieldNo.
    pub(crate) fn relation_field_no_page_view(&self, rec: &Rec) -> Result<(), Signal> {
        let Some(t) = self.database_field_table_of(rec) else {
            return Ok(());
        };
        let p = relation_field_no_pos(&t);
        let r = rec.borrow();
        if r.filters.iter().any(|f| f.field == p) || t.keys.get(r.key).is_some_and(|k| k.contains(&p)) {
            return relation_field_no_read(Reader::PageView).map_err(Signal::unsupported);
        }
        Ok(())
    }

    /// `TestFilter.SetFilter` on the field at `pos` of page record `rec`, opened as `open`.
    pub(crate) fn relation_field_no_page_filter(&self, rec: &Rec, pos: usize, open: PageOpen) -> Result<(), Signal> {
        match self.database_field_table_of(rec) {
            Some(t) if pos == relation_field_no_pos(&t) => {
                relation_field_no_read(Reader::PageFilter(open)).map_err(Signal::unsupported)
            }
            _ => Ok(()),
        }
    }
}
