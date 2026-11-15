//! The writes of a value the SQL column of a database table cannot hold (`sqltime::bad_field`): which statement raises the
//! platform's error, and when. The cells of Insert, Modify, ModifyAll and Rename that corpus PR 1398
//! (tests/al-language/record/filters/SqlDateFloor1753.al, the corpus CI legs of the majors 27, 28 and 29) measured run
//! through the assumption `sql-date-first-year` until that probe is pinned; every other write of such a value is refused.
//! The Insert of the values record/filters/P130DateFilters.al:255-293 pins stays a clean pass.

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::{RtType, Value};
use al_data::{Store, TableDef};

use super::Rec;
use crate::Session;
use crate::globaltriggers::GlobalOp;
use crate::sqltime::{BadKind, BadValue, InsertEvidence};
use crate::state::WriteOfAl;

/// What a failed Insert and a failed Modify put back on the buffer: its row version and the stamped fields.
pub(super) type Stamps = (Option<(usize, Value)>, crate::hostvalues::Replaced);

/// The facts of an Insert the gates of `insert_sql_range` take.
pub(super) struct InsertShape {
    /// The table has an AutoIncrement field.
    pub(super) auto: bool,
    /// The Insert runs the table's triggers.
    pub(super) run: bool,
    /// The Insert statement of AL code on a Record variable (`WriteOfAl::Insert`).
    pub(super) of_al: bool,
}

/// Who calls a Modify (`TxState::write_of_al`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ModifyCaller {
    /// The Modify statement of AL code on a Record variable, any form: Modify(), Modify(true), Modify(false), its result used.
    Statement,
    /// ModifyAll(.., true) of AL code on a Record variable, for one row.
    ModifyAllRow,
    /// RecordRef.Modify, a page save, ModifyAll(.., false) row by row, the platform's own Modify.
    Other,
}

/// The tables of SqlDateFloor1753.al by the code a write runs and the fields they have: the write cells name these.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TableCode {
    /// No trigger, table extension trigger, event subscriber or database trigger for the write, and no Blob field
    /// (`LT SD Floor Row`, the key tables).
    Plain,
    /// The table's own trigger of the write (OnInsert, OnModify) with the measured body (`own_trigger_measured`) and nothing
    /// else, no Blob field (`LT SD Floor Trig`).
    OwnTrigger,
    /// One static subscriber of the write's Before event and one of its After event, each with the measured log body
    /// (`one_log_subscriber`), and nothing else, no Blob field (`LT SD Floor Sub`).
    Subscribers,
    /// No code for the write, and a Blob field (`LT SD Floor Blob`).
    PlainWithBlob,
    /// The table's own trigger of the write with the measured log body (`own_trigger_logs`) and nothing else, no Blob field
    /// (`LT SD Floor Trig Log`).
    LogTrigger,
    /// The table's own trigger of the write with the measured body that sets a Date field (`own_trigger_sets_date`) and
    /// nothing else, no Blob field (`LT SD Floor Trig SetDt`): the field and the day of the date the `then` branch sets.
    DateSetTrigger { field: usize, day: i32 },
    /// One static subscriber of the write's After event, with the measured body that logs a Date field of Rec
    /// (`one_reader_subscriber`), and nothing else, no Blob field (`LT SD Floor Sub Read`).
    AfterReader,
    /// Any other table.
    Other,
}

/// The table's own trigger of a write, by its body (`records/sql_range_code.rs`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum OwnTrigger {
    None,
    /// `Hits += 10` (`own_trigger_measured`).
    Adds,
    /// `Log.Add('<text>' + Format(<Date field>, 0, 9))` (`own_trigger_logs`).
    Logs,
    /// `if F = <date> then F := <date in 1753> else F := <date>` (`own_trigger_sets_date`).
    SetsDate {
        field: usize,
        day: i32,
    },
    Other,
}

/// The subscribers of a write's Before and After events, by their number and bodies.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Subscribers {
    None,
    /// One static subscriber of each event, each with the log body (`one_log_subscriber`).
    BeforeAndAfterLog,
    /// No Before subscriber, one static After subscriber that logs a Date field of Rec (`one_reader_subscriber`).
    AfterReader,
    Other,
}

/// Which write a table's code is read for.
#[derive(Clone, Copy)]
enum Write {
    Insert,
    Modify,
    Rename,
}

impl Write {
    fn names(self) -> (&'static str, [&'static str; 3], [&'static str; 2], GlobalOp) {
        match self {
            Write::Insert => (
                "OnInsert",
                ["OnBeforeInsert", "OnInsert", "OnAfterInsert"],
                ["onbeforeinsertevent", "onafterinsertevent"],
                GlobalOp::Insert,
            ),
            Write::Modify => (
                "OnModify",
                ["OnBeforeModify", "OnModify", "OnAfterModify"],
                ["onbeforemodifyevent", "onaftermodifyevent"],
                GlobalOp::Modify,
            ),
            Write::Rename => (
                "OnRename",
                ["OnBeforeRename", "OnRename", "OnAfterRename"],
                ["onbeforerenameevent", "onafterrenameevent"],
                GlobalOp::Rename,
            ),
        }
    }
}

impl Session {
    /// Who calls the Modify starting now; takes its mark (`TxState::write_of_al`).
    pub(super) fn take_modify_caller(&mut self) -> ModifyCaller {
        if self.take_write_of_al(WriteOfAl::Modify) {
            ModifyCaller::Statement
        } else if self.take_write_of_al(WriteOfAl::ModifyAllRow) {
            ModifyCaller::ModifyAllRow
        } else {
            ModifyCaller::Other
        }
    }

    /// The code a write of `rec` into `t` runs, as one of the measured tables (`TableCode`).
    fn table_code(&mut self, rec: &Rec, t: &TableDef, write: Write) -> Result<TableCode, Signal> {
        let (own, ext, [before_event, after_event], global) = write.names();
        let own = match self.table_trigger(t, own) {
            None => OwnTrigger::None,
            Some(r) if self.own_trigger_measured(t, r) => OwnTrigger::Adds,
            Some(r) if self.own_trigger_logs(t, r) => OwnTrigger::Logs,
            Some(r) => match self.own_trigger_sets_date(t, r) {
                Some((field, day)) => OwnTrigger::SetsDate { field, day },
                None => OwnTrigger::Other,
            },
        };
        let ext = ext.iter().any(|n| !self.ext_triggers(t, n).is_empty());
        let subs = match (
            self.listened(t, before_event, None)?,
            self.listened(t, after_event, None)?,
        ) {
            (false, false) => Subscribers::None,
            (true, true) if self.one_log_subscriber(t, before_event)? && self.one_log_subscriber(t, after_event)? => {
                Subscribers::BeforeAndAfterLog
            }
            (false, true) if self.one_reader_subscriber(t, after_event)? => Subscribers::AfterReader,
            _ => Subscribers::Other,
        };
        let global = self.global_enabled(t, global)?;
        let user = self.is_user_row(t, rec.borrow().temporary);
        let blob = self.has_blob_field(t);
        Ok(match (own, subs, ext, global, user, blob) {
            (OwnTrigger::None, Subscribers::None, false, false, false, false) => TableCode::Plain,
            (OwnTrigger::Adds, Subscribers::None, false, false, false, false) => TableCode::OwnTrigger,
            (OwnTrigger::Logs, Subscribers::None, false, false, false, false) => TableCode::LogTrigger,
            (OwnTrigger::SetsDate { field, day }, Subscribers::None, false, false, false, false) => {
                TableCode::DateSetTrigger { field, day }
            }
            (OwnTrigger::None, Subscribers::BeforeAndAfterLog, false, false, false, false) => TableCode::Subscribers,
            (OwnTrigger::None, Subscribers::AfterReader, false, false, false, false) => TableCode::AfterReader,
            (OwnTrigger::None, Subscribers::None, false, false, false, true) => TableCode::PlainWithBlob,
            _ => TableCode::Other,
        })
    }

    /// The facts every measured write shares: a database record that is not a CRM table's, outside the page client.
    fn sql_range_write_measured(&self, rec: &Rec) -> bool {
        let r = rec.borrow();
        matches!((r.temporary, r.crm, self.tx.immediate_errors), (false, false, false))
    }

    /// An Insert of a value its SQL column cannot hold.
    ///
    /// The Insert does not fail, the next access of the table does (record/filters/P130DateFilters.al:255-275, stage `get`),
    /// as for a duplicate key: main's shape, without a Blob or AutoIncrement field, the result not used, no after-insert
    /// table extension trigger or subscriber; a value only corpus PR 1398 measured (`InsertEvidence::ProbeHour`,
    /// `ProbePlain`, `ProbeOutside`) only by the Insert statement into the tables it measured. Assumed
    /// (`sql-date-first-year`, corpus PR 1398 SqlDateFloor1753.al Subscriber_Date_1, Subscriber_DateTime_1): with the
    /// measured Before and After insert subscribers and Insert(true), the After event still fires, then the next access
    /// fails; Extra_SubscriberReadsDate: so does the one After subscriber that logs the Date field, and it sees the value.
    /// Assumed (`blob_insert_measured`): an Insert into a table with a Blob field and no insert code raises the error at the
    /// Insert. A CRM table has no SQL column: what its table connection does with such a value is not pinned. Every other
    /// shape is refused.
    pub(super) fn insert_sql_range(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        bad: BadValue,
        shape: InsertShape,
        (old_version, stamped): Stamps,
    ) -> Result<Value, Signal> {
        let code = self.table_code(rec, t, Write::Insert)?;
        let base = self.sql_range_write_measured(rec) && !self.ret_used && !shape.auto;
        // The Blob and subscriber cells measured a Date in 1753 (1753-06-15) and a DateTime in 1753 that stays in its minute
        // (1753-06-15 03:00): not a closing date, not 23:59:59.999.
        let cell_value = matches!(bad.insert, InsertEvidence::Reporter | InsertEvidence::ProbeHour);
        let blob_cell = base
            && shape.of_al
            && code == TableCode::PlainWithBlob
            && cell_value
            && blob_insert_measured(shape.run, Self::blobs_empty(rec), bad.kind);
        // The cells below insert a key no row has (Outside_Date_Insert, Outside_DateTime_Insert, Extra_SubscriberReadsDate, the
        // Blob cells); a stored key adds the duplicate-key error, whose order against the value's error is not measured.
        let keyed_cell = blob_cell
            || (base && shape.of_al && code == TableCode::AfterReader)
            || (base && bad.insert == InsertEvidence::ProbeOutside && shape.of_al && code == TableCode::Plain);
        let key_free = keyed_cell && !self.key_stored(rec, t)?;
        if blob_cell && key_free {
            assume(Assumption::SqlDateFirstYear);
            Self::restore_row_version(rec, old_version);
            Self::unstamp(rec, stamped);
            return Err(Signal::Error(Box::new(bad.error)));
        }
        // Subscriber_Date_1, Subscriber_DateTime_1: Insert(true), log `beforeInsert;afterInsert;`, the error at the next access.
        // Extra_SubscriberReadsDate: Insert(true) of the Date 1753-06-15, log `afterInsertRead:1753-06-15;`, the error at the
        // next access.
        let after_event = base
            && shape.of_al
            && shape.run
            && cell_value
            && (code == TableCode::Subscribers
                || (code == TableCode::AfterReader && bad.kind == BadKind::Date1753 && key_free));
        // Main's shape. A value only corpus PR 1398 measured runs only by the Insert statement into the tables it measured
        // (`InsertEvidence`).
        let value_measured = match bad.insert {
            InsertEvidence::Pinned | InsertEvidence::Reporter => true,
            InsertEvidence::ProbeHour => {
                shape.of_al && (code == TableCode::Plain || (code == TableCode::OwnTrigger && shape.run))
            }
            InsertEvidence::ProbePlain => shape.of_al && code == TableCode::Plain,
            InsertEvidence::ProbeOutside => shape.of_al && !shape.run && code == TableCode::Plain && key_free,
        };
        let main_shape = base
            && value_measured
            && !self.has_blob_field(t)
            && (!shape.run || self.ext_triggers(t, "OnAfterInsert").is_empty())
            && !self.listened(t, "onafterinsertevent", None)?;
        if !(after_event || main_shape) {
            return unsupported(format!(
                "an Insert into {} of a value its SQL column cannot hold, in a form the corpus does not pin",
                t.name
            ));
        }
        if after_event
            || matches!(
                bad.insert,
                InsertEvidence::Reporter
                    | InsertEvidence::ProbeHour
                    | InsertEvidence::ProbePlain
                    | InsertEvidence::ProbeOutside
            )
        {
            assume(Assumption::SqlDateFirstYear);
        }
        Self::restore_row_version(rec, old_version);
        Self::unstamp(rec, stamped);
        self.defer_insert(rec, t.object, bad.error);
        if after_event {
            self.table_event(t, "onafterinsertevent", rec, None, true, None)?;
        }
        Ok(Value::Boolean(true))
    }

    /// A Modify of a row with a value its SQL column cannot hold. Assumed (`sql-date-first-year`, corpus PR 1398
    /// SqlDateFloor1753.al): the Modify raises the platform's error and writes nothing, after the Before event and the table's
    /// OnModify trigger and without the After event (Subscriber_Date_1: log `beforeModify;`; Extra_TriggerLog: log
    /// `trigModify:1753-06-15;`; RowAfterError_1: the row keeps its values; Kept_RecordAfterModifyError: the record keeps
    /// what the trigger set), also when its result is used (`if not Modify()` raises, it does not answer false: Modify_Date_Forms_1),
    /// in the measured cells of `modify_cell_measured`. Every other Modify of such a value is refused: RecordRef.Modify, a
    /// page save, ModifyAll(.., false) row by row, a row that does not exist (the corpus pins only the Insert:
    /// record/filters/P130DateFilters.al:234-275).
    pub(super) fn fail_modify_sql_range(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        bad: BadValue,
        (caller, row_exists, run): (ModifyCaller, bool, bool),
        (old_version, stamped): Stamps,
    ) -> Result<Value, Signal> {
        // `row_exists`: the row is stored; `run`: Modify(true).
        Self::restore_row_version(rec, old_version);
        Self::unstamp(rec, stamped);
        let code = self.table_code(rec, t, Write::Modify)?;
        let form = ModifyForm {
            caller,
            run,
            used: self.ret_used,
        };
        let measured = self.sql_range_write_measured(rec)
            && row_exists
            && modify_cell_measured(form, code, bad.kind)
            && self.trigger_branch_ran(rec, code, &bad);
        if !measured {
            return unsupported("a Modify of a value its SQL column cannot hold, in a form not measured (not pinned)");
        }
        assume(Assumption::SqlDateFirstYear);
        Err(Signal::Error(Box::new(bad.error)))
    }

    /// ModifyAll(field, value) of AL code on a Record variable that does not run the triggers, of a table whose modify events no
    /// subscriber declares (`bulk`, one statement for every row): a value the column cannot hold raises the platform's error
    /// at the ModifyAll. Assumed (`sql-date-first-year`, corpus PR 1398 SqlDateFloor1753.al ModifyAll_Forms_1,
    /// ModifyAll_Forms_2, Trigger_ModifyAll_1 `trig modifyall`, Extra_ModifyAllNoRow; RowAfterError_2: the rows keep their
    /// values) in the cells of `modify_all_cell_measured`, for a field the table declares itself. Any other ModifyAll of AL
    /// code of such a value over no row is refused: main raised nothing there, and the measured ModifyAll(.., false) over no
    /// row raises, so whether the other forms raise is not known. With rows it keeps main's path: row by row through Modify.
    pub(crate) fn modify_all_sql_range(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        pos: usize,
        value: &Value,
        (of_al, bulk, no_row): (bool, bool, bool),
    ) -> Result<(), Signal> {
        let date_field = matches!(t.fields[pos].ty, RtType::Date | RtType::DateTime);
        if !(of_al && date_field && self.sql_range_write_measured(rec)) {
            return Ok(());
        }
        let caption = self.caption(t);
        let Some(bad) = crate::sqltime::bad_field(t, pos, value, &|i| self.field_caption(t, i), &caption)? else {
            return Ok(());
        };
        let own_field = self.engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .and_then(|s| s.fields.get(pos))
            .is_some_and(|f| f.declared_in == Some(t.object));
        let code = self.table_code(rec, t, Write::Modify)?;
        if bulk && own_field && modify_all_cell_measured(code, bad.kind) {
            assume(Assumption::SqlDateFirstYear);
            return Err(Signal::Error(Box::new(bad.error)));
        }
        if no_row {
            return unsupported(
                "ModifyAll over no row of a value its SQL column cannot hold, in a form not measured (not pinned)",
            );
        }
        Ok(())
    }

    /// A Rename to a key value its SQL column cannot hold. Assumed (`sql-date-first-year`, corpus PR 1398 SqlDateFloor1753.al
    /// Rename_DateKey_1, Rename_DateKey_2, Rename_DateKey_NextAccess_1, Rename_DateTimeKey_1): the Rename raises the platform's
    /// error and the row keeps its key, for a Date in 1753, the Date 1752-12-31 and a DateTime in 1753, in a table whose
    /// primary key is that one field and that has no rename code, Blob field or related field, by the Rename statement of
    /// AL code on a Record variable, its result not used, of a stored row whose fields outside the key the record holds as
    /// stored (the cells read the row with Get, then Rename). The record takes back its values from before the Rename, as for a
    /// unique key the Rename breaks (what it holds is not measured). Assumed (Extra_RenameAbsentKey_1, _2): when no row has
    /// the key the record holds (the cells assigned it, read nothing), the Rename to a Date or a DateTime in 1753 raises the
    /// RecordNotFound of a missing row first, as a Rename to a value the column holds does. Every other Rename of such a value
    /// is refused.
    pub(super) fn fail_rename_sql_range(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        bad: BadValue,
        (rename_of_al, unrelated): (bool, bool),
        old: &[Value],
    ) -> Result<Value, Signal> {
        // `rename_of_al`: a Rename AL code calls (`TxState::rename_from_al`), here on a Record variable (not a RecordRef);
        // `unrelated`: no field of another table relates to the table (`rename_references` found none).
        let shape = self.sql_range_write_measured(rec)
            && rename_of_al
            && !self.through_recordref
            && unrelated
            && !self.ret_used
            && t.keys[0] == [bad.field]
            && self.table_code(rec, t, Write::Rename)? == TableCode::Plain;
        if !shape {
            return unsupported("a Rename with a value its SQL column cannot hold (not pinned)");
        }
        let stored = self.row_under_key(rec, t, old)?;
        let measured = match (&stored, bad.kind) {
            (Some(row), BadKind::Date1753 | BadKind::Date1752 | BadKind::DateTime1753) => Self::holds_row(t, old, row),
            (None, BadKind::Date1753 | BadKind::DateTime1753) => {
                assume(Assumption::SqlDateFirstYear);
                rec.borrow_mut().fields = old.to_vec();
                return self.fail(t, old, false);
            }
            _ => false,
        };
        if !measured {
            return unsupported("a Rename with a value its SQL column cannot hold (not pinned)");
        }
        assume(Assumption::SqlDateFirstYear);
        rec.borrow_mut().fields = old.to_vec();
        Err(Signal::Error(Box::new(bad.error)))
    }

    /// For the table whose OnModify trigger sets a Date field (`TableCode::DateSetTrigger`, Extra_TriggerSetsDate
    /// `ts-sets-1753`): the bad value is that field's and the record holds the date the `then` branch sets, so that branch
    /// ran. Every other table: true.
    fn trigger_branch_ran(&self, rec: &Rec, code: TableCode, bad: &BadValue) -> bool {
        match code {
            TableCode::DateSetTrigger { field, day } => {
                bad.field == field && matches!(rec.borrow().fields.get(field), Some(Value::Date(d, false)) if *d == day)
            }
            _ => true,
        }
    }

    /// Every Blob field of `rec` is empty.
    fn blobs_empty(rec: &Rec) -> bool {
        rec.borrow().fields.iter().all(|v| match v {
            Value::Blob(b) => b.borrow().bytes.is_empty(),
            _ => true,
        })
    }

    /// A row of `t` has the primary key `rec` holds.
    fn key_stored(&mut self, rec: &Rec, t: &TableDef) -> Result<bool, Signal> {
        let fields = rec.borrow().fields.clone();
        Ok(self.row_under_key(rec, t, &fields)?.is_some())
    }

    /// The row of `t` stored under the primary key of `fields`.
    fn row_under_key(&mut self, rec: &Rec, t: &TableDef, fields: &[Value]) -> Result<Option<Vec<Value>>, Signal> {
        let cx = self.cx_for(rec, t)?;
        let key = Self::sql_key(rec, t, fields);
        let row = self
            .store_for(rec)?
            .get(&cx, t, &key)
            .map_err(|e| Signal::unsupported(format!("{e}")))?;
        Ok(row.map(|r| r.to_vec()))
    }

    /// Each stored field of `row` outside the primary key equals the one in `fields`.
    fn holds_row(t: &TableDef, fields: &[Value], row: &[Value]) -> bool {
        let compared = |i: &usize| t.fields[*i].stored && !t.keys[0].contains(i);
        (0..t.fields.len()).filter(compared).all(|i| {
            matches!(
                (fields.get(i), row.get(i)),
                (Some(a), Some(b)) if al_builtins::ops::compare(a, b).is_ok_and(|o| o.is_eq())
            )
        })
    }
}

/// The form of a Modify: who calls it, whether it runs the triggers (Modify(true)), whether its result is used.
#[derive(Clone, Copy)]
struct ModifyForm {
    caller: ModifyCaller,
    run: bool,
    used: bool,
}

/// The Modify cells SqlDateFloor1753.al measured (corpus PR 1398): form, table and kind of value.
/// - The statement in any form, `LT SD Floor Row`: Dates and DateTimes in 1753, the DateTime 1752-12-31 12:00
///   (Modify_Date_1753, Modify_DateTime_MorningHours_1 to Modify_DateTime_RoundingEdge_2, Modify_Date_Forms_1,
///   ValidateThenModify_1).
/// - The statement in any form, `LT SD Floor Trig`, `LT SD Floor Sub` and `LT SD Floor Blob`: Dates and DateTimes in 1753
///   (Trigger_Date_1, Trigger_DateTime_1, Subscriber_Date_1, Subscriber_DateTime_1 and _2, Blob_Date_1, Blob_DateTime_1).
/// - A row of ModifyAll(.., true), `LT SD Floor Row`, `Trig` and `Sub`: a Date in 1753 (ModifyAll_Forms_1, Trigger_ModifyAll_1).
/// - Modify() (or Modify(false), the same call) with its result not used, `LT SD Floor Row`: a closing date in 1753 (Modify_Date_ClosingAndOutside_1 and _2),
///   the Date 1752-12-31 (_3, Outside_Date_Modify), a Date and a DateTime from 1700-01-01 to 1752-12-31 (Outside_Date_Modify,
///   Outside_DateTime_Modify).
/// - Modify(true) and Modify(false) with the result not used, `LT SD Floor Trig Log`: a Date in 1753 (Extra_TriggerLog).
/// - Modify(true) with the result not used, `LT SD Floor Trig SetDt`, the trigger setting a Date in 1753
///   (Extra_TriggerSetsDate; the branch that ran: `trigger_branch_ran`).
fn modify_cell_measured(form: ModifyForm, code: TableCode, kind: BadKind) -> bool {
    use BadKind::{ClosingDate1753, Date1752, Date1753, DatePre1753, DateTime1752, DateTime1753, DateTimePre1753};
    use ModifyCaller::{ModifyAllRow, Statement};
    use TableCode::{DateSetTrigger, LogTrigger, OwnTrigger, Plain, PlainWithBlob, Subscribers};
    let ModifyForm { caller, run, used } = form;
    matches!(
        (caller, code, kind),
        (Statement, Plain, Date1753 | DateTime1753 | DateTime1752)
            | (
                Statement,
                OwnTrigger | Subscribers | PlainWithBlob,
                Date1753 | DateTime1753
            )
            | (ModifyAllRow, Plain | OwnTrigger | Subscribers, Date1753)
    ) || matches!(
        (caller, code, kind, run, used),
        (
            Statement,
            Plain,
            ClosingDate1753 | Date1752 | DatePre1753 | DateTimePre1753,
            false,
            false
        ) | (Statement, LogTrigger, Date1753, _, false)
            | (Statement, DateSetTrigger { .. }, Date1753, true, false)
    )
}

/// The Insert cells of `LT SD Floor Blob` (a key no row has, no insert code), by Insert(true), whether every Blob field is
/// empty, and the kind of value: Insert() with empty Blobs, a Date and a DateTime in 1753 (Blob_Date_1, Blob_DateTime_1,
/// Blob_DateTime_2); Insert() with content in the Blob, a Date in 1753 (Extra_BlobContent); Insert(true) with empty Blobs, a
/// Date and a DateTime in 1753 (Extra_BlobInsertTrue).
fn blob_insert_measured(run: bool, blobs_empty: bool, kind: BadKind) -> bool {
    use BadKind::{Date1753, DateTime1753};
    matches!(
        (run, blobs_empty, kind),
        (false, true, Date1753 | DateTime1753) | (false, false, Date1753) | (true, true, Date1753 | DateTime1753)
    )
}

/// The ModifyAll(.., false) cells SqlDateFloor1753.al measured, by table and value, each with and without rows in the filter:
/// `LT SD Floor Row` with a Date in 1753 (ModifyAll_Forms_1, ModifyAll_Forms_2) and with a DateTime in 1753
/// (ModifyAll_Forms_1, Extra_ModifyAllNoRow), `LT SD Floor Trig` with a Date in 1753 (Trigger_ModifyAll_1,
/// Extra_ModifyAllNoRow).
fn modify_all_cell_measured(code: TableCode, kind: BadKind) -> bool {
    matches!(
        (code, kind),
        (TableCode::Plain, BadKind::Date1753 | BadKind::DateTime1753) | (TableCode::OwnTrigger, BadKind::Date1753)
    )
}
