//! What a page background task's worker may do on a table whose filter group 1 entry of the test session it runs without
//! ([`Group1Filters::enter_worker`]): the calls the cells measured, by the form and setter of the entry as the cells had them
//! (assumption `bg-task-worker-filter-group-one`; corpus PR 1507, tests/al-language/pages/background-tasks/
//! BgTaskWorkerFilterGroup1.al, and corpus PR 1514, BgTaskWorkerFilterGroup1Shapes.al). Every other call on the table
//! refuses with main's wording ([`Session::group1_refusal`]): main read the test session's entry there.
//!
//! What a page background task's worker may do on a table whose filter group 1 entry of the test
//! session it runs without: by the entry's form and setter (`WorkerReads`, noted with the entry), the
//! calls the cells of corpus PR 1507 and PR 1514 (BgTaskWorkerFilterGroup1Shapes.al) measured; every
//! other call refuses with main's wording. Entry points: `Session::worker_call_measured` (by
//! `group1_use`), `worker_set_measured` (by `set_global_group_filter`).

use super::*;
use std::cmp::Ordering;

/// What a page background task's worker may do on a table whose test session entry it runs without, noted with the entry
/// ([`Session::worker_reads`]).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkerReads {
    /// No cell had the entry's form or setter: every call but FilterGroup refuses.
    None,
    /// The cells read the entry's form with a fresh Record only: its Count, then its GetFilters in filter group 1 (the
    /// `repro` mode of the worker G1W Reader, BgTaskWorkerFilterGroup1Shapes.al:323, :341, :359, :379, :403, :419, :440).
    Fresh,
    /// A Record variable's SetRange of one Integer value on Integer field `field` outside the primary key, the form of every
    /// other cell ([`worker_read_measured`], [`Session::worker_set_measured`]).
    Every { field: usize },
}

/// The one value of a SetRange(Field, Value) (`a..a`) or of a filter `=Value`.
fn single_value(e: &FilterExpr) -> Option<&Value> {
    match e {
        FilterExpr::Range(Some(a), Some(b)) => {
            (al_builtins::ops::compare(a, b).ok() == Some(Ordering::Equal)).then_some(a)
        }
        FilterExpr::Eq(v) => Some(v),
        _ => None,
    }
}

fn field_type(t: &TableDef, field: usize) -> Option<&RtType> {
    t.fields.get(field).map(|d| &d.ty)
}

/// SetFilter(Tag, '7|8') on the Integer field outside the primary key (BgTaskWorkerFilterGroup1Shapes.al:403-412).
fn two_integers_outside_key(t: &TableDef, f: &FieldFilter) -> bool {
    let two = matches!(&f.expr, FilterExpr::Or(v)
        if v.len() == 2 && v.iter().all(|e| matches!(single_value(e), Some(Value::Integer(_)))));
    two && outside_key(t, f.field) && field_type(t, f.field) == Some(&RtType::Integer)
}

/// SetRange("No.", B) of one Code value on the primary key, a single Code field (BgTaskWorkerFilterGroup1Shapes.al:419-433).
fn one_code_on_primary_key(t: &TableDef, f: &FieldFilter) -> bool {
    t.keys.first().is_some_and(|pk| pk[..] == [f.field])
        && matches!(field_type(t, f.field), Some(RtType::Code(_)))
        && matches!(single_value(&f.expr), Some(Value::Code(_)))
}

/// SetRange(Label, 'B') of one Text value on a Text field outside the primary key, next to SetRange(Tag, 7)
/// (BgTaskWorkerFilterGroup1Shapes.al:440-453).
fn one_text_outside_key(t: &TableDef, f: &FieldFilter) -> bool {
    outside_key(t, f.field)
        && matches!(field_type(t, f.field), Some(RtType::Text(_)))
        && matches!(single_value(&f.expr), Some(Value::Text(_)))
}

/// Whether the worker's call `op` (`args`) on a table whose entry it runs without is one the cells measured, while the worker
/// holds no group 1 entry of its own on the table: by `reads`, the kind of the `reader` and its view `r`.
fn worker_read_measured(
    reads: WorkerReads,
    reader: Option<VarKind>,
    op: Group1Op,
    args: &[Value],
    r: &RecordState,
) -> bool {
    use RecordOp::*;
    use VarKind::*;
    let fresh = reader_view_measured(r);
    let every = matches!(reads, WorkerReads::Every { .. });
    let own_group0 = !r.filters.is_empty() && r.filters.iter().all(|f| f.group == 0) && reader_order_measured(r);
    let find_arg = matches!(args.first(), Some(Value::Text(s) | Value::Code(s)) if *s == "-" || *s == ">");
    let Some(reader) = reader else {
        return false;
    };
    match (reader, op, r.filter_group, args.len()) {
        // The `repro` mode: Count, then GetFilters in filter group 1 (BgTaskWorkerFilterGroup1.al:322); Count of a record with a
        // filter of its own in filter group 0 (Shapes.al:460).
        (Record, Group1Op::Record(Count), 0, 0) => fresh || every && own_group0,
        (Record, Group1Op::Record(GetFilters), 1, 0) => fresh,
        _ if !every => false,
        // FindFirst (:366), FindLast (:379), IsEmpty (:392) and a FindSet loop (:405) of BgTaskWorkerFilterGroup1.al; Reset,
        // Clear, Init and GetFilters (Shapes.al:538); Insert (:551) and Modify (:563), which raise the read-only session's
        // error.
        (
            Record,
            Group1Op::Record(
                FindFirst | FindLast | IsEmpty | FindSet | Reset | Clear | Init | GetFilters | Insert | Modify,
            ),
            0,
            0,
        ) => fresh,
        // Next of a record that read a row: in a FindSet loop (BgTaskWorkerFilterGroup1.al:405) and after FindFirst
        // (Shapes.al:499).
        (Record, Group1Op::Record(Next), 0, 0) => fresh && r.position.is_some(),
        // Get (BgTaskWorkerFilterGroup1.al:418; Shapes.al:563), CalcSums (Shapes.al:512); GetFilter in filter group 1
        // (BgTaskWorkerFilterGroup1.al:444); the record's own SetRange(Field, From, To) in filter group 0 (Shapes.al:460).
        (Record, Group1Op::Record(Get | CalcSums), 0, 1)
        | (Record, Group1Op::Record(GetFilter), 1, 1)
        | (Record, Group1Op::Record(SetRange), 0, 3) => fresh,
        // Find('-') (Shapes.al:473) and Find('>') (:486).
        (Record, Group1Op::Record(Find), 0, 1) => fresh && find_arg,
        // A RecordRef: Count, GetFilters in filter group 1, then Close (BgTaskWorkerFilterGroup1.al:457).
        (RecordRef, Group1Op::RecordRef(m), g, 0) => {
            fresh && matches!((m, g), ("Count", 0) | ("GetFilters", 1) | ("Close", 1))
        }
        // A temporary record the worker fills: Insert, Count, GetFilters in filter group 1 (Shapes.al:525).
        (TemporaryRecord, Group1Op::Record(Insert | Count), 0, 0)
        | (TemporaryRecord, Group1Op::Record(GetFilters), 1, 0) => fresh,
        _ => false,
    }
}

impl Session {
    /// What a page background task's worker may do on the table of `rec` while it runs without the test session's entry
    /// `filters`, which `setter` set ([`WorkerReads`]).
    pub(super) fn worker_reads(&self, rec: &Rec, filters: &[FieldFilter], setter: Option<VarKind>) -> WorkerReads {
        let Ok(t) = self.table(rec) else {
            return WorkerReads::None;
        };
        let record = setter == Some(VarKind::Record);
        match filters {
            [f] if record && one_integer_outside_key(&t, f) => WorkerReads::Every { field: f.field },
            // A temporary record (Shapes.al:341) or a RecordRef (:359) set the one Integer value.
            [f] if setter.is_some() && one_integer_outside_key(&t, f) => WorkerReads::Fresh,
            [f] if record && (two_integers_outside_key(&t, f) || one_code_on_primary_key(&t, f)) => WorkerReads::Fresh,
            [a, b]
                if record
                    && (one_integer_outside_key(&t, a) && one_text_outside_key(&t, b)
                        || one_integer_outside_key(&t, b) && one_text_outside_key(&t, a)) =>
            {
                WorkerReads::Fresh
            }
            _ => WorkerReads::None,
        }
    }

    /// The worker's call `op` on table `table`: where the worker runs without the test session's entry of the table, it runs
    /// only as the cells measured it ([`WorkerReads`]); FilterGroup always runs, as on main. Once the worker set its own entry
    /// there (BgTaskWorkerFilterGroup1.al:489), the reads of that entry follow the in-session rules of
    /// [`Session::group1_use`].
    pub(super) fn worker_call_measured(
        &self,
        rec: &Rec,
        table: u32,
        op: Group1Op,
        args: &[Value],
    ) -> Result<(), Signal> {
        let Some(reads) = self.global_group1.kept_aside(table) else {
            return Ok(());
        };
        if op.is_filter_group() {
            return Ok(());
        }
        let measured = match reads {
            WorkerReads::None => false,
            WorkerReads::Every { .. } if self.global_group1.filters.by_table.contains_key(&table) => true,
            _ => {
                let r = rec.borrow();
                let reader = VarKind::of(matches!(op, Group1Op::Record(_)), r.temporary);
                worker_read_measured(reads, reader, op, args, &r)
            }
        };
        if measured {
            return Ok(());
        }
        self.group1_refusal(false, op)
    }

    /// The worker's SetFilter or SetRange in filter group 1 on table `table`, where it runs without the test session's entry
    /// there: only a Record variable's SetRange on the entry's field, of one Integer value (BgTaskWorkerFilterGroup1.al:489)
    /// or with no value (Shapes.al:685), while the record holds no group 1 filter and the worker no entry of its own there.
    pub(super) fn worker_set_measured(
        &self,
        rec: &Rec,
        table: u32,
        method: RecordOp,
        args: &[Value],
    ) -> Result<(), Signal> {
        let Some(reads) = self.global_group1.kept_aside(table) else {
            return Ok(());
        };
        let field = match args.first() {
            Some(Value::Integer(p)) => usize::try_from(*p).ok(),
            _ => None,
        };
        let shape = match args.len() {
            1 => true,
            2 => self.integer_value(rec, args),
            _ => false,
        };
        let fresh = {
            let r = rec.borrow();
            reader_view_measured(&r) && !r.temporary
        };
        let measured = matches!(reads, WorkerReads::Every { field: f } if field == Some(f))
            && method == RecordOp::SetRange
            && shape
            && fresh
            && self.is_al_record_call(method, rec)
            && !self.global_group1.filters.by_table.contains_key(&table);
        if measured {
            return Ok(());
        }
        self.group1_refusal(false, Group1Op::Record(method))
    }
}
