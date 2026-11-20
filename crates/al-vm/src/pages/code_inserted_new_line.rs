//! A List page on the line `New()` started, whose table field's OnValidate inserted Rec itself (`if "Entry No." = 0 then
//! Insert(true)`, an AutoIncrement key field) before the control's OnValidate calls `CurrPage.Update()` or `CurrPage.Update(false)`
//! (al-runner issue 8186). BC treats the line as the stored row from then on: `Update()` modifies it (OnModifyRecord and the
//! table's OnModify run inside the OnValidate, OnInsertRecord never runs), `Update(false)` saves nothing, and both refresh onto
//! that row; the next SetValue lands on it. Measured by corpus pull request 1425 (merged, not pinned yet) in
//! pages/currpage/ListNewLineCodeInsertUpdate.al, and by the reporter's repro; assumption `list-new-line-code-insert-update` until a
//! bump pins it. The entry condition is `code_inserted_cell`: the facts every measured cell shares, then the cells themselves. A
//! validation that ends with an error after the page moved onto the row is refused (`refuse_error_after_landing`).

use super::pinned_shape::TestCall;
use super::*;
use al_builtins::filter::FilterExpr;
use al_builtins::page_op::PageMethod;

/// The page's own triggers, as a measured page declares them (ListNewLineCodeInsertUpdate.al).
#[derive(Clone, Copy)]
enum Triggers {
    /// OnNewRecord, OnInsertRecord, OnModifyRecord, OnAfterGetRecord and OnAfterGetCurrRecord: pages 97032 (:136-162) and 97033
    /// (:199-225).
    Row,
    /// None: page 97039 (:345-372) and the repro's page.
    Quiet,
}

/// How the page was opened, with the view it shows.
#[derive(Clone, Copy)]
enum Opened {
    /// The test's `TestPage.OpenEdit()`, no filter (:520, :532, :544, :556, :568, :580, :790, :805).
    OpenEdit,
    /// AL code's `Page.RunModal(Id, Rec)` as a statement, in Edit mode, its record without a filter; the test's ModalPageHandler
    /// gets the page (:232-237, :610-617).
    RunModal,
    /// The same, its record filtered with `SetRange` to one value of the key's first field: the repro of al-runner issue 8186.
    RunModalOneGroup,
}

/// The triggers of `Triggers::Row`, lower case.
const ROW_TRIGGERS: [&str; 5] = [
    "onnewrecord",
    "oninsertrecord",
    "onmodifyrecord",
    "onaftergetrecord",
    "onaftergetcurrrecord",
];

impl Session {
    /// Whether `CurrPage.Update()` (`update_false` false) or `CurrPage.Update(false)` on `st` is a measured cell: a measured
    /// page (`code_inserted_page`) opened as measured (`code_inserted_open`), on the line `New()` started (`code_inserted_line`),
    /// whose table field inserted the line (`inserted_by_table_field`), in a measured view (`code_inserted_view`). Every other shape
    /// keeps main's way.
    fn code_inserted_cell(&mut self, st: &St, update_false: bool) -> Result<bool, Signal> {
        let Some((triggers, split)) = self.code_inserted_page(st) else {
            return Ok(false);
        };
        let Some(opened) = self.code_inserted_open(st)? else {
            return Ok(false);
        };
        // (triggers, AutoSplitKey, open, Update(false)) of each cell.
        let cell = matches!(
            (triggers, split, opened, update_false),
            // Page 97032: Update() :515-536, Update(false) :563-584, RunModal with Update() :588-617.
            (Triggers::Row, true, Opened::OpenEdit, _)
                | (Triggers::Row, true, Opened::RunModal, false)
                // Page 97033: :539-560.
                | (Triggers::Row, false, Opened::OpenEdit, false)
                // Page 97039: :783-810.
                | (Triggers::Quiet, true, Opened::OpenEdit, false)
                // The repro of issue 8186.
                | (Triggers::Quiet, true, Opened::RunModalOneGroup, false)
        );
        Ok(cell && self.code_inserted_line(st) && self.inserted_by_table_field(st)? && self.code_inserted_view(st)?)
    }

    /// The page's triggers and whether it declares AutoSplitKey, for a measured page: PageType List (`is_list_page`, :105) over a
    /// database table, with no part, no page extension and no event subscriber, that declares only PageType, SourceTable,
    /// AutoSplitKey (as true), ApplicationArea and UsageCategory (:105-109, :169-172, :347-351, and the repro's page), and the
    /// triggers of `Triggers`.
    fn code_inserted_page(&self, st: &St) -> Option<(Triggers, bool)> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let measured_property = |(name, value): &(String, String)| match name.to_ascii_lowercase().as_str() {
            "autosplitkey" => value.trim().eq_ignore_ascii_case("true"),
            "pagetype" | "sourcetable" | "applicationarea" | "usagecategory" => true,
            _ => false,
        };
        let properties = &engine.program.objects[object].properties;
        let split = properties.iter().any(|(n, _)| n.eq_ignore_ascii_case("AutoSplitKey"));
        let no_parts = page.controls.iter().all(|c| c.kind != ir::ControlKind::Part);
        let shape = self.is_list_page(st)
            && self.has_source(st)
            && self.unextended_unsubscribed(st)
            && no_parts
            && properties.iter().all(measured_property)
            && !self.page_rec(st).borrow().temporary;
        let names: Vec<&str> = page.triggers.iter().map(|(n, _)| n.as_str()).collect();
        let triggers = match names.len() {
            0 => Triggers::Quiet,
            5 if ROW_TRIGGERS.iter().all(|t| names.contains(t)) => Triggers::Row,
            _ => return None,
        };
        shape.then_some((triggers, split))
    }

    /// How `st` was opened, for a measured open: the test's OpenEdit (`PageState::test_open_edit`) on a view without filters, or
    /// a static `Page.RunModal(Id, Rec)` statement in Edit mode without filters or with one `SetRange` on the key's first field
    /// to the line's value in group 0. The view is in the primary key's ascending order and not limited to marked rows.
    fn code_inserted_open(&mut self, st: &St) -> Result<Option<Opened>, Signal> {
        let (open_edit, run_modal) = {
            let s = st.borrow();
            let run_modal = s.modal
                && s.modal_run == ModalRun::Static
                && s.statement_run
                && s.static_run_record
                && !s.lookup
                && s.mode == PageMode::Edit
                && s.host.is_none();
            (s.test_open_edit(), run_modal)
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let r = rec.borrow();
        let sorted = r.key == 0 && r.ascending && r.field_descending == 0 && r.adhoc.is_none() && !r.marked_only;
        let one_group = |f: &al_builtins::filter::FieldFilter| {
            let line = &r.fields[f.field..=f.field];
            f.group == 0
                && table.keys[0].first() == Some(&f.field)
                && matches!(&f.expr, FilterExpr::Range(Some(a), Some(b))
                    if Self::same_values(std::slice::from_ref(a), line) && Self::same_values(std::slice::from_ref(b), line))
        };
        let opened = match (open_edit, run_modal, r.filters.as_slice()) {
            (true, false, []) => Opened::OpenEdit,
            (false, true, []) => Opened::RunModal,
            (false, true, [f]) if one_group(f) => Opened::RunModalOneGroup,
            _ => return Ok(None),
        };
        Ok(sorted.then_some(opened))
    }

    /// The page stands on the line `New()` started, editable, inside a control's validation, and the test made exactly the
    /// measured calls on it since the open: `New()`, then the SetValue that runs now (:477-479, :697-698, the repro's handler).
    fn code_inserted_line(&self, st: &St) -> bool {
        let line = {
            let s = st.borrow();
            s.open
                && s.host.is_none()
                && s.parts.is_empty()
                && s.hidden_parts.is_empty()
                && !s.test_opened_new
                && s.cursor == Cursor::NewLine
                && s.draft
                && s.new_by_call
                && !s.draft_stored
                && !s.editable_off
                && s.validating_control
        };
        line && matches!(
            Self::test_calls(st).as_deref(),
            Some([TestCall::Page(PageMethod::New), TestCall::Write(_)])
        )
    }

    /// Whether Rec is the row the table field's OnValidate inserted from the line before the control's OnValidate began (:30-37):
    /// the primary key is a Code field, then an AutoIncrement Integer field (:23-27, :44); the AutoIncrement field was blank when
    /// the line started and the Code field holds the line's value; the row is stored and was written after the line started and
    /// before the control's own OnValidate began (its SystemRowVersion above the store's mark when the line started,
    /// `new_line_writes`, and not above the mark when the control's OnValidate began, `control_onvalidate`). The table has
    /// no table extension and no event subscriber.
    fn inserted_by_table_field(&mut self, st: &St) -> Result<bool, Signal> {
        let (Some(start), Some((line_mark, _)), Some((control_mark, _))) = ({
            let s = st.borrow();
            (s.draft_start.clone(), s.new_line_writes, s.control_onvalidate.as_ref().map(|v| v.writes))
        }) else {
            return Ok(false);
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let engine = self.engine();
        let object = table.object as usize;
        let Some(schema) = engine.program.objects[object].table.as_ref() else {
            return Ok(false);
        };
        let [code, auto] = table.keys[0][..] else {
            return Ok(false);
        };
        let key_shape = matches!(schema.fields[code].ty, al_builtins::value::RtType::Code(_))
            && matches!(schema.fields[auto].ty, al_builtins::value::RtType::Integer)
            && schema.fields[auto].auto_increment;
        let plain_table = schema.extensions.is_empty() && !engine.events.any_subscriber_of(object);
        let from_line = matches!(start.get(auto), Some(Value::Integer(0)))
            && Self::same_values(&rec.borrow().fields[code..=code], &start[code..=code]);
        if !(key_shape && plain_table && from_line) {
            return Ok(false);
        }
        let Some(stored) = self.stored_row(&rec)? else {
            return Ok(false);
        };
        Ok(crate::stale_rename::row_version(&table, &stored)
            .is_some_and(|v| (line_mark + 1..=control_mark).contains(&v)))
    }

    /// Whether the view holds the inserted row last, alone or after one row with the same value in the key's first field: the
    /// measured views (an empty view, or row 'A' before the line, :386-398, :620-641, and the repro's group). Two rows before it,
    /// or a row after it, are not measured.
    fn code_inserted_view(&mut self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let Some(&code) = table.keys[0].first() else {
            return Ok(false);
        };
        let row = table.primary_key(&rec.borrow().fields);
        let is_row = |fields: &[Value]| Self::same_key(&table.primary_key(fields), &row);
        Ok(match self.view_rows(&rec, 3)?.as_slice() {
            [only] => is_row(only),
            [before, last] => {
                is_row(last) && Self::same_values(&before[code..=code], &rec.borrow().fields[code..=code])
            }
            _ => false,
        })
    }

    /// The page control's own OnValidate trigger `r` of control `idx`, recorded in `PageState::control_onvalidate` while it runs
    /// (`inserted_by_table_field` and `refresh_new_line.rs` `found_row_cell` read it).
    pub(super) fn run_control_onvalidate(&mut self, st: &St, idx: usize, r: usize) -> Result<Value, Signal> {
        let start = Some(crate::pages::state::ControlOnValidate::new(self.write_mark(), self.store.change_counter()));
        let outer = std::mem::replace(&mut st.borrow_mut().control_onvalidate, start);
        let result = self.control_routine(st, idx, r);
        let mut s = st.borrow_mut();
        s.control_onvalidate = outer;
        // An OnValidate that failed after its save left the new-row line runs no refresh (`ValidateStart::left_new_line`).
        if let (Err(_), Some(start)) = (&result, s.filters_before_validate.as_mut()) {
            start.left_new_line = false;
        }
        drop(s);
        result
    }

    /// A control's validation ended with `e` (`validate_step_close`): an AL error after `land_on_code_inserted_line` moved the
    /// page onto the row (an OnValidate that fails after `CurrPage.Update`, whose insert the error rolls back) is not measured,
    /// and refused. Every other signal goes on.
    pub(super) fn refuse_error_after_landing(&mut self, st: &St, e: Signal) -> Signal {
        let landed = st.borrow().refresh.starts_from_code_inserted_line();
        match e {
            Signal::Error(_) if landed => {
                st.borrow_mut().forget_code_inserted_line();
                Signal::unsupported(
                    "an error ended the validation after CurrPage.Update moved the page from its new line onto the row the table \
                     field inserted (not measured)",
                )
            }
            e => e,
        }
    }
    /// `CurrPage.Update()` (`arg` `None`) or `CurrPage.Update(false)` in a measured cell (`code_inserted_cell`): the page leaves
    /// the new line for the stored row before the call goes on, so `Update()` saves it as a stored row (its modify,
    /// `P.MOD(X/);T.M(X/);` before the OnValidate ends, ListNewLineCodeInsertUpdate.al:419 `ExpectedSetName`) and `Update(false)`
    /// saves nothing. The refresh the call asked for reads the view from its first row (`code_inserted_refresh_due`).
    /// `Update(true)` written out is not measured and keeps main's way.
    pub(super) fn land_on_code_inserted_line(&mut self, st: &St, arg: Option<&Value>) -> Result<(), Signal> {
        let save = match arg {
            None => true,
            Some(Value::Boolean(false)) => false,
            _ => return Ok(()),
        };
        if !self.code_inserted_cell(st, !save)? {
            return Ok(());
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ListNewLineCodeInsertUpdate);
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.new_by_call = false;
            s.dirty = false;
            s.refresh.land_on_code_inserted_line();
        }
        if !save {
            self.set_xrec(st);
        }
        let key = self.row_key(&self.page_rec(st))?;
        let calls = st.borrow().test_calls.len();
        st.borrow_mut().code_inserted_row = Some((key, calls));
        Ok(())
    }

    /// Whether the refresh about to run is the one `land_on_code_inserted_line` asked for: `asked` (the mark it left, taken by
    /// every refresh), a refresh of `CurrPage.Update` on the open page standing on the landed row.
    pub(super) fn code_inserted_refresh_due(&mut self, st: &St, asked: bool, by_update: bool) -> Result<bool, Signal> {
        if !asked || !by_update || !st.borrow().open || st.borrow().cursor != Cursor::Row {
            return Ok(false);
        }
        let key = self.row_key(&self.page_rec(st))?;
        Ok(st
            .borrow()
            .code_inserted_row
            .as_ref()
            .is_some_and(|(landed, _)| Self::same_key(landed, &key)))
    }

    /// `Previous()` on the row that refresh landed on, when it is the only row of the view and the test only read and wrote
    /// controls since: BC answers false, runs no trigger and keeps an edit typed since unsaved (ListNewLineCodeInsertUpdate.al:455
    /// `prev` steps of the empty views: `F/X` and an empty log; `Next()` then saves it). `None` for every other move, which also
    /// ends the mark.
    pub(super) fn previous_on_code_inserted_row(&mut self, st: &St, how: PageMethod) -> Result<Option<bool>, Signal> {
        let Some((landed, calls)) = st.borrow_mut().code_inserted_row.take() else {
            return Ok(None);
        };
        if how != PageMethod::Previous || st.borrow().cursor != Cursor::Row {
            return Ok(None);
        }
        // The calls since the landing SetValue: control reads and writes, then this Previous.
        let only_fields = Self::test_calls(st).is_some_and(|c| {
            c.len() > calls
                && c[calls..c.len() - 1]
                    .iter()
                    .all(|t| matches!(t, TestCall::Read | TestCall::Write(_)))
                && c[c.len() - 1] == TestCall::Page(PageMethod::Previous)
        });
        let key = self.row_key(&self.page_rec(st))?;
        if !only_fields || !Self::same_key(&landed, &key) || self.view_row_count(st)? != 1 {
            return Ok(None);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ListNewLineCodeInsertUpdate);
        Ok(Some(false))
    }
}
