//! A List page on the line `New()` started, whose table field's OnValidate inserted Rec itself (`if "Entry No." = 0 then
//! Insert(true)`, an AutoIncrement key field) before the control's OnValidate calls `CurrPage.Update()` or `CurrPage.Update(false)`
//! (al-runner issue 8186). BC treats the line as the stored row from then on: `Update()` modifies it (OnModifyRecord and the
//! table's OnModify run inside the OnValidate, OnInsertRecord never runs), `Update(false)` saves nothing, and both refresh onto
//! that row; the next SetValue lands on it. Measured by corpus pull request 1425 (merged, not pinned yet) in
//! pages/currpage/ListNewLineCodeInsertUpdate.al, and by the reporter's repro; assumption `list-new-line-code-insert-update` until a
//! bump pins it. `CurrPage.Update(true)` written out, and `CurrPage.SaveRecord()` followed by `CurrPage.Update(false)`, do what
//! `Update()` does (SaveRecord modifies the row, the Update(false) after it asks the same refresh; al-runner issue 8285): measured
//! by corpus pull request 1436 (open) in pages/currpage/ListNewLineSaveRecordUpdate.al, assumption
//! `list-new-line-code-insert-save`. The entry condition is `code_inserted_cell`: the facts every measured cell shares, then the
//! cells themselves. A validation that ends with an error after the page moved onto the row is refused
//! (`refuse_error_after_landing`), and so is one whose SaveRecord left the line without the `Update(false)` after it
//! (`code_inserted_step_end`).

use super::pinned_shape::TestCall;
use super::state::CodeInsertedRow;
use super::*;
use al_builtins::assumptions::{Assumption, assume};
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
    /// OnAfterGetRecord only, and one CardPart FactBox (`code_inserted_factbox`): ListNewLineSaveRecordUpdate.al page 97474
    /// (:210-254), and the partner app's page in the repro of al-runner issue 8285.
    FactboxHost,
}

/// How the page was opened, with the view it shows.
#[derive(Clone, Copy)]
enum Opened {
    /// The test's `TestPage.OpenEdit()`, no filter (:520, :532, :544, :556, :568, :580, :790, :805).
    OpenEdit,
    /// AL code's `Page.RunModal(Id, Rec)` as a statement, in Edit mode, its record without a filter; the test's ModalPageHandler
    /// gets the page (:232-237, :610-617).
    RunModal,
    /// The same, its record filtered with `SetRange` to one value of the key's first field: the repro of al-runner issue 8186,
    /// and ListNewLineSaveRecordUpdate.al:272-281 (the opener) with a row of another group outside the view.
    RunModalOneGroup,
}

/// The page call that leaves the line, as the control's OnValidate makes it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Leave {
    /// `CurrPage.Update()`.
    Update,
    /// `CurrPage.Update(false)`.
    UpdateFalse,
    /// `CurrPage.Update(true)` (ListNewLineSaveRecordUpdate.al:131, :196).
    UpdateTrue,
    /// `CurrPage.SaveRecord()`, then `CurrPage.Update(false)` (ListNewLineSaveRecordUpdate.al:127-129, :193-194).
    SaveRecord,
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
    /// The assumption a call `leave` on `st` runs under when it is a measured cell: a measured page (`code_inserted_page`)
    /// opened as measured (`code_inserted_open`), on the line `New()` started (`code_inserted_line`), whose table field inserted
    /// the line (`inserted_by_table_field`), in a measured view (`code_inserted_view`). `None` for every other shape, which keeps
    /// main's way.
    fn code_inserted_cell(&mut self, st: &St, leave: Leave) -> Result<Option<Assumption>, Signal> {
        let Some((triggers, split)) = self.code_inserted_page(st)? else {
            return Ok(None);
        };
        let Some(opened) = self.code_inserted_open(st)? else {
            return Ok(None);
        };
        // (triggers, AutoSplitKey, open, call) of each cell.
        let assumption = match (triggers, split, opened, leave) {
            // ListNewLineCodeInsertUpdate.al, page 97032: Update() :515-536, Update(false) :563-584, RunModal with Update()
            // :588-617.
            (Triggers::Row, true, Opened::OpenEdit, Leave::Update | Leave::UpdateFalse)
            | (Triggers::Row, true, Opened::RunModal, Leave::Update)
            // Page 97033: :539-560.
            | (Triggers::Row, false, Opened::OpenEdit, Leave::Update)
            // Page 97039: :783-810.
            | (Triggers::Quiet, true, Opened::OpenEdit, Leave::Update)
            // The repro of issue 8186.
            | (Triggers::Quiet, true, Opened::RunModalOneGroup, Leave::Update) => Assumption::ListNewLineCodeInsertUpdate,
            // ListNewLineSaveRecordUpdate.al, page 97472 (:105-170): :429-559, :739; page 97473 (:172-208): :561-675, :757-782.
            (
                Triggers::Row | Triggers::Quiet,
                true,
                Opened::OpenEdit | Opened::RunModalOneGroup,
                Leave::UpdateTrue | Leave::SaveRecord,
            ) => Assumption::ListNewLineCodeInsertSave,
            // ListNewLineSaveRecordUpdate.al, page 97474: :677-737, :784-829.
            (Triggers::FactboxHost, true, Opened::OpenEdit | Opened::RunModalOneGroup, Leave::SaveRecord) => {
                Assumption::ListNewLineCodeInsertSave
            }
            _ => return Ok(None),
        };
        let measured =
            self.code_inserted_line(st) && self.inserted_by_table_field(st)? && self.code_inserted_view(st)?;
        Ok(measured.then_some(assumption))
    }

    /// The page's triggers and whether it declares AutoSplitKey, for a measured page: PageType List (`is_list_page`, :105) over a
    /// database table, with no page extension and no event subscriber, that declares only PageType, SourceTable, AutoSplitKey (as
    /// true), ApplicationArea and UsageCategory (:105-109, :169-172, :347-351, and the repro's page; ListNewLineSaveRecordUpdate.al
    /// :105-111, :172-178, :210-216), and the triggers of `Triggers`: no part, except the one FactBox of `Triggers::FactboxHost`.
    fn code_inserted_page(&self, st: &St) -> Result<Option<(Triggers, bool)>, Signal> {
        let (shape, split, triggers, no_parts) = {
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
            let shape = self.is_list_page(st)
                && self.has_source(st)
                && self.unextended_unsubscribed(st)
                && properties.iter().all(measured_property)
                && !self.page_rec(st).borrow().temporary;
            let names: Vec<String> = page.triggers.iter().map(|(n, _)| n.clone()).collect();
            let no_parts = page.controls.iter().all(|c| c.kind != ir::ControlKind::Part);
            (shape, split, names, no_parts)
        };
        let triggers = match triggers.as_slice() {
            [] if no_parts => Triggers::Quiet,
            names if no_parts && names.len() == 5 && ROW_TRIGGERS.iter().all(|t| names.iter().any(|n| n == t)) => {
                Triggers::Row
            }
            [only] if only == "onaftergetrecord" && self.code_inserted_factbox(st)? => Triggers::FactboxHost,
            _ => return Ok(None),
        };
        Ok(shape.then_some((triggers, split)))
    }

    /// Whether host `st` shows the measured FactBox, every fact required (ListNewLineSaveRecordUpdate.al:210-270, and the
    /// partner app's page in the repro of al-runner issue 8285, whose controls carry no ApplicationArea and whose part declares
    /// UsageCategory):
    /// - one part control, shown, in the FactBoxes area (:237-244), with no property but ApplicationArea and a SubPageLink that
    ///   links each field of the primary key to itself, in key order (:242), no SubPageView, no Provider;
    /// - the part: a CardPart over the host's table (:258-259), its Rec stored, not marked, filtered by the link only, with no
    ///   page trigger, no page extension and no subscriber, that declares only PageType, SourceTable, ApplicationArea and
    ///   UsageCategory, and whose controls are fields of Rec without a trigger and with no property but ApplicationArea
    ///   (:266-267);
    /// - the host's other controls: a repeater without a property and fields of Rec with no property but ApplicationArea
    ///   (:220-234).
    fn code_inserted_factbox(&self, st: &St) -> Result<bool, Signal> {
        let parts = st.borrow().parts.clone();
        let [(idx, part)] = parts.as_slice() else {
            return Ok(false);
        };
        if !st.borrow().hidden_parts.is_empty() || !self.has_source(part) || !self.unextended_unsubscribed(part) {
            return Ok(false);
        }
        let (host_rec, part_rec) = (self.page_rec(st), self.page_rec(part));
        let key = self.table(&host_rec)?.keys[0].clone();
        let same_table = {
            let (h, p) = (host_rec.borrow(), part_rec.borrow());
            h.table == p.table
                && !p.temporary
                && !p.marked_only
                && p.filters
                    .iter()
                    .all(|f| f.group == super::parts::LINK_GROUP && key.contains(&f.field))
        };
        let engine = self.engine();
        let part_object = part.borrow().object as usize;
        let (host_page, part_page) = (page_of(&engine, st), page_of(&engine, part));
        let control = &host_page.controls[*idx];
        let link = control.part_link.as_ref().is_some_and(|l| {
            l.provider.is_none()
                && l.view.is_none()
                && l.entries.len() == key.len()
                && l.entries
                    .iter()
                    .zip(&key)
                    .all(|(e, k)| matches!(e, ir::LinkEntry::Field(pf, hf) if pf == k && hf == k))
        });
        let part_control = control.factbox
            && control
                .props
                .iter()
                .all(|(k, _)| k == "applicationarea" || k == "subpagelink");
        let card_part = engine.object_facts[part_object].page_type == Some(ir::PageType::Other("cardpart".into()));
        let part_properties = engine.program.objects[part_object].properties.iter().all(|(k, _)| {
            matches!(
                k.to_ascii_lowercase().as_str(),
                "pagetype" | "sourcetable" | "applicationarea" | "usagecategory"
            )
        });
        let plain = |controls: &[ir::Control], part_at: Option<usize>| {
            controls.iter().enumerate().all(|(i, c)| match c.kind {
                ir::ControlKind::Field => {
                    matches!(c.binding, ir::Binding::Field(_))
                        && c.owner.is_none()
                        && c.ext_triggers.is_empty()
                        && c.props.iter().all(|(k, _)| k == "applicationarea")
                }
                ir::ControlKind::Repeater => c.props.is_empty(),
                ir::ControlKind::Part => part_at == Some(i),
                _ => false,
            })
        };
        Ok(same_table
            && link
            && part_control
            && card_part
            && part_properties
            && part_page.triggers.is_empty()
            && part_page.controls.iter().all(|c| c.triggers.is_empty())
            && plain(&host_page.controls, Some(*idx))
            && plain(&part_page.controls, None))
    }

    /// How `st` was opened, for a measured open: the test's OpenEdit (`PageState::test_open_edit`) on a view without filters, or
    /// a static `Page.RunModal(Id, Rec)` statement in Edit mode without filters or with one `SetRange` on the key's first field
    /// to the line's value in group 0. The view is in the primary key's ascending order and not limited to marked rows.
    fn code_inserted_open(&mut self, st: &St) -> Result<Option<Opened>, Signal> {
        let (open_edit, run_modal) = {
            let s = st.borrow();
            let run_modal = s.modal
                && s.run_static == Some(true)
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
    /// measured calls on it since the open: `New()`, then the SetValue that runs now (:477-479, :697-698, the repro's handler;
    /// ListNewLineSaveRecordUpdate.al:354-356, :397-399).
    fn code_inserted_line(&self, st: &St) -> bool {
        let line = {
            let s = st.borrow();
            s.open
                && s.host.is_none()
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
    /// `new_line_writes`, and not above the mark when the control's OnValidate began, `control_onvalidate_writes`). The table has
    /// no table extension and no event subscriber.
    fn inserted_by_table_field(&mut self, st: &St) -> Result<bool, Signal> {
        let (Some(start), Some((line_mark, _)), Some((control_mark, _))) = ({
            let s = st.borrow();
            (s.draft_start.clone(), s.new_line_writes, s.control_onvalidate_writes)
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

    /// The page control's own OnValidate trigger `r` of control `idx`, with the store's write mark at its start in
    /// `PageState::control_onvalidate_writes` while it runs (`inserted_by_table_field` reads it).
    pub(super) fn run_control_onvalidate(&mut self, st: &St, idx: usize, r: usize) -> Result<Value, Signal> {
        let mark = Some(self.write_mark());
        let outer = std::mem::replace(&mut st.borrow_mut().control_onvalidate_writes, mark);
        let result = self.control_routine(st, idx, r);
        st.borrow_mut().control_onvalidate_writes = outer;
        result
    }

    /// A control's validation ended with `e` (`validate_step_close`): an AL error after `land_on_code_inserted_line` or
    /// `save_record_on_code_inserted_line` moved the page onto the row (an OnValidate that fails after `CurrPage.Update` or
    /// `CurrPage.SaveRecord`, whose insert the error rolls back) is not measured, and refused. Every other signal goes on.
    pub(super) fn refuse_error_after_landing(&mut self, st: &St, e: Signal) -> Signal {
        let landed = {
            let s = st.borrow();
            s.refresh_from_code_inserted_line || s.code_inserted_saved
        };
        match e {
            Signal::Error(_) if landed => {
                st.borrow_mut().forget_code_inserted_line();
                Signal::unsupported(
                    "an error ended the validation after CurrPage.Update or CurrPage.SaveRecord moved the page from its new line \
                     onto the row the table field inserted (not measured)",
                )
            }
            e => e,
        }
    }

    /// A control's validation ended without an error (`validate_step_close`): a `CurrPage.SaveRecord` that left the new line
    /// without the `CurrPage.Update(false)` after it is not measured, and refused (ListNewLineSaveRecordUpdate.al:127-129,
    /// :193-194 measure the pair only).
    pub(super) fn code_inserted_step_end(&mut self, st: &St) -> Result<(), Signal> {
        if !st.borrow().code_inserted_saved {
            return Ok(());
        }
        st.borrow_mut().forget_code_inserted_line();
        unsupported(
            "CurrPage.SaveRecord moved the page from its new line onto the row the table field inserted, and the validation ended \
             without the CurrPage.Update(false) after it (not measured)",
        )
    }

    /// `CurrPage.Update()` (`arg` `None`), `CurrPage.Update(false)` or `CurrPage.Update(true)` in a measured cell
    /// (`code_inserted_cell`): the page leaves the new line for the stored row before the call goes on, so `Update()` and
    /// `Update(true)` save it as a stored row (its modify, `P.MOD(X/);T.M(X/);` before the OnValidate ends,
    /// ListNewLineCodeInsertUpdate.al:419 `ExpectedSetName`, ListNewLineSaveRecordUpdate.al:510 `setname`) and `Update(false)`
    /// saves nothing. The refresh the call asked for reads the view from its first row (`code_inserted_refresh_due`). After a
    /// `CurrPage.SaveRecord` that left the line, see `update_after_code_inserted_save`.
    pub(super) fn land_on_code_inserted_line(&mut self, st: &St, arg: Option<&Value>) -> Result<(), Signal> {
        if st.borrow().code_inserted_saved {
            return self.update_after_code_inserted_save(st, arg);
        }
        let leave = match arg {
            None => Leave::Update,
            Some(Value::Boolean(false)) => Leave::UpdateFalse,
            Some(Value::Boolean(true)) => Leave::UpdateTrue,
            _ => return Ok(()),
        };
        let Some(assumption) = self.code_inserted_cell(st, leave)? else {
            return Ok(());
        };
        self.leave_code_inserted_line(st, assumption, leave != Leave::UpdateFalse);
        self.ask_code_inserted_refresh(st, assumption)
    }

    /// `CurrPage.SaveRecord()` in a measured cell (`code_inserted_cell`, `Leave::SaveRecord`): the page leaves the new line for the
    /// stored row, and the save goes on as the modify of that row (`P.MOD(X/);T.M(X/);P.SR;`, ListNewLineSaveRecordUpdate.al:445
    /// `setname`). The `CurrPage.Update(false)` the cell makes next asks the refresh (`update_after_code_inserted_save`). A second
    /// SaveRecord before it is not measured, and refused.
    pub(super) fn save_record_on_code_inserted_line(&mut self, st: &St) -> Result<(), Signal> {
        if st.borrow().code_inserted_saved {
            st.borrow_mut().forget_code_inserted_line();
            return unsupported(
                "a second CurrPage.SaveRecord after the first moved the page from its new line onto the row the table field \
                 inserted (not measured)",
            );
        }
        let Some(assumption) = self.code_inserted_cell(st, Leave::SaveRecord)? else {
            return Ok(());
        };
        self.leave_code_inserted_line(st, assumption, true);
        st.borrow_mut().code_inserted_saved = true;
        Ok(())
    }

    /// The `CurrPage.Update` after a `CurrPage.SaveRecord` that left the line (`save_record_on_code_inserted_line`): `Update(false)`
    /// asks the refresh `Update()` asks (ListNewLineSaveRecordUpdate.al:445 `setname`: `P.SR;P.V.end;P.AGR(X);P.AGR(A);P.AGR(X);
    /// P.AGR(X);P.AGCR(X);`); `Update()` and `Update(true)` are not measured there, and refused.
    fn update_after_code_inserted_save(&mut self, st: &St, arg: Option<&Value>) -> Result<(), Signal> {
        if !matches!(arg, Some(Value::Boolean(false))) {
            st.borrow_mut().forget_code_inserted_line();
            return unsupported(
                "CurrPage.Update() or Update(true) after CurrPage.SaveRecord moved the page from its new line onto the row the \
                 table field inserted (not measured)",
            );
        }
        st.borrow_mut().code_inserted_saved = false;
        assume(Assumption::ListNewLineCodeInsertSave);
        self.ask_code_inserted_refresh(st, Assumption::ListNewLineCodeInsertSave)
    }

    /// The page leaves the new line for the row the table field inserted, under `assumption`; without `save` the row it shows
    /// is xRec from here on.
    fn leave_code_inserted_line(&mut self, st: &St, assumption: Assumption, save: bool) {
        assume(assumption);
        {
            let mut s = st.borrow_mut();
            s.cursor = Cursor::Row;
            s.draft = false;
            s.new_by_call = false;
            s.dirty = false;
        }
        if !save {
            self.set_xrec(st);
        }
    }

    /// The refresh about to run lands on the row the page left its line for (`code_inserted_refresh_due`), and the next move
    /// starts from it (`previous_on_code_inserted_row`, which marks `assumption`).
    fn ask_code_inserted_refresh(&mut self, st: &St, assumption: Assumption) -> Result<(), Signal> {
        let key = self.row_key(&self.page_rec(st))?;
        let mut s = st.borrow_mut();
        s.refresh_from_code_inserted_line = true;
        let calls = s.test_calls.len();
        s.code_inserted_row = Some(CodeInsertedRow {
            key,
            calls,
            assumption,
            read_before_leave: false,
        });
        Ok(())
    }

    /// The refresh `land_on_code_inserted_line` asked for (`code_inserted_refresh_due`): the view again from the row
    /// (`reread_view_from_row`). The FactBox host reads the row, then the row before it, and not the row once more
    /// (ListNewLineSaveRecordUpdate.al:692 `setname=...T.M(X/);P.AGR(X/);P.AGR(A/);`, :677 `P.AGR(X/);` on an empty view, where the
    /// page with the five triggers reads `P.AGR(X);P.AGR(A);P.AGR(X);P.AGR(X);P.AGCR(X);`, :445); with a row before, it reads
    /// the row again before it leaves it (`read_code_inserted_row_again`).
    pub(super) fn code_inserted_refresh(
        &mut self,
        st: &St,
        without_curr: bool,
        without_parts: bool,
    ) -> Result<(), Signal> {
        let factbox = matches!(self.code_inserted_page(st)?, Some((Triggers::FactboxHost, _)));
        self.reread_view_from_row(st, without_curr, without_parts, !factbox)?;
        if factbox && self.view_row_count(st)? > 1 {
            if let Some(row) = st.borrow_mut().code_inserted_row.as_mut() {
                row.read_before_leave = true;
            }
        }
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
            .is_some_and(|row| Self::same_key(&row.key, &key)))
    }

    /// `Previous()` on the row that refresh landed on, when the test only read and wrote controls since: on the view's only row
    /// BC answers false, runs no trigger and keeps an edit typed since unsaved (ListNewLineCodeInsertUpdate.al:455 `prev` steps of
    /// the empty views: `F/X` and an empty log; `Next()` then saves it; ListNewLineSaveRecordUpdate.al:429, :494, :561, :618, :677
    /// the same after SaveRecord and Update(true)). The FactBox host with a row before reads the row again, then moves as any
    /// list does (`read_code_inserted_row_again`). `None` for every other move, which also ends the mark; on the FactBox host
    /// another move is not measured, and refused.
    pub(super) fn previous_on_code_inserted_row(&mut self, st: &St, how: PageMethod) -> Result<Option<bool>, Signal> {
        let Some(row) = st.borrow_mut().code_inserted_row.take() else {
            return Ok(None);
        };
        let previous = how == PageMethod::Previous
            && st.borrow().cursor == Cursor::Row
            && self.only_fields_since(st, &row, &[&[TestCall::Page(PageMethod::Previous)]])
            && Self::same_key(&row.key, &self.row_key(&self.page_rec(st))?);
        if row.read_before_leave {
            if !(previous && self.wrote_since(st, &row)) {
                return unsupported(MOVE_OFF_FACTBOX_ROW);
            }
            self.read_code_inserted_row_again(st, &row)?;
            return Ok(None);
        }
        let factbox = matches!(self.code_inserted_page(st)?, Some((Triggers::FactboxHost, _)));
        if !previous || self.view_row_count(st)? != 1 || (factbox && !self.wrote_since(st, &row)) {
            if factbox {
                return unsupported(MOVE_OFF_FACTBOX_ROW);
            }
            return Ok(None);
        }
        assume(row.assumption);
        Ok(Some(false))
    }

    /// The page closes (`close_with`) on the row its line became. On the FactBox host the close is measured after a SetValue and
    /// control reads only (`Close()` or `OK().Invoke()`), and any other close there is refused; with a row before, the host
    /// reads the row again before the close saves it (ListNewLineSaveRecordUpdate.al:784 `close=P.AGR(X/);T.M(X/V);`, :801 with
    /// OK), on an empty view it does not (:816 `close=T.M(X/V);`). Other pages close as before.
    pub(super) fn code_inserted_row_at_close(&mut self, st: &St) -> Result<(), Signal> {
        let Some(row) = st.borrow_mut().code_inserted_row.take() else {
            return Ok(());
        };
        let factbox = matches!(self.code_inserted_page(st)?, Some((Triggers::FactboxHost, _)));
        if !factbox {
            return Ok(());
        }
        // `Close()`, or `OK().Invoke()`.
        let closes: [&[TestCall]; 2] = [
            &[TestCall::Page(PageMethod::Close)],
            &[TestCall::Page(PageMethod::OK), TestCall::Other(PageMethod::Invoke)],
        ];
        if !(st.borrow().cursor == Cursor::Row
            && self.only_fields_since(st, &row, &closes)
            && self.wrote_since(st, &row))
        {
            return unsupported(MOVE_OFF_FACTBOX_ROW);
        }
        if row.read_before_leave {
            self.read_code_inserted_row_again(st, &row)?;
        }
        Ok(())
    }

    /// The FactBox host, whose refresh read another row last (`code_inserted_refresh`), reads the row it stands on again before
    /// it leaves it: OnAfterGetRecord on the row as stored, after which Rec holds the edits typed since again, which the save then
    /// writes (ListNewLineSaveRecordUpdate.al:692 `prevlog=P.AGR(X/);T.M(X/V);`, :784 `close=P.AGR(X/);T.M(X/V);`).
    fn read_code_inserted_row_again(&mut self, st: &St, row: &CodeInsertedRow) -> Result<(), Signal> {
        assume(row.assumption);
        let rec = self.page_rec(st);
        let Some(stored) = self.stored_row(&rec)? else {
            return unsupported(MOVE_OFF_FACTBOX_ROW);
        };
        let typed = std::mem::replace(&mut rec.borrow_mut().fields, stored);
        let read = self.page_trigger(st, "onaftergetrecord", vec![]);
        rec.borrow_mut().fields = typed;
        read.map(|_| ())
    }

    /// Whether the test wrote a control of `st` since the landing on `row`: every measured move off the FactBox host's row follows
    /// the SetValue on Value (ListNewLineSaveRecordUpdate.al:404-416).
    fn wrote_since(&self, st: &St, row: &CodeInsertedRow) -> bool {
        Self::test_calls(st).is_some_and(|c| {
            c.get(row.calls..)
                .unwrap_or_default()
                .iter()
                .any(|t| matches!(t, TestCall::Write(_)))
        })
    }

    /// Whether the test's calls on `st` since the landing on `row` are control reads and writes, then one of `tails`.
    fn only_fields_since(&self, st: &St, row: &CodeInsertedRow, tails: &[&[TestCall]]) -> bool {
        Self::test_calls(st).is_some_and(|c| {
            let since = c.get(row.calls..).unwrap_or_default();
            tails.iter().any(|tail| {
                since.ends_with(tail)
                    && since[..since.len() - tail.len()]
                        .iter()
                        .all(|t| matches!(t, TestCall::Read | TestCall::Write(_)))
            })
        })
    }
}

/// The refusal of a call that leaves the FactBox host's row other than as measured.
const MOVE_OFF_FACTBOX_ROW: &str = "a call that leaves the row a List page with a CardPart FactBox made of its new line, other than \
                                    Previous(), Close() and OK() after a SetValue and control reads (not measured)";
