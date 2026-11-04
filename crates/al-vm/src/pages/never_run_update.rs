//! `CurrPage.Update` and `SaveRecord` of a Page variable the client does not show (never ran, or its modal page closed): they do
//! nothing (see the arm in `pageops.rs` that calls [`Session::never_run_update`] for the pins), except on a new record its code
//! filled, which `Update(true)` and `Update()` insert.

use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageOp;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Never ran, `Update(true)` or `Update()` after the page's code set fields of a new record of its source table: BC inserts
    /// the record, through OnInsertRecord and the table's OnInsert, numbered by AutoSplitKey (corpus PR 1133, merged on master,
    /// format-evaluate/tablefilter-neutral/TableFilterEnumForms.al:419 UpdateNeverRunPage_NewRecord_UpdateTrue, :443
    /// _UpdateNoArgument; issue 6951; the Base Application "Table Filter" page fills its lines this way). Update(false) inserts
    /// nothing (:431 _UpdateFalse). With the blank record it does nothing (pages/currpage/UnopenedCurrPage.al
    /// Direct_Part_UpdateTrue), and so does `SaveRecord`.
    pub(super) fn never_run_update(&mut self, method: PageOp, st: &St, args: &[Value]) -> Result<Value, Signal> {
        let saves = method != PageOp::PageSaveRecord && !matches!(args.first(), Some(Value::Boolean(false)));
        if saves && !st.borrow().ran && st.borrow().instance.is_some() && self.has_source(st) {
            let rec = self.page_rec(st);
            let table = rec.borrow().table as usize;
            let fresh = self.initial_fields(table);
            // A stored record handed over with SetRecord is no new record (UnopenedCurrPage.al:299
            // Update_Then_RunModal_Card_HandlerReads `update-true` = `start end`).
            if !Self::same_values(&rec.borrow().fields, &fresh) && !self.row_exists(&rec)? {
                self.never_run_insert(st, &rec, &fresh)?;
            }
        }
        Ok(Value::Null)
    }

    /// The insert of `never_run_update`. Only the pinned shape runs: a page with AutoSplitKey whose new line has no number yet,
    /// the first line on an empty view (10000, BelowxRec No) or the next one after the line inserted before it, which is the
    /// last of the view (that line + 10000, BelowxRec Yes): TableFilterEnumForms.al:424-426 `[P.OnInsertRecord:10000/1 below=No
    /// | T.OnInsert:10000/1]`, `[P.OnInsertRecord:20000/4 below=Yes | T.OnInsert:20000/4]`.
    fn never_run_insert(&mut self, st: &St, rec: &Rec, fresh: &[Value]) -> Result<(), Signal> {
        assume(Assumption::NeverRunUpdateInsertsNewRecord);
        let refused = || {
            unsupported(
                "CurrPage.Update(true) or Update() of a page variable that never ran, over a new record its code filled, in a shape \
                 the corpus does not pin (no AutoSplitKey, a numbered line, or not after the last line)",
            )
        };
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let split = engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("AutoSplitKey") && is_true(v));
        let t = self.table(rec)?;
        let Some((&last, lead)) = t.keys[0].split_last() else {
            return refused();
        };
        if !split || !matches!(rec.borrow().fields[last], Value::Integer(0)) {
            return refused();
        }
        let xrec = self.page_xrec_fields(st);
        let below = !Self::same_values(&xrec, fresh);
        let after = if below {
            let same_lead = lead
                .iter()
                .all(|&k| ops::compare(&xrec[k], &rec.borrow().fields[k]).is_ok_and(|o| o.is_eq()));
            match &xrec[last] {
                Value::Integer(n) if same_lead => Some(*n),
                _ => return refused(),
            }
        } else {
            None
        };
        // Nothing may follow the line the new one goes after (or, for the first, be in the view at all).
        let probe = Rc::new(RefCell::new(rec.borrow().clone()));
        {
            let fields = rec.borrow().fields.clone();
            let mut p = probe.borrow_mut();
            p.filters.clear();
            p.marked_only = false;
            for &k in lead {
                p.filters.push(al_builtins::filter::FieldFilter {
                    field: k,
                    expr: al_builtins::filter::FilterExpr::Eq(fields[k].clone()),
                    group: 0,
                });
            }
            if let Some(n) = after {
                p.filters.push(al_builtins::filter::FieldFilter {
                    field: last,
                    expr: al_builtins::filter::FilterExpr::Gt(Value::Integer(n)),
                    group: 0,
                });
            }
        }
        if !matches!(
            self.record_op(RecordOp::IsEmpty, &probe, &mut [])?,
            Value::Boolean(true)
        ) {
            return refused();
        }
        rec.borrow_mut().fields[last] = Value::Integer(after.unwrap_or(0) + 10000);
        if self.page_trigger_answer(st, "oninsertrecord", vec![Value::Boolean(below)])? == Some(false) {
            return unsupported(
                "OnInsertRecord answered false for the insert of CurrPage.Update of a page variable that never ran (not pinned)",
            );
        }
        self.ret_used = false;
        self.record_op(RecordOp::Insert, rec, &mut [Value::Boolean(true)])?;
        self.set_xrec(st);
        Ok(())
    }
}
