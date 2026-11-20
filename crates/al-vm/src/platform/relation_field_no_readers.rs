//! Readers of the Field table's RelationFieldNo (column 22) other than a Record variable, for a row whose relation names no
//! field (`field_relation::relation_columns` writes 0 there, pinned for Record reads by
//! metadata/platform-tables/FieldRelationFieldNo.al:161-260).
//!
//! The readers corpus PR 1477 measured (FieldRelationFieldNoReaders.al, not yet pinned) run under the assumption
//! `relation-field-no-readers`; every other reader of that 0 is refused. A temporary Field record holds what AL wrote into
//! it and keeps main's result, and so do a Record variable's field reads, filters and order.

use super::pos;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::value::{RecordState, Value};
use al_data::TableDef;
use std::cell::RefCell;
use std::rc::Rc;

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

/// A reader of RelationFieldNo other than a Record variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reader {
    /// `FieldRef.Value` of a RecordRef.
    FieldRefValue,
    /// A RecordRef read whose filters name RelationFieldNo.
    RecordRefFilter,
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
/// - `FieldRef.Value` after FindFirst, FindSet and Next, Get(RecordId) and GetTable (:148, :174, :198, :235);
/// - a RecordRef read filtered on RelationFieldNo by FieldRef.SetRange, FieldRef.SetFilter or a SetView WHERE (:283-325);
/// - a List page opened with OpenView: Value and AsInteger (:355, :380) and TestFilter.SetFilter (:412);
/// - a Card page opened with OpenView or caught by Trap from Page.Run(Page::X, FieldRec): AsInteger (:434-453).
pub(crate) fn relation_field_no_read(r: Reader) -> Result<(), String> {
    match r {
        Reader::FieldRefValue
        | Reader::RecordRefFilter
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
            Reader::FieldRefValue => "FieldRef.Value".into(),
            Reader::RecordRefFilter => "a RecordRef read filtered on it".into(),
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

impl Session {
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

    /// A RecordRef read of the database Field table: filters on RelationFieldNo mark the assumption, an order that has it
    /// is refused.
    pub(crate) fn relation_field_no_recordref_scan(&self, rec: &Rec) -> Result<(), Signal> {
        let Some(t) = self.database_field_table_of(rec) else {
            return Ok(());
        };
        let p = relation_field_no_pos(&t);
        let r = rec.borrow();
        if t.keys.get(r.key).is_some_and(|k| k.contains(&p)) {
            return relation_field_no_read(Reader::RecordRefOrder).map_err(Signal::unsupported);
        }
        if r.filters.iter().any(|f| f.field == p) {
            return relation_field_no_read(Reader::RecordRefFilter).map_err(Signal::unsupported);
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
