//! When a card's DataCaptionExpression runs (al-runner issue 6924).

use super::*;

impl Session {
    /// Whether the page is a card with a source table and a DataCaptionExpression of its own (a list, a part and an
    /// expression a page extension sets are not pinned, they keep the evaluation at read).
    pub(super) fn card_with_data_caption_expression(&self, st: &St) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        engine.program.objects[object].is_kind(al_ir::ObjKind::Page)
            && engine.program.objects[object]
                .properties
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("DataCaptionExpression"))
            && self.has_source(st)
            && !self.is_list(st)
            && st.borrow().host.is_none()
    }

    /// Runs the DataCaptionExpression of a card and keeps what it returned. It runs after OnAfterGetRecord and again after
    /// OnAfterGetCurrRecord, never before OnInit, OnOpenPage or OnFindRecord (corpus
    /// pages/properties/DataCaptionExprOrder.al:171 DCE_RunModal_TableProcedure_Rows `OnAfterGetRecord(I1 ctl=) |
    /// DC(filter=I1|I2 rec=I1) | OnAfterGetCurrRecord(I1 ctl=) | DC(...)`). The controls bound to globals it sets show the value
    /// of the previous run, as they were read before it ran (same test, `H open ... ctl=` empty, `H next ... ctl=I1|I2`).
    /// Before a trigger runs: the expression runs first when a row trigger ran since it last did, and always before
    /// OnAfterGetCurrRecord (an empty table run modally runs it with no OnAfterGetRecord before, DataCaptionExprOrder.al:224
    /// DCE_RunModal_TableProcedure_Empty `OnFindRecord(=><) | DC(filter=I9 rec=I9) | OnAfterGetCurrRecord(I9 ctl=) | DC(..)`).
    /// Two OnAfterGetRecord in a row run it once, after the second (:371 DCE_PageRun_Rows, the Update step). The expression is not
    /// run before OnAfterGetRecord, OnInit, OnOpenPage or OnFindRecord of the first read.
    pub(super) fn data_caption_before_trigger(&mut self, st: &St, name: &str) -> Result<(), Signal> {
        if name == "onaftergetrecord" || !self.card_with_data_caption_expression(st) {
            return Ok(());
        }
        let due = std::mem::take(&mut st.borrow_mut().data_caption_due);
        if due || name == "onaftergetcurrrecord" {
            self.run_data_caption(st)?;
        }
        Ok(())
    }

    /// After a trigger: OnAfterGetRecord and OnAfterGetCurrRecord leave the expression due.
    pub(super) fn data_caption_after_trigger(&mut self, st: &St, name: &str) {
        if matches!(name, "onaftergetrecord" | "onaftergetcurrrecord") && self.card_with_data_caption_expression(st) {
            st.borrow_mut().data_caption_due = true;
        }
    }

    /// The test code reads or drives the page: a due expression runs first.
    pub(super) fn flush_data_caption(&mut self, st: &St) -> Result<(), Signal> {
        if std::mem::take(&mut st.borrow_mut().data_caption_due) && self.card_with_data_caption_expression(st) {
            self.run_data_caption(st)?;
        }
        Ok(())
    }

    fn run_data_caption(&mut self, st: &St) -> Result<(), Signal> {
        if !self.card_with_data_caption_expression(st) {
            return Ok(());
        }
        if st.borrow().mode == PageMode::New {
            return Ok(());
        }
        self.run_data_caption_held_back(st)
    }

    /// Runs the page's DataCaptionExpression, keeps what it returned and holds back the plain globals it assigned until the
    /// next row arrives (`apply_data_caption_globals`).
    pub(super) fn run_data_caption_held_back(&mut self, st: &St) -> Result<(), Signal> {
        let inst = st.borrow().instance.clone().expect("open page");
        let before = inst.borrow().globals.clone();
        let value = self.data_caption_expression_value(st);
        // The globals the expression assigned show only from the next row that arrives: the page's own code and its controls
        // read the old values until then (DataCaptionExprOrder.al:171 `OnAfterGetCurrRecord(I1 ctl=)` after `DC(filter=I1|I2 ..)`
        // set the global, `H open ... ctl=` empty, `OnAfterGetRecord(I2 ctl=I1|I2)` after Next; :395 DCE_TestPage_OpenView_Rows
        // `H filtered ... ctl=` after the filter). Only plain values are held back; a record, a codeunit or a list stays shared.
        let mut pending = std::mem::take(&mut st.borrow_mut().data_caption_pending);
        {
            let mut i = inst.borrow_mut();
            for (n, old) in before.into_iter().enumerate() {
                if !is_plain(&old)
                    || !is_plain(&i.globals[n])
                    || Self::same_values(std::slice::from_ref(&old), std::slice::from_ref(&i.globals[n]))
                {
                    continue;
                }
                let new = std::mem::replace(&mut i.globals[n], old);
                pending.retain(|(p, _)| *p != n);
                pending.push((n, new));
            }
        }
        st.borrow_mut().data_caption_pending = pending;
        st.borrow_mut().data_caption_value = Some(value?);
        Ok(())
    }

    /// A card with a DataCaptionExpression run modally, after an action that updated it: the client reads a blank row, then the
    /// shown row again before the refresh's OnFindRecord (corpus pages/properties/DataCaptionExprOrder.al:171
    /// DCE_RunModal_TableProcedure_Rows and :195 DCE_RunModal_NoFilter_Rows `OnAfterGetRecord(I2) | OnAfterGetRecord(I2) | DC(..) |
    /// OnAfterGetRecord( ) | OnAfterGetRecord(I2) | OnFindRecord(=)`; Page.Run and TestPage read no blank row, :262, :330). Pinned for
    /// these pages only: another modal card keeps the reads it had.
    pub(super) fn modal_card_blank_read_after_update(&mut self, st: &St) -> Result<(), Signal> {
        let shape = {
            let s = st.borrow();
            s.modal && !s.lookup && s.host.is_none() && s.cursor == Cursor::Row && !s.draft
        };
        if !shape || !self.card_with_data_caption_expression(st) {
            return Ok(());
        }
        self.flush_data_caption(st)?;
        let rec = self.page_rec(st);
        let shown = rec.borrow().fields.clone();
        let table = rec.borrow().table as usize;
        rec.borrow_mut().fields = self.initial_fields(table);
        let blank = self.populate_current(st);
        rec.borrow_mut().fields = shown;
        blank?;
        self.populate_current(st)?;
        // The two reads are the client's own; the expression runs after neither (same test: no `DC` before `OnFindRecord(=)`).
        st.borrow_mut().data_caption_due = false;
        Ok(())
    }

    /// A card with a DataCaptionExpression run modally over a view with no row reads a blank row, OnAfterGetRecord alone, when it
    /// closes, before OnQueryClosePage (corpus pages/properties/DataCaptionExprOrder.al:224 DCE_RunModal_TableProcedure_Empty
    /// `H open cap=LT DCE ctl= | OnAfterGetRecord( ctl=I9) | OnQueryClosePage`; a TestPage on an empty table reads nothing, :349).
    pub(super) fn modal_card_empty_read_at_close(&mut self, st: &St) -> Result<(), Signal> {
        let shape = {
            let s = st.borrow();
            s.modal && !s.lookup && s.host.is_none() && s.cursor == Cursor::None
        };
        if !shape || !self.card_with_data_caption_expression(st) {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let shown = rec.borrow().fields.clone();
        let table = rec.borrow().table as usize;
        rec.borrow_mut().fields = self.initial_fields(table);
        let read = self.populate_current(st);
        rec.borrow_mut().fields = shown;
        st.borrow_mut().data_caption_due = false;
        read
    }

    /// A row arrives: the globals the last DataCaptionExpression assigned take effect.
    pub(super) fn apply_data_caption_globals(&mut self, st: &St) {
        let pending = std::mem::take(&mut st.borrow_mut().data_caption_pending);
        if pending.is_empty() {
            return;
        }
        let inst = st.borrow().instance.clone().expect("open page");
        let mut i = inst.borrow_mut();
        for (n, v) in pending {
            i.globals[n] = v;
        }
    }
}

fn is_plain(v: &Value) -> bool {
    matches!(
        v,
        Value::Boolean(_)
            | Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Decimal(_)
            | Value::Text(_)
            | Value::Code(_)
            | Value::Date(..)
            | Value::Time(_)
            | Value::DateTime(_)
            | Value::Option(..)
            | Value::Enum(..)
    )
}
