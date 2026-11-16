//! The xRec a new line of an AutoSplitKey ListPart hands to OnInsertRecord when SetValue moves the focus to a second control
//! and the line is inserted there: the row the line was started below, also when that row is the last row of the view
//! (tests/al-language/pages/parts/PartSplitKeyInsertTiming.al:530 Timing_Card_Two_DescrSeq, `INS 35000 x=20000` :552; :560
//! Timing_Card_Two_SeqDescr, :590 Timing_Card_Two_QtyDescr, :620 Timing_Card_Three; :899 Timing_CardPK_Two, `INS 50000
//! x=40000`, :929 Timing_CardPK_Three; :1175 TableTrig_Card_DescrSeq, :1205 TableTrig_Card_SeqDescr, :1262
//! TableTrig_Card_Three with table code). Only the measured shape takes it; every other shape keeps the earlier path (the
//! blank xRec after the last row, `insert_draft`).

use super::pinned_shape::{PartStep, PinnedPart, TestCall, code_stmts, table_code, text_log_routine, unconverted};
use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// The insert of a new line when the focus moves to a second control of it (`activate`). In the measured shape the
    /// line's OnInsertRecord sees the row it was started below as xRec. When the line stays new (OnInsertRecord answered
    /// false) or the insert raised, xRec is put back to what it held before, so a later insert of the same line starts from
    /// main's xRec. No measured shape gets there: the gate requires an OnInsertRecord that answers Yes and table code that
    /// raises nothing.
    pub(super) fn insert_at_second_control(&mut self, st: &St, setting_value: bool) -> Result<(), Signal> {
        let anchor = if setting_value {
            self.split_part_second_control_anchor(st)?
        } else {
            None
        };
        let before = anchor.map(|anchor| {
            let before = self.page_xrec_fields(st);
            self.set_xrec_fields(st, anchor);
            before
        });
        let inserted = self.insert_draft(st);
        if let Some(before) = before
            && (inserted.is_err() || st.borrow().draft)
        {
            self.set_xrec_fields(st, before);
        }
        inserted
    }

    /// The row the line was started below, when the line is the measured shape; `None` for every other shape. Every fact
    /// the probe measured is required:
    /// - the pages (`PAGES`, `part_pages_are`) and the link of `auto_split_part_pinned`: a Card host with no trigger, a
    ///   ListPart whose only properties beyond the plain ones are AutoSplitKey (no DelayedInsert, no SourceTableView) and that
    ///   declares OnInsertRecord, the one reader of this xRec; the host opened by the test's OpenEdit, the part a direct part
    ///   of it;
    /// - the part read ascending, in the primary key (:921, page 62728) or in a secondary key its SubPageView sets (:552,
    ///   page 62727);
    /// - the part's triggers log and OnInsertRecord answers Yes (`part_trigger_bodies_measured`);
    /// - the test's calls (`second_control_steps`);
    /// - the line started below the last row of a view the client holds whole (`part_line_started_at_view_end`; four rows);
    /// - nothing wrote to the store since `New()` started the line, and the line's number is still blank;
    /// - the part's table runs no code, or code of the measured kinds (`part_table_code_measured`).
    fn split_part_second_control_anchor(&mut self, st: &St) -> Result<Option<Vec<Value>>, Signal> {
        let Some(anchor) = st.borrow().after_new.clone() else {
            return Ok(None);
        };
        // The probe's pages: page 62724 (AutoSplitKey, no DelayedInsert, no SourceTableView, OnNewRecord and OnInsertRecord
        // that log) or 62726 (the same over table 62722) on host pages 62727, 62728 and 62732.
        const PAGES: PinnedPart = PinnedPart {
            host_type: ir::PageType::Card,
            host_triggers: &[],
            part_triggers: &["onnewrecord", "oninsertrecord"],
            part_properties: &["autosplitkey"],
            part_variable_controls: false,
            part_variable_validate: false,
            part_field_validate_update: false,
            part_expression_controls: false,
            unlinked: false,
            sibling_parts: false,
            user_filter: false,
        };
        let rec = self.page_rec(st);
        let (root, part) = Self::tree_root(st);
        let shape = part.is_some()
            && root.borrow().test_open_edit()
            && rec.borrow().ascending
            && self.has_auto_split_key(st)
            && self.is_list(st)
            && self.declares(st, "oninsertrecord")
            && self.part_pages_are(st, &PAGES)?
            && self.part_trigger_bodies_measured(st)
            && self.auto_split_part_pinned(st, &rec)?
            && self.second_control_steps(st, &rec)?
            && self.part_line_started_at_view_end(st, &anchor)? == Some(true)
            && st.borrow().new_line_writes == Some(self.write_mark())
            && self.new_line_number_blank(&rec)?
            && self.part_table_code_measured(&rec);
        Ok(shape.then_some(anchor))
    }

    /// The test's calls since the host opened, as the probe makes them (:552..:557): the host's `GoToKey`, the part's `Last`
    /// answering Yes, `New`, a value typed into one control and, running now, into another; both controls over fields of Rec
    /// outside the primary key; no call raised. Value reads in between are left out (`part_steps`).
    fn second_control_steps(&self, st: &St, rec: &Rec) -> Result<bool, Signal> {
        use PageMethod as M;
        use PartStep::{Host, Own};
        use TestCall::{Page, Write};
        let Some(steps) = Self::part_steps(st) else {
            return Ok(false);
        };
        let [
            Host(Page(M::GoToKey)),
            Own(Page(M::Last)),
            Own(Page(M::New)),
            Own(Write(a)),
            Own(Write(b)),
        ] = steps[..]
        else {
            return Ok(false);
        };
        let table = self.table(rec)?;
        let engine = self.engine();
        let page = page_of(&engine, st);
        let non_key = |i: usize| matches!(page.controls.get(i).map(|c| &c.binding), Some(ir::Binding::Field(pos)) if !table.keys[0].contains(pos));
        let (root, _) = Self::tree_root(st);
        let none_failed = root.borrow().tree_calls.iter().all(|c| !c.failed);
        // `activate` inserts only when the focus moves, so the two controls are two different ones.
        Ok(non_key(a) && non_key(b) && none_failed)
    }

    /// Every trigger the part declares has the probe's body (`measured_part_trigger_body`).
    fn part_trigger_bodies_measured(&self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        page_of(&engine, st)
            .triggers
            .iter()
            .all(|(name, r)| measured_part_trigger_body(&engine.program, object, name, *r))
    }

    /// The last primary key field of the line (its Integer number) still holds 0: nothing numbered the line yet.
    fn new_line_number_blank(&self, rec: &Rec) -> Result<bool, Signal> {
        let table = self.table(rec)?;
        let Some(&pos) = table.keys[0].last() else {
            return Ok(false);
        };
        Ok(matches!(rec.borrow().fields.get(pos), Some(Value::Integer(0))))
    }

    /// The code the part's table can run is none, or of the kinds tables 62721 and 62722 of the probe have (:33, :60): a table
    /// OnInsert and field OnValidate triggers on fields outside the primary key, each a log call
    /// (`Log.Add('T-I ' + Format("Line No."))`), then at most one write to a field of Rec outside the primary key of a constant
    /// or of that field plus a constant (`Qty := 7`, `Qty := Qty + 1`, `Descr := Descr + '+ins'`). No other trigger or
    /// procedure, no table extension, no subscriber.
    fn part_table_code_measured(&self, rec: &Rec) -> bool {
        let engine = self.engine();
        let t = rec.borrow().table as usize;
        let program = &engine.program;
        let o = &program.objects[t];
        let (Some(table), Some(rec_slot)) = (o.table.as_ref(), o.rec_slot) else {
            return false;
        };
        let key = &table.keys[0];
        table_code(&engine, t).is_some()
            && o.routines.iter().enumerate().all(|(r, routine)| {
                let kind = routine.is_trigger
                    && match routine.field {
                        None => routine.name.eq_ignore_ascii_case("OnInsert"),
                        Some(f) => routine.name.eq_ignore_ascii_case("OnValidate") && !key.contains(&f),
                    };
                kind && measured_table_trigger_body(program, t, r, rec_slot, key)
            })
    }
}

/// Statement `s` of routine `routine` is a log call (`Log.Add('T-I ' + Format("Line No."))`): a procedure that only appends
/// text (`text_log_routine`) of a codeunit in a local variable, not as a try call, with one argument built with `+` of text
/// constants and `Format(x)` of a value `formatted` accepts.
fn log_call(program: &ir::Program, routine: &ir::Routine, s: &ir::Stmt, formatted: &dyn Fn(&ir::Expr) -> bool) -> bool {
    fn text(e: &ir::Expr, formatted: &dyn Fn(&ir::Expr) -> bool) -> bool {
        match unconverted(e) {
            ir::Expr::Const(ir::Const::Text(_)) => true,
            ir::Expr::Binary(ir::BinOp::Add, a, b) => text(a, formatted) && text(b, formatted),
            ir::Expr::Builtin {
                id,
                receiver: None,
                args,
                ..
            } => {
                *id == al_builtins::builtin_id::ids::SYSTEM_FORMAT_1
                    && matches!(args.as_slice(), [ir::Arg::Value(v)] if formatted(unconverted(v)))
            }
            _ => false,
        }
    }
    let ir::Stmt::Expr(ir::Expr::Call {
        object,
        routine: callee,
        instance: Some(instance),
        args,
        try_call: false,
    }) = s
    else {
        return false;
    };
    matches!(&**instance, ir::Expr::Load(ir::Place::Local(l))
        if routine.locals.get(*l).is_some_and(|s| s.ty == RtType::Codeunit(*object as u32)))
        && text_log_routine(program, *object, *callee)
        && matches!(args.as_slice(), [ir::Arg::Value(v)] if text(v, formatted))
}

/// `e` reads a field of the record in global `slot`.
fn field_of(e: &ir::Expr, slot: usize) -> Option<usize> {
    match e {
        ir::Expr::Load(ir::Place::Field(base, f)) if matches!(**base, ir::Place::Global(g) if g == slot) => Some(*f),
        _ => None,
    }
}

/// The body of the part's trigger `name` (routine `r` of page `page`) is the probe's (page 62724, :153, :160): OnNewRecord a
/// log call, OnInsertRecord a log call then `exit(true)`; the log text formats fields of Rec and xRec and the trigger's
/// parameter.
fn measured_part_trigger_body(program: &ir::Program, page: usize, name: &str, r: usize) -> bool {
    let o = &program.objects[page];
    let (Some(routine), Some(rec), Some(xrec)) = (o.routines.get(r), o.rec_slot, o.xrec_slot) else {
        return false;
    };
    let formatted = |e: &ir::Expr| {
        field_of(e, rec).is_some()
            || field_of(e, xrec).is_some()
            || matches!(e, ir::Expr::Load(ir::Place::Local(l)) if *l < routine.params)
    };
    let log = |s: &ir::Stmt| log_call(program, routine, s, &formatted);
    let exits_true = |e: &ir::Expr| matches!(unconverted(e), ir::Expr::Const(ir::Const::Boolean(true)));
    let mut stmts = code_stmts(program, page, r);
    // The lowering ends a trigger with a return value by an `exit` of its default, unreachable after an `exit`.
    if let [.., ir::Stmt::Exit(Some(_)), ir::Stmt::Exit(Some(_))] = stmts.as_slice() {
        stmts.pop();
    }
    match (name, stmts.as_slice()) {
        ("onnewrecord", [s]) => log(s),
        ("oninsertrecord", [s, ir::Stmt::Exit(Some(e))]) => log(s) && exits_true(e),
        _ => false,
    }
}

/// The body of trigger `r` of table `t` (Rec in global `rec`, primary key `key`) is a log call of fields of Rec, then at most
/// one write of a constant or of the field itself plus a constant to a field of Rec outside the primary key (see
/// `part_table_code_measured`).
fn measured_table_trigger_body(program: &ir::Program, t: usize, r: usize, rec: usize, key: &[usize]) -> bool {
    let Some(routine) = program.objects[t].routines.get(r) else {
        return false;
    };
    let formatted = |e: &ir::Expr| field_of(e, rec).is_some();
    let write = |s: &ir::Stmt| {
        let ir::Stmt::Assign(ir::Place::Field(base, f), v) = s else {
            return false;
        };
        let value = match unconverted(v) {
            ir::Expr::Const(_) => true,
            ir::Expr::Binary(ir::BinOp::Add, a, b) => {
                field_of(unconverted(a), rec) == Some(*f) && matches!(unconverted(b), ir::Expr::Const(_))
            }
            _ => false,
        };
        matches!(**base, ir::Place::Global(g) if g == rec) && !key.contains(f) && value
    };
    match code_stmts(program, t, r).as_slice() {
        [log] => log_call(program, routine, log, &formatted),
        [log, w] => log_call(program, routine, log, &formatted) && write(w),
        _ => false,
    }
}
