//! The page with code of al-runner issue 8434: page 97492 or 97493 of ListNewLineFactboxesCode.al (corpus pull request 1454,
//! merged, not pinned yet), with its table, its three CardPart FactBoxes (one enqueues a page background task), the tables
//! they read and the codeunits they call, all line for line (`code_inserted_pinned.rs` `coded_code`). The probe measures it on
//! Business Central 29.1 (11 of 11 green), under the assumption `list-new-line-coded-factboxes` until a bump pins it. Opened
//! by a Page.RunModal statement on a record filtered to one group, on an empty view (:1401-1476):
//! - the open runs the parts' OnAfterGetCurrRecord on the blank record (`open=P.OPEN;W.AGCR();`), `coded_open_on_empty_view`;
//! - the first SetValue after `New()` runs it again, before the validation (`setname=W.AGCR();T.VN(Station);...`),
//!   `coded_line_before_set_value`;
//! - the validation lands on the row as on the FactBox host (`code_inserted_cell`), and its refresh reads the row twice in a
//!   secondary key and once in the primary key, then the parts (`P.AGR(Station/);P.AGR(Station/);R.AGR(Station/0);
//!   W.AGCR(Station/s);W.DONE(K2);`, :1407 against :1445), `coded_refresh`;
//! - up to two later SetValues whose validation saves the row and calls `CurrPage.Update(false)` refresh the same way
//!   (:1407 `setvalue`, :1539 the value already there, :1576 a second one), then `OK()` runs nothing (`ok=`).
//!
//! Any other page, or other code, keeps main's way (`coded_page`). The table may have manual subscribers nobody binds
//! (:255-274, :1592 binds one: refused). A row before the line (:1477), an error in the validation (:1499), a Value before any
//! Name (:1553) and every other call are refused.
//!
//! The reporter's repro of the issue (ListNewLineFactboxesRepro.al, corpus pull request 1517, open: its own page, table,
//! parts and codeunits, line for line, `CodedObjects::Repro`) runs the same way under the assumption
//! `list-new-line-coded-factboxes-repro`, opened and driven only as it is measured: the open and the view above, `New()`, the
//! Name SetValue that lands, one Value SetValue and `OK()` (:611-683).
//!
//! The page with code of al-runner issue 8434 (assumption `list-new-line-coded-factboxes`, corpus
//! pull request 1454, merged, not pinned yet): the probe's pages 97492/97493, their table, three
//! CardPart FactBoxes (one enqueues a page background task), the tables they read and the codeunits
//! they call, line for line (`coded_code` in `code_inserted_pinned.rs`), opened by a Page.RunModal
//! statement on an empty view; or the reporter's repro's page with its objects
//! (`CodedObjects::Repro`, assumption `list-new-line-coded-factboxes-repro`, noted at the open with
//! the assumption every step runs under, `coded_assume`), driven only by the calls its test makes
//! (one later SetValue, then OK). Its facts are `coded_page` (`Triggers::Coded` of
//! `code_inserted_new_line.rs`, which owns the landing and the calls after it); the open lets the
//! parts read the blank record (`coded_open_on_empty_view`, by `sync_parts_at_list_open`), the first
//! SetValue after `New()` lets them read the new line before the validation
//! (`coded_line_before_set_value`, by `test_page_op_held` and `test_control_op_held`), which must
//! then land (`coded_must_have_landed`); the refresh after the landing and after up to two later
//! SetValues reads the row once more in another key than the primary key and spends the reads-done
//! mark (`coded_refresh`, by `code_inserted_refresh`), and the code between the save and the update
//! may write other tables and the row itself (`coded_rows`, read by `view_kept`). A read of a page
//! background task of its parts on a table the test session changed and has not committed is refused
//! (`bg_watched_read` in `background.rs`, by `record_op`).
//!
//! A page of their shape with other code (`coded_extra_shape`, `CodedObjects::Extra`; al-runner issue 8826) takes the same
//! steps under the assumption `list-new-line-extra-code-coded` (ListNewLineExtraCodeCodedPage.al, corpus pull request 1537,
//! open, 7 of 7 green on BC 29.1 in 3 runs). Its page, controls, actions and three parts equal the cells' in every fact but
//! their code (`coded_extra_shape`, `coded_extra_parts`); an OnOpenPage that wrote, or a record handed to the open in a
//! secondary key or an ad hoc sort order, keeps main's way, a part that writes at the open is refused
//! (`coded_open_on_empty_view`); only a first SetValue whose control saves and whose table field inserts
//! takes the steps (`coded_write_may_land`). Then its SaveRecord under the effect gate (`extra_code_coded_cell`), nothing
//! between the save and the Update but a modify of the row through another record (`extra_coded_between_kept`), its
//! refresh reading the row as stored first and refusing a trigger that writes (:773), its worker's reads through the query
//! cache (`bg_drain`), then one SetValue of a control without code and `OK()` (`FactboxStep::CodedExtraTyped`). A row before
//! the line (:821) and a table with record triggers (:839) keep main's way.

use super::pinned_shape::TestCall;
use super::state::{CodedLine, FactboxStep};
use super::*;
use al_builtins::assumptions::assume;
use al_builtins::builtin_id::ids;
use al_builtins::page_op::PageMethod;

/// A step of the page with code `st` runs under the assumption of the objects it showed at its open (`CodedObjects`).
pub(super) fn coded_assume(st: &St) {
    assume(st.borrow().coded_objects.assumption());
}

impl Session {
    /// Whether page `st` is the page with code: the probe's objects line for line (`coded_code`), shown as the probe shows
    /// them: Rec stored; no subscriber to the page or its table but manual ones (:255-274; whether one is bound matters only
    /// from the landing on, where `no_active_subscriber` refuses it, :1592); its three parts shown, each over the host's
    /// table, stored, not marked and filtered by its link only (`coded_part`).
    pub(super) fn coded_page(&self, st: &St) -> Result<bool, Signal> {
        if !self.has_source(st) {
            return Ok(false);
        }
        let manual_only = |object: usize| self.engine().events.manual_subscribers_of(object).is_some();
        let rec = self.page_rec(st);
        let table = rec.borrow().table as usize;
        let parts = st.borrow().parts.clone();
        if !(!rec.borrow().temporary
            && manual_only(st.borrow().object as usize)
            && manual_only(table)
            && st.borrow().hidden_parts.is_empty()
            && parts.len() == 3)
        {
            return Ok(false);
        }
        for (_, part) in &parts {
            if !self.coded_part(st, part)? {
                return Ok(false);
            }
        }
        Ok(self.coded_code(st).is_some() || self.coded_extra_shape(st)?)
    }

    /// Whether host `st`, whose code is not the probe's or the repro's, has the shape of the pages of
    /// ListNewLineExtraCodeCodedPage.al's cells (`CodedObjects::Extra`), every fact by equality with them (review of al-runner
    /// PR 8834, 21:06Z; the repro page :300-409 and the logging page :481-604):
    /// - the page: a List page with no page extension and no subscriber, that declares only PageType, SourceTable,
    ///   AutoSplitKey (as true), ApplicationArea and UsageCategory (`pinned_property`: no SourceTableView, so the view is in
    ///   the primary key), and exactly the triggers OnOpenPage, OnAfterGetRecord and OnDeleteRecord; a table without OnModify
    ///   and OnRename (`no_save_trigger`; OnInsert and OnDelete measured by ListNewLineExtraCodeCodedTableTriggers.al);
    /// - its controls: a repeater without a property; fields of Rec or of a variable with no property but ApplicationArea and
    ///   Editable as false, and no trigger but OnValidate; actions with no property but ApplicationArea and no trigger but
    ///   OnAction; three part controls (`coded_extra_parts`).
    fn coded_extra_shape(&self, st: &St) -> Result<bool, Signal> {
        if !(self.is_list_page(st) && self.unextended_unsubscribed(st) && self.no_save_trigger(st)) {
            return Ok(false);
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let properties = &engine.program.objects[object].properties;
        let split = properties
            .iter()
            .any(|(n, v)| n.eq_ignore_ascii_case("AutoSplitKey") && v.trim().eq_ignore_ascii_case("true"));
        let page = page_of(&engine, st);
        let mut triggers: Vec<&str> = page.triggers.iter().map(|(n, _)| n.as_str()).collect();
        triggers.sort_unstable();
        let host_triggers = triggers == ["onaftergetrecord", "ondeleterecord", "onopenpage"];
        let controls = page.controls.iter().all(|c| {
            let props = |allowed: &[&str]| {
                c.props
                    .iter()
                    .all(|(k, v)| allowed.contains(&k.as_str()) && Self::plain_prop(k, v))
            };
            let triggers = |allowed: &str| c.triggers.iter().all(|(n, _)| n == allowed);
            let own =
                c.owner.is_none() && c.ext_triggers.is_empty() && c.run_object.is_none() && c.action_ref.is_none();
            own && match c.kind {
                ir::ControlKind::Repeater => c.props.is_empty() && c.triggers.is_empty(),
                ir::ControlKind::Field => {
                    matches!(c.binding, ir::Binding::Field(_) | ir::Binding::Var(_))
                        && props(&["applicationarea", "editable"])
                        && triggers("onvalidate")
                }
                ir::ControlKind::Action => props(&["applicationarea"]) && triggers("onaction"),
                ir::ControlKind::Part => c.triggers.is_empty(),
                _ => false,
            }
        });
        let pinned = properties.iter().all(super::code_inserted_new_line::pinned_property);
        drop(engine);
        Ok(split && pinned && host_triggers && controls && self.coded_extra_parts(st)?)
    }

    /// Whether `value` of property `name` of a control is as the cells write it: ApplicationArea and a SubPageLink as written,
    /// Editable only as the literal false (the Kind column, ListNewLineExtraCodeCodedPage.al:329).
    fn plain_prop(name: &str, value: &ir::PropVal) -> bool {
        match name {
            "editable" => matches!(value, ir::PropVal::Lit(v) if v.trim().eq_ignore_ascii_case("false")),
            _ => matches!(value, ir::PropVal::Lit(_)),
        }
    }

    /// Whether the three parts of host `st` are the cells' three FactBoxes in their order (ListNewLineExtraCodeCodedPage.al
    /// :207-298, :411-479): each part control in the FactBoxes area with no property but ApplicationArea and a SubPageLink
    /// that links each field of the primary key to itself (`Session::part_linked_on_key`); each part a CardPart that declares
    /// only PageType and SourceTable, whose controls are fields with no property but ApplicationArea and no trigger, and
    /// actions with no property but ApplicationArea and no trigger but OnAction; the first part's only trigger
    /// OnAfterGetRecord, the second none, the third OnAfterGetCurrRecord, which enqueues a page background task, and
    /// OnPageBackgroundTaskCompleted, and OnPageBackgroundTaskError as well (ListNewLineExtraCodeCodedTaskError.al:135-192).
    fn coded_extra_parts(&self, st: &St) -> Result<bool, Signal> {
        let key = self.table(&self.page_rec(st))?.keys[0].clone();
        let parts = st.borrow().parts.clone();
        let engine = self.engine();
        let page = page_of(&engine, st);
        let wanted: [&[&str]; 3] = [
            &["onaftergetrecord"],
            &[],
            &["onaftergetcurrrecord", "onpagebackgroundtaskcompleted"],
        ];
        if parts.len() != wanted.len() {
            return Ok(false);
        }
        for ((idx, part), want) in parts.iter().zip(wanted) {
            let control = &page.controls[*idx];
            let control_ok = Self::part_linked_on_key(control, &key)
                && control
                    .props
                    .iter()
                    .all(|(k, v)| matches!(k.as_str(), "applicationarea" | "subpagelink") && Self::plain_prop(k, v));
            let part_object = part.borrow().object as usize;
            let part_page = page_of(&engine, part);
            let mut names: Vec<&str> = part_page.triggers.iter().map(|(n, _)| n.as_str()).collect();
            names.sort_unstable();
            let part_controls = part_page.controls.iter().all(|c| {
                let props = c
                    .props
                    .iter()
                    .all(|(k, v)| k == "applicationarea" && Self::plain_prop(k, v));
                let own =
                    c.owner.is_none() && c.ext_triggers.is_empty() && c.run_object.is_none() && c.action_ref.is_none();
                own && props
                    && match c.kind {
                        ir::ControlKind::Field => {
                            matches!(c.binding, ir::Binding::Field(_) | ir::Binding::Var(_)) && c.triggers.is_empty()
                        }
                        ir::ControlKind::Action => c.triggers.iter().all(|(n, _)| n == "onaction"),
                        _ => false,
                    }
            });
            let properties = engine.program.objects[part_object]
                .properties
                .iter()
                .all(|(k, _)| matches!(k.to_ascii_lowercase().as_str(), "pagetype" | "sourcetable"));
            let enqueues = !want.contains(&"onaftergetcurrrecord")
                || part_page
                    .triggers
                    .iter()
                    .find(|(n, _)| n == "onaftergetcurrrecord")
                    .is_some_and(|&(_, r)| {
                        Self::routine_calls(&engine, part_object, r, &[ids::PAGE_ENQUEUEBACKGROUNDTASK_1])
                    });
            if !(control_ok
                && Self::is_card_part(&engine, part_object)
                && properties
                && part_controls
                && (names == want || (want.len() == 2 && names == [want[0], want[1], "onpagebackgroundtaskerror"]))
                && enqueues)
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether part `part` of host `st` shows its rows as the probe's parts do: over the host's table, stored, not marked,
    /// filtered by its link on the primary key only, with no page extension and no subscriber.
    fn coded_part(&self, st: &St, part: &St) -> Result<bool, Signal> {
        if !self.has_source(part) || !self.unextended_unsubscribed(part) {
            return Ok(false);
        }
        let (host_rec, part_rec) = (self.page_rec(st), self.page_rec(part));
        let key = self.table(&host_rec)?.keys[0].clone();
        let (h, p) = (host_rec.borrow(), part_rec.borrow());
        Ok(h.table == p.table
            && !p.temporary
            && !p.marked_only
            && p.filters
                .iter()
                .all(|f| f.group == super::parts::LINK_GROUP && key.contains(&f.field)))
    }

    /// Whether every subscriber to an event of table object `table` is a manual one with no instance bound now (the probe's
    /// :255-275, which its tests do not bind but :1592).
    pub(super) fn no_active_subscriber(&self, table: usize) -> bool {
        let engine = self.engine();
        let subscribers = engine.events.manual_subscribers_of(table);
        drop(engine);
        subscribers.is_some_and(|objects| objects.iter().all(|&o| !self.is_bound_object(o)))
    }

    /// The parts of host `st` as it opens. On the page with code opened as measured on an empty view, they run
    /// OnAfterGetCurrRecord on the blank record (:1407 `open=P.OPEN;W.AGCR();`) and the steps of the page with code begin;
    /// `false` for every other page and open.
    pub(super) fn coded_open_on_empty_view(&mut self, st: &St) -> Result<bool, Signal> {
        let line = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine && s.draft && !s.dirty && !s.draft_stored && s.host.is_none()
        };
        if !(line
            && self.code_inserted_coded_open(st)?
            && self.code_inserted_coded_kind(st)?
            && self.view_row_count(st)? == 0)
        {
            return Ok(false);
        }
        let objects = self.coded_code(st).unwrap_or(super::state::CodedObjects::Extra);
        let extra = objects == super::state::CodedObjects::Extra;
        // On a page of the shape of the page with code, the cells' OnOpenPage and parts write nothing at the open
        // (ListNewLineExtraCodeCodedPage.al:379-382, :773 `OPEN;U.AGCR();`): a page whose OnOpenPage wrote keeps main's way,
        // and a part that writes while it reads the blank record is not measured, and refused (review of al-runner PR 8834,
        // 21:06Z). The cells open the page on a record in the primary key: a record handed to Page.RunModal in a secondary key
        // or an ad hoc sort order keeps main's way too (review of al-runner PR 8834, 02:53Z).
        let primary_key = {
            let rec = self.page_rec(st);
            let r = rec.borrow();
            r.key == 0 && r.adhoc.is_none()
        };
        if extra && (self.store.change_counter() != st.borrow().open_changes || !primary_key) {
            return Ok(false);
        }
        st.borrow_mut().coded_objects = objects;
        coded_assume(st);
        st.borrow_mut().coded_line = CodedLine::EmptyOpen;
        let before = self.store.change_counter();
        self.with_blank_landing(st, |s| s.sync_parts_at_open(st))?;
        if extra && self.store.change_counter() != before {
            st.borrow_mut().coded_line = CodedLine::Off;
            return unsupported(
                "a part of a List page of the shape of the page with code wrote the store when it read the blank record at the \
                 open (not measured)",
            );
        }
        Ok(true)
    }

    /// The test's `call` on host `st`, before it runs. After the open of `coded_open_on_empty_view`, the first SetValue of a
    /// control of the host after `New()` lets the parts read the new line first, OnAfterGetCurrRecord on the blank record (:1407
    /// `setname=W.AGCR();T.VN(Station);...`), and the validation must then land (`coded_must_have_landed`); any other call
    /// ends the steps. After a validation error before the landing (`CodedLine::Failed`) every call on the page or its parts is
    /// refused: the cell's error leaves the handler (ListNewLineExtraCodeCodedTableTriggers.al:746).
    pub(super) fn coded_line_before_set_value(&mut self, st: &St, call: Option<TestCall>) -> Result<(), Signal> {
        if call.is_some() && Self::tree_root(st).0.borrow().coded_line == CodedLine::Failed {
            return unsupported(
                "a call on a List page with code and FactBoxes after the validation of its first SetValue raised an error before \
                 the landing (not measured)",
            );
        }
        if st.borrow().coded_line != CodedLine::EmptyOpen {
            return Ok(());
        }
        let Some(call) = call else {
            return Ok(());
        };
        let first_write = matches!(call, TestCall::Write(_))
            && matches!(
                Self::test_calls(st).as_deref(),
                Some([TestCall::Page(PageMethod::New), TestCall::Write(_)])
            )
            && {
                let s = st.borrow();
                s.cursor == Cursor::NewLine && s.draft && !s.dirty && !s.draft_stored && s.new_by_call
            };
        match call {
            // On a page of the shape of the page with code, only a write whose validation can land as the cells' do takes the
            // steps: the control's OnValidate calls CurrPage.SaveRecord() and its table field's OnValidate inserts
            // (ListNewLineExtraCodeCodedPage.al:31-36, :318-324). Every other first write keeps main's way (review of al-runner
            // PR 8834, 21:06Z, point 1).
            TestCall::Write(i) if first_write && !self.coded_write_may_land(st, i) => {
                st.borrow_mut().coded_line = CodedLine::Off;
                Ok(())
            }
            _ if first_write => {
                coded_assume(st);
                st.borrow_mut().coded_line = CodedLine::LineRead;
                self.with_blank_landing(st, |s| s.sync_parts(st))
            }
            TestCall::Read | TestCall::Page(PageMethod::New) => Ok(()),
            _ => {
                st.borrow_mut().coded_line = CodedLine::Off;
                Ok(())
            }
        }
    }

    /// Whether the first write of control `idx` of host `st` may land as the cells' does: on the probe's and the repro's pages
    /// always (their code is compared line for line); on a page of their shape (`CodedObjects::Extra`) when the control's
    /// OnValidate calls `CurrPage.SaveRecord()` itself and the OnValidate of the table field it shows calls Insert itself
    /// (`routine_calls`).
    fn coded_write_may_land(&self, st: &St, idx: usize) -> bool {
        if st.borrow().coded_objects != super::state::CodedObjects::Extra {
            return true;
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let Some(control) = page_of(&engine, st).controls.get(idx) else {
            return false;
        };
        let ir::Binding::Field(field) = control.binding else {
            return false;
        };
        let saves = control
            .triggers
            .iter()
            .find(|(n, _)| n == "onvalidate")
            .is_some_and(|&(_, r)| Self::routine_calls(&engine, object, r, &[ids::PAGE_SAVERECORD_1]));
        let table = self.page_rec(st).borrow().table as usize;
        let inserts = engine.program.objects[table]
            .table
            .as_ref()
            .and_then(|t| t.fields.get(field))
            .and_then(|f| f.on_validate)
            .is_some_and(|(o, r)| {
                Self::routine_calls(
                    &engine,
                    o,
                    r,
                    &[ids::RECORD_INSERT_1, ids::RECORD_INSERT_2, ids::RECORD_INSERT_3],
                )
            });
        saves && inserts
    }

    /// Whether routine `routine` of object `object` calls one of the builtins `wanted` itself, in any statement or expression
    /// (`for_each_expr_post`); a call in a procedure it calls does not count.
    fn routine_calls(
        engine: &crate::Engine,
        object: usize,
        routine: usize,
        wanted: &[al_builtins::builtin_id::BuiltinId],
    ) -> bool {
        let mut found = false;
        al_ir::types::for_each_expr_post(
            engine.program.objects[object].routines[routine].body.stmts(),
            &mut |e| {
                found |= matches!(e, ir::Expr::Builtin { id, .. } if wanted.contains(id));
            },
        );
        found
    }

    /// Whether error `e` of the first SetValue's validation on host `st`, a page of the page-with-code shape with other code, came
    /// out of code the cells measure raising it, by its stack, innermost first: its table's OnInsert at the insert the landing
    /// control's table field makes in its OnValidate (ListNewLineExtraCodeCodedTableTriggers.al:746, two frames); that table
    /// field's OnValidate itself, a TestField before the insert or an error after it (:1249, :1259); the landing control's own
    /// OnValidate before its save (:1269), or with the table field skipping its insert before or after the control's plain
    /// save (:1674, :1684; `PageState::control_failed_as_measured`, review of al-runner PR 8840, 07:30Z). Any other stack
    /// keeps main's way (review of al-runner PR 8840, 01:32Z).
    pub(super) fn coded_error_measured(&self, st: &St, e: &al_builtins::native::AlError) -> bool {
        let landing = match Self::test_calls(st).as_deref() {
            Some([TestCall::Page(PageMethod::New), TestCall::Write(landing)]) => *landing,
            _ => return false,
        };
        let engine = self.engine();
        let Some(control) = page_of(&engine, st).controls.get(landing) else {
            return false;
        };
        let ir::Binding::Field(field) = control.binding else {
            return false;
        };
        let page = st.borrow().object as usize;
        let control_validate = control
            .trigger("onvalidate")
            .and_then(|r| crate::callstack::frame_head(&engine.program, page, r));
        let table = self.page_rec(st).borrow().table as usize;
        let routines = &engine.program.objects[table].routines;
        let head = |found: Option<usize>| found.and_then(|r| crate::callstack::frame_head(&engine.program, table, r));
        let on_insert = head(
            routines
                .iter()
                .position(|r| r.is_trigger && r.field.is_none() && r.name.eq_ignore_ascii_case("OnInsert")),
        );
        let field_validate = head(
            routines
                .iter()
                .position(|r| r.is_trigger && r.field == Some(field) && r.name.eq_ignore_ascii_case("OnValidate")),
        );
        let is =
            |line: &str, head: &Option<String>| head.as_ref().is_some_and(|h| line.starts_with(&format!("{h} line ")));
        match e.call_stack.lines().collect::<Vec<_>>().as_slice() {
            [inner, outer] => is(inner, &on_insert) && is(outer, &field_validate),
            [only] => {
                is(only, &field_validate) || (is(only, &control_validate) && st.borrow().control_failed_as_measured)
            }
            _ => false,
        }
    }

    /// Whether the validation of the SetValue that let the parts read the new line has landed on its row: before the landing an
    /// error (:1499) or a validation that does not land (:1553, Value before any Name) is not measured, and refused.
    pub(super) fn coded_must_have_landed(&self, st: &St) -> Result<(), Signal> {
        if st.borrow().coded_line != CodedLine::LineRead {
            return Ok(());
        }
        st.borrow_mut().forget_code_inserted_line();
        unsupported(
            "the validation of the first SetValue on a List page with code and FactBoxes, whose parts read its new line, did not \
             leave the line for the row its table field inserted as measured (not measured)",
        )
    }

    /// A SetValue on the tree of host `st` ended with `result`. When it was the first SetValue that let the parts read the new
    /// line (`CodedLine::LineRead`), or a SetValue on the landed row (`FactboxStep::CodedTyped`), and it raised an error before
    /// or in its validation, or ended without the landing or the refresh it must ask for, it is not measured, and refused.
    pub(super) fn coded_after_set_value(&self, st: &St, result: Result<Value, Signal>) -> Result<Value, Signal> {
        let (root, _) = Self::tree_root(st);
        let pending = {
            let r = root.borrow();
            r.coded_line == CodedLine::LineRead
                || matches!(
                    r.code_inserted_row.as_ref().map(|c| c.factbox),
                    Some(FactboxStep::CodedTyped { .. })
                )
        };
        match result {
            Err(Signal::Error(_)) | Ok(_) if pending => {
                root.borrow_mut().forget_code_inserted_line();
                unsupported(
                    "a SetValue on a List page with code and FactBoxes, on its new line or on the row it became, ended without the \
                     landing or the save and update the probe measures (not measured)",
                )
            }
            r => r,
        }
    }

    /// Whether control `idx` of host `st` takes a SetValue on the row its new line became (ListNewLineFactboxesCode.al:1407
    /// `setvalue`, :1576 `setvalue2`): a field control over Rec outside the primary key, with a TableRelation and an OnValidate
    /// of its own, other than the control whose SetValue landed.
    pub(super) fn coded_value_control(&self, st: &St, idx: usize) -> bool {
        let landing = match Self::test_calls(st).as_deref() {
            Some([TestCall::Page(PageMethod::New), TestCall::Write(landing), ..]) => *landing,
            _ => return false,
        };
        let engine = self.engine();
        let Some(control) = page_of(&engine, st).controls.get(idx) else {
            return false;
        };
        let ir::Binding::Field(f) = control.binding else {
            return false;
        };
        idx != landing
            && control.trigger("onvalidate").is_some()
            && control.props.iter().any(|(k, _)| k == "tablerelation")
            && self.table(&self.page_rec(st)).is_ok_and(|t| !t.keys[0].contains(&f))
    }

    /// The refresh of the page with code after its new line landed, or after a later SetValue saved the row
    /// (`code_inserted_refresh`): the row once more first in a secondary key (:1407 `P.AGR(Station/);P.AGR(Station/);`, :1445
    /// once in the primary key), then as on the FactBox host. The page's and the parts' triggers leave the rows of the page's
    /// table, Rec as stored and Rec's view as they were; otherwise not measured, and refused.
    pub(super) fn coded_refresh(&mut self, st: &St, without_curr: bool, without_parts: bool) -> Result<(), Signal> {
        let rows = self.coded_rows(st)?;
        let view = self.page_rec(st).borrow().clone();
        let primary_key = {
            let rec = self.page_rec(st);
            let r = rec.borrow();
            r.key == 0 && r.adhoc.is_none()
        };
        if !primary_key {
            self.populate_current(st)?;
        }
        let extra = st.borrow().coded_objects == super::state::CodedObjects::Extra;
        if extra {
            // The Update reads the row as stored before the page's OnAfterGetRecord runs on it: the first read shows a write
            // made through another record after the save (ListNewLineExtraCodeCodedPage.al:773 `AGR(A/Resolved/Stamped)`).
            let rec = self.page_rec(st);
            let mut pk = self.table(&rec)?.primary_key(&rec.borrow().fields);
            self.ret_used = true;
            self.record_op(al_builtins::record_op::RecordOp::Get, &rec, &mut pk)?;
        }
        let changes = self.store.change_counter();
        self.reread_view_from_row(st, without_curr, without_parts, false)?;
        // The refresh's reads are its own: on this page without OnAfterGetCurrRecord the mark they leave for the refresh
        // (`note_reads_done`) is spent here, so it does not skip the refresh of the next SetValue (:1407 `setvalue`). Only here,
        // under the assumption: what the mark does on the other callers of `reread_view_from_row` is not measured (review of
        // al-runner PR 8463, round 1, finding 3).
        st.borrow_mut().refresh.spend_reads_done();
        let rec = self.page_rec(st);
        let stored = self.stored_row(&rec)?;
        let rows_kept = match (rows, self.coded_rows(st)?) {
            (Some(a), Some(b)) => a.len() == b.len() && a.iter().zip(&b).all(|(x, y)| Self::same_values(x, y)),
            _ => false,
        };
        let view_kept = Self::same_view(&rec.borrow(), &view);
        // On a page of the probe's shape with other code the triggers only read: one that wrote the store (a part's write of
        // another table) is not measured either.
        let quiet = !extra || self.store.change_counter() == changes;
        let kept =
            quiet && rows_kept && view_kept && stored.is_some_and(|s| Self::same_values(&rec.borrow().fields, &s));
        if !kept {
            st.borrow_mut().forget_code_inserted_line();
            return unsupported(
                "a trigger of a List page with code and FactBoxes, or of its parts, wrote or changed Rec when the refresh read the \
                 row its new line became (not measured)",
            );
        }
        let steps = match st.borrow().code_inserted_row.as_ref().map(|r| r.factbox) {
            Some(FactboxStep::CodedTyped { steps }) => steps + 1,
            _ => 0,
        };
        if let Some(row) = st.borrow_mut().code_inserted_row.as_mut() {
            row.factbox = FactboxStep::Coded { steps };
        }
        Ok(())
    }

    /// Every stored row of the table of the page's record, in the primary key: what the code between the save and the update
    /// must leave as it is on the page with code (`SavedView::rows`). `None` past 1000 rows.
    pub(super) fn coded_rows(&mut self, st: &St) -> Result<Option<Vec<Vec<Value>>>, Signal> {
        let probe = Rc::new(RefCell::new(self.page_rec(st).borrow().clone()));
        {
            let mut p = probe.borrow_mut();
            p.filters.clear();
            p.marks.clear();
            p.marked_only = false;
            p.key = 0;
            p.ascending = true;
            p.adhoc = None;
            p.field_descending = 0;
        }
        let rows = self.view_rows(&probe, 1001)?;
        Ok((rows.len() <= 1000).then_some(rows))
    }
}
