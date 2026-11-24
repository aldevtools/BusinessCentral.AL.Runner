//! A List page on the line `New()` started, whose table field's OnValidate inserted Rec itself (`if "Entry No." = 0 then
//! Insert(true)`, an AutoIncrement key field) before the control's OnValidate calls `CurrPage.Update()` or `CurrPage.Update(false)`
//! (al-runner issue 8186). BC treats the line as the stored row from then on: `Update()` modifies it (OnModifyRecord and the
//! table's OnModify run inside the OnValidate, OnInsertRecord never runs), `Update(false)` saves nothing, and both refresh onto
//! that row; the next SetValue lands on it. Pinned by pages/currpage/ListNewLineCodeInsertUpdate.al:515-617, :783-810.
//! `CurrPage.Update(true)` written out, and `CurrPage.SaveRecord()` followed by `CurrPage.Update(false)`, do what `Update()` does
//! (SaveRecord modifies the row, the Update(false) after it asks the same refresh; al-runner issue 8285): pinned by
//! pages/currpage/ListNewLineSaveRecordUpdate.al:429-874, for the probe's code only, line for line (`code_inserted_pinned.rs`;
//! the reporter's repro under an assumption); on the page without triggers other code runs under the effect gate of
//! `code_inserted_extra.rs`. The entry condition is `code_inserted_cell`: the facts every pinned cell shares,
//! then the cells themselves. A validation that ends with an error after the page moved onto the row is refused
//! (`refuse_error_after_landing`), and so is one whose SaveRecord left the line without the `Update(false)` after it
//! (`code_inserted_step_end`), and the host with two CardPart FactBoxes (ListNewLineCodeInsertUpdate.al:249-299) at its open
//! (`refuse_code_inserted_parts_host`): BC's part triggers there (:718, :736) are not the runner's.
//!
//! A List page's `New()` line that the table field's OnValidate inserted itself, then
//! `CurrPage.Update()`, `Update(false)`, `Update(true)` or `SaveRecord()` then `Update(false)` in the
//! control's OnValidate: the page lands on the stored row (Update(), Update(true) and SaveRecord
//! modify it), the refresh reads the view from its first row, and `Previous()` on the view's only row
//! answers false without a save (al-runner issues 8186 and 8285, pinned by
//! pages/currpage/ListNewLineCodeInsertUpdate.al and ListNewLineSaveRecordUpdate.al). On a host with
//! one CardPart FactBox (SaveRecord then Update(false)) the refresh leaves out the last read of the
//! row, and with a row before, Previous() and Close/OK read the row again before they save it; any
//! other call that leaves the row there is refused. Only the pinned cells take it
//! (`code_inserted_cell`: page properties and triggers, the FactBox (`code_inserted_factbox`), open,
//! calls, key, view); issue 8186's `Update()` under a RunModal filtered to one group is refused
//! there, every other shape keeps the earlier behavior, a validation that ends with an error after
//! the move, or whose SaveRecord is not followed by Update(false), is refused
//! (`refuse_error_after_landing`, `code_inserted_step_end`), and the open of the probe's host with
//! two CardPart FactBoxes is refused (`refuse_code_inserted_parts_host`, by
//! `sync_parts_at_list_open`). Entry points: `land_on_code_inserted_line` (by `CurrPage.Update`),
//! `save_record_on_code_inserted_line` (by `CurrPage.SaveRecord`), `code_inserted_saved` (by
//! `save_from_update`: notes Rec, its view and the store after the save, which the later calls must
//! keep), `run_control_onvalidate`, `refuse_error_after_landing` and `code_inserted_step_end` (by the
//! SetValue's validation), `code_inserted_refresh_due` and `code_inserted_refresh` (by
//! `run_refresh`), `previous_on_code_inserted_row` (by `end_move_without_read`),
//! `code_inserted_row_at_close` (by `close_with`), `refuse_off_factbox_row` (by `test_page_op_held`
//! and `test_control_op_held`: on the FactBox host only reads, one SetValue of a control without
//! code, then Previous, Close or OK); the refresh is `reread_view_from_row` in `refresh_new_line.rs`.

use super::code_inserted_pinned::PinnedCode;
use super::pinned_shape::TestCall;
use super::state::{CodeInsertedRow, CodeInsertedSave, CodedLine, FactboxStep, SavedView};
use super::*;
use al_builtins::filter::FilterExpr;
use al_builtins::page_op::PageMethod;

/// The page's own triggers, as a measured page declares them (ListNewLineCodeInsertUpdate.al).
#[derive(Clone, Copy)]
enum Triggers {
    /// OnNewRecord, OnInsertRecord, OnModifyRecord, OnAfterGetRecord and OnAfterGetCurrRecord: pages 97032 (:136-162) and 97033
    /// (:199-225).
    Row,
    /// None: page 97039 (:345-372).
    Quiet,
    /// OnAfterGetRecord only, and one CardPart FactBox (`code_inserted_factbox`): ListNewLineSaveRecordUpdate.al page 97474
    /// (:210-254), and the partner app's page in the repro of al-runner issue 8285.
    FactboxHost,
    /// Code in OnOpenPage, OnAfterGetRecord and OnDeleteRecord, control expressions, actions, and up to three CardPart
    /// FactBoxes with triggers (`code_inserted_coded.rs`): ListNewLineFactboxesCode.al pages 97492 and 97493 (corpus pull
    /// request 1454), and the partner app's page in the repro of al-runner issue 8434.
    Coded,
    /// OnAfterGetRecord and OnAfterGetCurrRecord, and one CardPart FactBox whose OnAfterGetRecord sets a variable a field shows
    /// (`code_inserted_factbox` with code): ListNewLineExtraCodeFactbox.al pages 97694 (:101-164) and 97698 (:217-296), for
    /// code that is not a pinned page's only (`code_inserted_extra.rs`).
    FactboxExtra,
}

/// How the page was opened, with the view it shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Opened {
    /// The test's `TestPage.OpenEdit()`, no filter (:520, :532, :544, :556, :568, :580, :790, :805).
    OpenEdit,
    /// AL code's `Page.RunModal(Id, Rec)` as a statement, in Edit mode, its record without a filter; the test's ModalPageHandler
    /// gets the page (:232-237, :610-617).
    RunModal,
    /// The same, its record filtered with `SetRange` to one value of the key's first field: ListNewLineSaveRecordUpdate.al:272-281
    /// (the opener) with a row of another group outside the view. The repro of al-runner issue 8186 (`CurrPage.Update()` on the
    /// page without triggers opened so) is refused (`code_inserted_cell`).
    RunModalOneGroup,
    /// The test's `TestPage.OpenEdit()`, then a filter of the test on the key's first field to the line's value (`TestPage.Filter.
    /// SetFilter`): measured only for code that is not a pinned page's (ListNewLineExtraCode.al:372, `code_inserted_extra.rs`).
    OpenEditOneGroup,
}

/// The refusal of a SaveRecord or Update(true) cell whose code is not the measured code (`code_inserted_pinned.rs`), and not
/// within the effect gate of `code_inserted_extra.rs`.
pub(super) const UNMEASURED_CODE: &str = "CurrPage.SaveRecord or Update(true) on the new line whose table field inserted it, on a page \
     whose code, or the code of its table, its part or what they call, is not the code ListNewLineSaveRecordUpdate.al or the \
     repro of al-runner issue 8285 runs (not measured)";

/// The measured view the inserted row stands in (`code_inserted_view`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum View {
    /// The row alone.
    Alone,
    /// The row last, after one row of its group.
    AfterRow,
}

/// How the row Rec is came to be (`inserted_by_table_field`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Inserted {
    /// The table field's OnValidate inserted it before the control's OnValidate began.
    ByTableField,
    /// The same, then the control's OnValidate wrote it once through another record, and nothing else.
    Rewritten,
}

/// The page call that leaves the line, as the control's OnValidate makes it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Leave {
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

/// A page property the pinned pages declare: PageType, SourceTable, AutoSplitKey as true, ApplicationArea and UsageCategory
/// (:105-109, :169-172, :251-255, :347-351).
pub(super) fn pinned_property((name, value): &(String, String)) -> bool {
    match name.to_ascii_lowercase().as_str() {
        "autosplitkey" => value.trim().eq_ignore_ascii_case("true"),
        "pagetype" | "sourcetable" | "applicationarea" | "usagecategory" => true,
        _ => false,
    }
}

impl Session {
    /// Whether part control `control` is a FactBox linked by a SubPageLink of `field(..)` entries that link each field of
    /// primary key `key` to itself, in key order, with no SubPageView and no Provider (the measured FactBoxes:
    /// ListNewLineCodeInsertUpdate.al:282, ListNewLineSaveRecordUpdate.al:242, ListNewLineExtraCodeCodedPage.al:337).
    pub(super) fn part_linked_on_key(control: &ir::Control, key: &[usize]) -> bool {
        control.factbox
            && control.part_link.as_ref().is_some_and(|l| {
                l.provider.is_none()
                    && l.view.is_none()
                    && l.entries.len() == key.len()
                    && l.entries
                        .iter()
                        .zip(key)
                        .all(|(e, k)| matches!(e, ir::LinkEntry::Field(pf, hf) if pf == k && hf == k))
            })
    }

    /// Whether page object `object` is a CardPart.
    pub(super) fn is_card_part(engine: &crate::Engine, object: usize) -> bool {
        engine.object_facts[object].page_type == Some(ir::PageType::Other("cardpart".into()))
    }

    /// The open of the host of ListNewLineCodeInsertUpdate.al:249-299 (page 97036), every fact by equality: a List page
    /// (:251) over a stored table, unextended and unsubscribed, that declares only the pinned properties with AutoSplitKey
    /// (:251-255, `pinned_property`), whose only trigger is OnAfterGetRecord (:292-295), with exactly two part controls
    /// (:279-288), each a FactBox showing a CardPart over the host's table (:303-304, :331-332) linked by a SubPageLink of two
    /// `field(..)` entries, each a field of the table's two-field primary key to itself (:282, :287; the key :23-27). BC runs
    /// the parts' OnAfterGetCurrRecord at the open of an empty view and keeps them on the row before at `New()` (:736
    /// `open=F.AGCR();`, :718 `newparts=A/`); the runner does not: refused. Every other page keeps main's open.
    pub(super) fn refuse_code_inserted_parts_host(&self, st: &St) -> Result<(), Signal> {
        if !(self.is_list_page(st) && self.has_source(st)) {
            return Ok(());
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let host_rec = self.page_rec(st);
        let table = self.table(&host_rec)?;
        let key = &table.keys[0];
        let triggers = matches!(page.triggers.as_slice(), [(n, _)] if n == "onaftergetrecord");
        let split = engine.program.objects[object]
            .properties
            .iter()
            .any(|(n, v)| n.eq_ignore_ascii_case("AutoSplitKey") && v.trim().eq_ignore_ascii_case("true"));
        let shape = self.unextended_unsubscribed(st)
            && engine.program.objects[object].properties.iter().all(pinned_property)
            && split
            && triggers
            && !host_rec.borrow().temporary
            && key.len() == 2;
        if !shape {
            return Ok(());
        }
        let part_controls: Vec<&ir::Control> = page
            .controls
            .iter()
            .filter(|c| c.kind == ir::ControlKind::Part)
            .collect();
        let parts = st.borrow().parts.clone();
        let card_parts = parts.len() == 2
            && parts.iter().all(|(idx, part)| {
                engine.object_facts[part.borrow().object as usize].page_type_lower() == "cardpart"
                    && self.has_source(part)
                    && self.page_rec(part).borrow().table == host_rec.borrow().table
                    && Self::part_linked_on_key(&page.controls[*idx], key)
            });
        if part_controls.len() == 2 && card_parts {
            return Err(Signal::unsupported(
                "the open of a List page with AutoSplitKey and two CardPart FactBoxes linked to its two-field key: BC runs the \
                 parts' OnAfterGetCurrRecord at the open of an empty view and keeps them on the row before at New() \
                 (ListNewLineCodeInsertUpdate.al:718, :736), the runtime does not",
            ));
        }
        Ok(())
    }

    /// Whether a call `leave` on `st` is a pinned cell: a pinned page (`code_inserted_page`) opened as pinned
    /// (`code_inserted_open`), on the line `New()` started (`code_inserted_line`), whose table field inserted the line
    /// (`inserted_by_table_field`), in a pinned view (`code_inserted_view`). Every other shape keeps main's way, except the repro
    /// of al-runner issue 8186, which is refused.
    fn code_inserted_cell(&mut self, st: &St, leave: Leave) -> Result<bool, Signal> {
        let Some((triggers, split)) = self.code_inserted_page(st)? else {
            return Ok(false);
        };
        let Some((opened, primary_key)) = self.code_inserted_open(st)? else {
            return Ok(false);
        };
        let coded = matches!(triggers, Triggers::Coded);
        // Only the page with code is measured in another key than the primary key (`code_inserted_coded.rs`).
        if !primary_key && !coded {
            return Ok(false);
        }
        // (triggers, AutoSplitKey, open, call) of each cell.
        let cell = matches!(
            (triggers, split, opened, leave),
            // ListNewLineCodeInsertUpdate.al, page 97032: Update() :515-536, Update(false) :563-584, RunModal with Update()
            // :588-617.
            (Triggers::Row, true, Opened::OpenEdit, Leave::Update | Leave::UpdateFalse)
                | (Triggers::Row, true, Opened::RunModal, Leave::Update)
                // Page 97033: :539-560.
                | (Triggers::Row, false, Opened::OpenEdit, Leave::Update)
                // Page 97039: :783-810.
                | (Triggers::Quiet, true, Opened::OpenEdit, Leave::Update)
                // ListNewLineSaveRecordUpdate.al, page 97472 (:105-170): :429-559, :739; page 97473 (:172-208): :561-675, :757-782.
                | (
                    Triggers::Row | Triggers::Quiet,
                    true,
                    Opened::OpenEdit | Opened::RunModalOneGroup,
                    Leave::UpdateTrue | Leave::SaveRecord,
                )
                // ListNewLineSaveRecordUpdate.al, page 97474: :677-737, :784-874.
                | (Triggers::FactboxHost, true, Opened::OpenEdit | Opened::RunModalOneGroup, Leave::SaveRecord)
                // ListNewLineExtraCode.al:372, for code that is not a pinned page's only (`code_inserted_extra.rs`).
                | (Triggers::Quiet, true, Opened::OpenEditOneGroup, Leave::SaveRecord)
                // ListNewLineExtraCodeFactbox.al, for code that is not a pinned page's only (`code_inserted_extra.rs`).
                | (Triggers::FactboxExtra, true, Opened::RunModalOneGroup, Leave::SaveRecord)
        );
        // The repro of issue 8186: page 97039's shape opened by a RunModal filtered to one group. The corpus opens that page
        // with OpenEdit only for Update() (ListNewLineCodeInsertUpdate.al:783-810), and main's way is the result the issue
        // reports as wrong: refused.
        let repro = matches!(
            (triggers, split, opened, leave),
            (Triggers::Quiet, true, Opened::RunModalOneGroup, Leave::Update)
        );
        // ListNewLineFactboxesCode.al (corpus pull request 1454, merged, not pinned yet): :1401-1476, :1516-1591, once the parts read the new line before
        // the validation (`coded_line_before_set_value`).
        let coded_cell = coded
            && matches!(
                (split, opened, leave),
                (true, Opened::RunModalOneGroup, Leave::SaveRecord)
            )
            && st.borrow().coded_line == CodedLine::LineRead;
        if !(cell || coded_cell || repro) {
            return Ok(false);
        }
        let page_object = st.borrow().object as usize;
        let inserted = match self.code_inserted_line(st, opened == Opened::OpenEditOneGroup) {
            true => self.inserted_by_table_field(st, coded)?,
            false => None,
        };
        // Only the effect gate takes a row the control's OnValidate wrote once before the save (`code_inserted_extra.rs`).
        let rewritten = inserted == Some(Inserted::Rewritten);
        let view = match inserted {
            Some(Inserted::ByTableField) => self.code_inserted_view(st, coded)?,
            Some(Inserted::Rewritten) if matches!(triggers, Triggers::Quiet) && leave == Leave::SaveRecord => {
                self.code_inserted_view(st, coded)?
            }
            _ => None,
        };
        let shape = view.is_some() && (!coded || self.no_active_subscriber(page_object));
        // A page of the shape of the page with code, but not its code: the effect gate (`extra_code_coded_cell`).
        if shape && coded && st.borrow().coded_objects == super::state::CodedObjects::Extra {
            self.extra_code_coded_cell(st, opened, view, leave)?;
        }
        if shape && coded {
            super::code_inserted_coded::coded_assume(st);
            st.borrow_mut().coded_line = CodedLine::Landed;
        }
        if shape && repro {
            return Err(Signal::unsupported(
                "CurrPage.Update on the new line whose table field inserted it, on a List page without triggers opened by \
                 Page.RunModal with a filter on the key's first field (ListNewLineCodeInsertUpdate.al pins OpenEdit only, :783-810)",
            ));
        }
        // The SaveRecord and Update(true) cells run only the code that was measured (`code_inserted_pinned.rs`).
        if shape && cell && matches!(leave, Leave::UpdateTrue | Leave::SaveRecord) {
            match self.code_inserted_code(st)? {
                Some(PinnedCode::Probe) if opened != Opened::OpenEditOneGroup && !rewritten => {}
                // The repro is measured only when Page.RunModal opens it on one group with one row before the new line
                // (Issue8285.al:146-211); OpenEdit and an empty group are not (review of #8305, round 4).
                Some(PinnedCode::Repro)
                    if opened == Opened::RunModalOneGroup && view == Some(View::AfterRow) && !rewritten =>
                {
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ListNewLineSaveRecordRepro)
                }
                // Other code on the page without triggers: the effect gate of the cells of al-runner issues 8712 and 8713; on the
                // page with triggers and a FactBox, of al-runner issue 8734.
                None if matches!(triggers, Triggers::Quiet) => {
                    self.extra_code_cell(st, opened, view, leave, rewritten, false)?
                }
                None if matches!(triggers, Triggers::FactboxExtra) => {
                    self.extra_code_cell(st, opened, view, leave, rewritten, true)?
                }
                Some(PinnedCode::Probe | PinnedCode::Repro) | None => return Err(Signal::unsupported(UNMEASURED_CODE)),
            }
        }
        Ok(shape)
    }

    /// The page's triggers and whether it declares AutoSplitKey, for a measured page: PageType List (`is_list_page`, :105) over a
    /// database table, with no page extension and no event subscriber, that declares only PageType, SourceTable, AutoSplitKey (as
    /// true), ApplicationArea and UsageCategory (:105-109, :169-172, :347-351; ListNewLineSaveRecordUpdate.al
    /// :105-111, :172-178, :210-216), and the triggers of `Triggers`: no part, except the one FactBox of `Triggers::FactboxHost`.
    fn code_inserted_page(&self, st: &St) -> Result<Option<(Triggers, bool)>, Signal> {
        let (shape, split, triggers, no_parts) = {
            let engine = self.engine();
            let object = st.borrow().object as usize;
            let page = page_of(&engine, st);
            let properties = &engine.program.objects[object].properties;
            let split = properties.iter().any(|(n, _)| n.eq_ignore_ascii_case("AutoSplitKey"));
            let shape = self.is_list_page(st)
                && self.has_source(st)
                && self.unextended_unsubscribed(st)
                && properties.iter().all(pinned_property)
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
            [only] if only == "onaftergetrecord" && self.code_inserted_factbox(st, false)? => Triggers::FactboxHost,
            [a, b]
                if [a, b].iter().any(|n| n.as_str() == "onaftergetrecord")
                    && [a, b].iter().any(|n| n.as_str() == "onaftergetcurrrecord")
                    && self.code_inserted_factbox(st, true)? =>
            {
                Triggers::FactboxExtra
            }
            _ if self.coded_page(st)? => return Ok(Some((Triggers::Coded, split))),
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
    fn code_inserted_factbox(&self, st: &St, with_code: bool) -> Result<bool, Signal> {
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
        let link = Self::part_linked_on_key(control, &key);
        let part_control = control
            .props
            .iter()
            .all(|(k, _)| k == "applicationarea" || k == "subpagelink");
        let card_part = Self::is_card_part(&engine, part_object);
        let part_properties = engine.program.objects[part_object].properties.iter().all(|(k, _)| {
            matches!(
                k.to_ascii_lowercase().as_str(),
                "pagetype" | "sourcetable" | "applicationarea" | "usagecategory"
            )
        });
        let plain = |controls: &[ir::Control], part_at: Option<usize>, vars: bool| {
            controls.iter().enumerate().all(|(i, c)| match c.kind {
                ir::ControlKind::Field => {
                    (matches!(c.binding, ir::Binding::Field(_)) || vars && matches!(c.binding, ir::Binding::Var(_)))
                        && c.owner.is_none()
                        && c.ext_triggers.is_empty()
                        && c.props.iter().all(|(k, _)| k == "applicationarea")
                }
                ir::ControlKind::Repeater => c.props.is_empty(),
                ir::ControlKind::Part => part_at == Some(i),
                _ => false,
            })
        };
        // The part of `Triggers::FactboxExtra` declares OnAfterGetRecord, and a field shows a variable of its own
        // (ListNewLineExtraCodeFactbox.al:75-99, :189-215); the part of `Triggers::FactboxHost` declares no trigger.
        let part_triggers = match with_code {
            true => matches!(part_page.triggers.as_slice(), [(n, _)] if n == "onaftergetrecord"),
            false => part_page.triggers.is_empty(),
        };
        Ok(same_table
            && link
            && part_control
            && card_part
            && part_properties
            && part_triggers
            && part_page.controls.iter().all(|c| c.triggers.is_empty())
            && plain(&host_page.controls, Some(*idx), false)
            && plain(&part_page.controls, None, with_code))
    }

    /// How `st` was opened, for a measured open: the test's OpenEdit (`PageState::test_open_edit`) on a view without filters, or
    /// a static `Page.RunModal(Id, Rec)` statement in Edit mode without filters or with one `SetRange` on the key's first field
    /// to the line's value in group 0; and whether the view is in the primary key (not another key, not an ad hoc sort such as
    /// a SourceTableView that sorts on a field no key begins with). The view is ascending and not limited to marked rows.
    fn code_inserted_open(&mut self, st: &St) -> Result<Option<(Opened, bool)>, Signal> {
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
        let sorted = r.ascending && r.field_descending == 0 && !r.marked_only;
        let one_group = |f: &al_builtins::filter::FieldFilter| {
            let line = &r.fields[f.field..=f.field];
            f.group == 0
                && table.keys[0].first() == Some(&f.field)
                && matches!(&f.expr, FilterExpr::Range(Some(a), Some(b))
                    if Self::same_values(std::slice::from_ref(a), line) && Self::same_values(std::slice::from_ref(b), line))
        };
        // The test's `Filter.SetFilter(Field, Value)` on the key's first field to the line's value (ListNewLineExtraCode.al:376).
        let test_one_group = |f: &al_builtins::filter::FieldFilter| {
            let line = &r.fields[f.field..=f.field];
            f.group == 0
                && table.keys[0].first() == Some(&f.field)
                && matches!(&f.expr, FilterExpr::Eq(v) if Self::same_values(std::slice::from_ref(v), line))
        };
        let opened = match (open_edit, run_modal, r.filters.as_slice()) {
            (true, false, []) => Opened::OpenEdit,
            (true, false, [f]) if test_one_group(f) => Opened::OpenEditOneGroup,
            (false, true, []) => Opened::RunModal,
            (false, true, [f]) if one_group(f) => Opened::RunModalOneGroup,
            _ => return Ok(None),
        };
        Ok(sorted.then_some((opened, r.key == 0 && r.adhoc.is_none())))
    }

    /// Whether `st` is the page with code (`Triggers::Coded`), not one of the pages the other kinds take first (the FactBox host
    /// also has the facts of the page with code).
    pub(super) fn code_inserted_coded_kind(&self, st: &St) -> Result<bool, Signal> {
        Ok(matches!(self.code_inserted_page(st)?, Some((Triggers::Coded, _))))
    }

    /// Whether the page with code was opened as its probe opens it: a Page.RunModal statement on a record filtered to one group
    /// (ListNewLineFactboxesCode.al:1138-1147), in any key, ascending.
    pub(super) fn code_inserted_coded_open(&mut self, st: &St) -> Result<bool, Signal> {
        Ok(matches!(
            self.code_inserted_open(st)?,
            Some((Opened::RunModalOneGroup, _))
        ))
    }

    /// The page stands on the line `New()` started, editable, inside a control's validation, and the test made exactly the
    /// measured calls on it since the open: `New()`, then the SetValue that runs now (:477-479, :697-698;
    /// ListNewLineSaveRecordUpdate.al:354-356, :397-399); `filtered` (`Opened::OpenEditOneGroup`), first the test's
    /// `Filter`, then `SetFilter` (ListNewLineExtraCode.al:375-376).
    fn code_inserted_line(&self, st: &St, filtered: bool) -> bool {
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
        // A part's calls count too: they are kept with the page's own (`PageState::tree_calls`).
        let no_part_calls = {
            let s = st.borrow();
            !s.tree_calls_unknown && s.tree_calls.iter().all(|c| c.part.is_none())
        };
        let calls = Self::test_calls(st);
        let measured = match calls.as_deref() {
            Some([TestCall::Page(PageMethod::New), TestCall::Write(_)]) => !filtered,
            Some(
                [
                    TestCall::Page(PageMethod::Filter),
                    TestCall::Other(PageMethod::SetFilter),
                    TestCall::Page(PageMethod::New),
                    TestCall::Write(_),
                ],
            ) => filtered,
            _ => false,
        };
        line && no_part_calls && measured
    }

    /// Whether Rec is the row the table field's OnValidate inserted from the line before the control's OnValidate began (:30-37):
    /// the primary key is a Code field, then an AutoIncrement Integer field (:23-27, :44); the AutoIncrement field was blank when
    /// the line started and the Code field holds the line's value; the row is stored and was written after the line started and
    /// before the control's own OnValidate began (its SystemRowVersion above the store's mark when the line started,
    /// `new_line_writes`, and not above the mark when the control's OnValidate began, `control_onvalidate`). The table has
    /// no table extension and no event subscriber; with `manual_unbound` (the page with code), manual subscribers nobody binds
    /// are allowed (ListNewLineFactboxesCode.al:255-275). `Inserted::Rewritten` for that row when the control's OnValidate then
    /// wrote it once, and nothing else (`code_inserted_extra.rs`, ListNewLineExtraCode.al:443).
    fn inserted_by_table_field(&mut self, st: &St, manual_unbound: bool) -> Result<Option<Inserted>, Signal> {
        let (Some(start), Some((line_mark, _)), Some(control)) = ({
            let s = st.borrow();
            (s.draft_start.clone(), s.new_line_writes, s.control_onvalidate.clone())
        }) else {
            return Ok(None);
        };
        let (control_mark, control_deletes) = control.writes;
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let engine = self.engine();
        let object = table.object as usize;
        let Some(schema) = engine.program.objects[object].table.as_ref() else {
            return Ok(None);
        };
        let [code, auto] = table.keys[0][..] else {
            return Ok(None);
        };
        let key_shape = matches!(schema.fields[code].ty, al_builtins::value::RtType::Code(_))
            && matches!(schema.fields[auto].ty, al_builtins::value::RtType::Integer)
            && schema.fields[auto].auto_increment;
        let plain_table = schema.extensions.is_empty()
            && if manual_unbound {
                self.no_active_subscriber(object)
            } else {
                !engine.events.any_subscriber_of(object)
            };
        let from_line = matches!(start.get(auto), Some(Value::Integer(0)))
            && Self::same_values(&rec.borrow().fields[code..=code], &start[code..=code]);
        if !(key_shape && plain_table && from_line) {
            return Ok(None);
        }
        let Some(stored) = self.stored_row(&rec)? else {
            return Ok(None);
        };
        let by_table_field = |v: i64| (line_mark + 1..=control_mark).contains(&v);
        let Some(version) = crate::stale_rename::row_version(&table, &stored) else {
            return Ok(None);
        };
        if by_table_field(version) {
            return Ok(Some(Inserted::ByTableField));
        }
        // The row the table field inserted when the control's OnValidate began, written once since, and nothing else written.
        let rewritten = control.row_version.is_some_and(by_table_field)
            && version == control_mark + 1
            && self.write_mark() == (control_mark + 1, control_deletes);
        Ok(rewritten.then_some(Inserted::Rewritten))
    }

    /// Which measured view holds the inserted row last: alone, or after one row with the same value in the key's first field
    /// (an empty view, or row 'A' before the line, :386-398, :620-641; ListNewLineSaveRecordUpdate.al also with a row of
    /// another group outside the view of a RunModal filtered to one group); on the page with code (`alone`) only the empty
    /// view. `None` for two rows before it, or a row after it: not measured.
    fn code_inserted_view(&mut self, st: &St, alone: bool) -> Result<Option<View>, Signal> {
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let Some(&code) = table.keys[0].first() else {
            return Ok(None);
        };
        let row = table.primary_key(&rec.borrow().fields);
        let is_row = |fields: &[Value]| Self::same_key(&table.primary_key(fields), &row);
        Ok(match self.view_rows(&rec, 3)?.as_slice() {
            [only] if is_row(only) => Some(View::Alone),
            [before, last]
                if !alone
                    && is_row(last)
                    && Self::same_values(&before[code..=code], &rec.borrow().fields[code..=code]) =>
            {
                Some(View::AfterRow)
            }
            _ => None,
        })
    }

    /// The page control's own OnValidate trigger `r` of control `idx`, recorded in `PageState::control_onvalidate` while it runs
    /// (`inserted_by_table_field` and `refresh_new_line.rs` `found_row_cell` read it).
    pub(super) fn run_control_onvalidate(&mut self, st: &St, idx: usize, r: usize) -> Result<Value, Signal> {
        let mut begun = crate::pages::state::ControlOnValidate::new(self.write_mark(), self.store.change_counter());
        begun.row_version = self.new_line_row_version(st);
        let start = Some(begun);
        let outer = std::mem::replace(&mut st.borrow_mut().control_onvalidate, start);
        self.pages.onvalidate_pages.push(st.clone());
        let result = self.control_routine(st, idx, r);
        self.pages.onvalidate_pages.pop();
        let mut s = st.borrow_mut();
        s.control_onvalidate = outer;
        // An OnValidate that failed after its save left the new-row line runs no refresh (`ValidateStart::left_new_line`).
        if let (Err(_), Some(start)) = (&result, s.filters_before_validate.as_mut()) {
            start.left_new_line = false;
        }
        drop(s);
        result
    }

    /// A control's validation ended with `e` (`validate_step_close`): an AL error after `land_on_code_inserted_line` or
    /// `save_record_on_code_inserted_line` moved the page onto the row (an OnValidate that fails after `CurrPage.Update` or
    /// `CurrPage.SaveRecord`, whose insert the error rolls back) is not measured, and refused. Every other signal goes on.
    pub(super) fn refuse_error_after_landing(&mut self, st: &St, e: Signal) -> Signal {
        let landed = {
            let s = st.borrow();
            s.refresh.starts_from_code_inserted_line()
                || !matches!(s.code_inserted_save, CodeInsertedSave::Off)
                || matches!(
                    s.code_inserted_row.as_ref().map(|r| r.factbox),
                    Some(FactboxStep::CodedTyped { .. })
                )
        };
        // On a page of the page-with-code shape with other code, an error before the landing that the cells measure (OnInsert's at
        // the table field's insert, the table field's own, the control's before its save: `coded_error_measured`) reaches the
        // test and leaves the handler (ListNewLineExtraCodeCodedTableTriggers.al:746, :1249-:1269): a later call on the page, or
        // its close after a handler caught it, is refused (`coded_line_before_set_value`, `code_inserted_row_at_close`). Any
        // other error there is refused (`coded_must_have_landed`; review of al-runner PR 8840, 01:32Z).
        let extra_line_read = {
            let s = st.borrow();
            s.coded_line == CodedLine::LineRead && s.coded_objects == super::state::CodedObjects::Extra
        };
        if matches!(&e, Signal::Error(err) if extra_line_read && self.coded_error_measured(st, err)) {
            st.borrow_mut().forget_code_inserted_line();
            st.borrow_mut().coded_line = CodedLine::Failed;
            return e;
        }
        if matches!(e, Signal::Error(_))
            && let Err(refused) = self.coded_must_have_landed(st)
        {
            return refused;
        }
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

    /// A control's validation ended without an error (`validate_step_close`). After `CurrPage.SaveRecord` or `CurrPage.Update(true)`
    /// left the new line, it must end as the cells do (ListNewLineSaveRecordUpdate.al:123-134, :190-197): the SaveRecord followed
    /// by its `CurrPage.Update(false)`, and the page's record, its view and the table as the save left them (`view_kept`). Any
    /// other end is not measured, and refused.
    pub(super) fn code_inserted_step_end(&mut self, st: &St) -> Result<(), Signal> {
        self.coded_must_have_landed(st)?;
        let save = std::mem::take(&mut st.borrow_mut().code_inserted_save);
        let why = match save {
            CodeInsertedSave::Off => return Ok(()),
            // The effect gate: nothing changed since the Update (`extra_code_unchanged`).
            CodeInsertedSave::Asked(view) if view.extra => {
                if self.extra_code_unchanged(st, &view)? {
                    return Ok(());
                }
                "the validation changed the page's record, the table or the store after CurrPage.Update moved the page from its \
                 new line onto the row the table field inserted, under code that is not a pinned page's (not measured)"
            }
            CodeInsertedSave::Asked(view) if self.view_kept(st, &view)? => return Ok(()),
            // CurrPage.SaveRecord() with no Update after it on the page with triggers and a FactBox: BC refreshes nothing and the
            // page stays on the saved row (ListNewLineExtraCodeFactbox.al:701, :713), also after a field of Rec set or a row of
            // another table inserted or modified (:766-837, `extra_code_save_kept`).
            CodeInsertedSave::Saved(view)
                if view.extra
                    && matches!(self.code_inserted_page(st)?, Some((Triggers::FactboxExtra, _)))
                    && self.extra_code_save_kept(st, &view)? =>
            {
                return Ok(());
            }
            CodeInsertedSave::Asked(_) => {
                "the validation changed the page's record, its view or the table after CurrPage.SaveRecord or Update(true) moved the \
                 page from its new line onto the row the table field inserted (not measured)"
            }
            CodeInsertedSave::Pending { .. } | CodeInsertedSave::Saved(_) => {
                "CurrPage.SaveRecord moved the page from its new line onto the row the table field inserted, and the validation \
                 ended without the CurrPage.Update(false) after it (not measured)"
            }
        };
        st.borrow_mut().forget_code_inserted_line();
        unsupported(why)
    }

    /// Whether the page's record (its fields, filters, filter group, key, sort order and MarkedOnly) and the store's write mark
    /// are as `view` noted them after the save. On the page with code the procedure its controls call writes another table and
    /// may write the row itself, then reads Rec again (ListNewLineFactboxesCode.al:276-297): there every other row of the page's
    /// table is as the save left it, Rec holds the row as stored now, and no subscriber of the table or the page is bound.
    pub(super) fn view_kept(&mut self, st: &St, view: &SavedView) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let fields_kept = match &view.rows {
            None => self.write_mark() == view.writes && Self::same_values(&rec.borrow().fields, &view.rec.fields),
            Some(rows) => {
                let table = self.table(&rec)?;
                let key = table.primary_key(&view.rec.fields);
                let same_row = Self::same_key(&table.primary_key(&rec.borrow().fields), &key);
                let page_object = st.borrow().object as usize;
                let now = self.coded_rows(st)?;
                let stored = self.stored_row(&rec)?;
                same_row
                    && self.no_active_subscriber(rec.borrow().table as usize)
                    && self.no_active_subscriber(page_object)
                    && stored.is_some_and(|s| match view.extra {
                        // The effect gate: the row or Rec changed since the save, not both (`extra_code_kept`).
                        true => Self::extra_code_kept(&rec.borrow().fields, &s, &view.rec.fields),
                        false => Self::same_values(&rec.borrow().fields, &s),
                    })
                    && now.is_some_and(|now| {
                        now.len() == rows.len()
                            && now
                                .iter()
                                .zip(rows)
                                .all(|(a, b)| Self::same_values(a, b) || Self::same_key(&table.primary_key(a), &key))
                    })
            }
        };
        Ok(fields_kept && Self::same_view(&rec.borrow(), &view.rec))
    }

    /// Whether records `r` and `v` have the same view: filters, filter group, key, sort order and MarkedOnly.
    pub(super) fn same_view(r: &RecordState, v: &RecordState) -> bool {
        crate::records::same_filters(&r.filters, &v.filters)
            && r.filter_group == v.filter_group
            && r.key == v.key
            && r.adhoc == v.adhoc
            && r.ascending == v.ascending
            && r.field_descending == v.field_descending
            && r.marked_only == v.marked_only
    }

    /// The save of `CurrPage.SaveRecord` or `CurrPage.Update(true)` that left the new line (`save_from_update`) is done: the page's
    /// record and the store as it left them are noted, which the calls after it must keep (`view_kept`).
    pub(super) fn code_inserted_saved(&mut self, st: &St) -> Result<(), Signal> {
        let CodeInsertedSave::Pending { asked } = st.borrow().code_inserted_save else {
            return Ok(());
        };
        let extra = std::mem::take(&mut st.borrow_mut().code_inserted_extra);
        let rows = match self.code_inserted_page(st)? {
            Some((Triggers::Coded, _)) => match self.coded_rows(st)? {
                Some(rows) => Some(rows),
                None => {
                    return unsupported(
                        "a List page with code and FactBoxes over a table of more than 1000 rows (not measured)",
                    );
                }
            },
            _ if extra => Some(self.extra_code_rows(st)?),
            _ => None,
        };
        let view = Box::new(SavedView {
            rec: self.page_rec(st).borrow().clone(),
            writes: self.write_mark(),
            rows,
            extra,
        });
        st.borrow_mut().code_inserted_save = match asked {
            true => CodeInsertedSave::Asked(view),
            false => CodeInsertedSave::Saved(view),
        };
        Ok(())
    }

    /// `CurrPage.Update()` (`arg` `None`), `CurrPage.Update(false)` or `CurrPage.Update(true)` in a measured cell
    /// (`code_inserted_cell`): the page leaves the new line for the stored row before the call goes on, so `Update()` and
    /// `Update(true)` save it as a stored row (its modify, `P.MOD(X/);T.M(X/);` before the OnValidate ends,
    /// ListNewLineCodeInsertUpdate.al:419 `ExpectedSetName`, ListNewLineSaveRecordUpdate.al:510 `setname`) and `Update(false)`
    /// saves nothing. The refresh the call asked for reads the view from its first row (`code_inserted_refresh_due`). After a
    /// `CurrPage.SaveRecord` that left the line, see `update_after_code_inserted_save`.
    pub(super) fn land_on_code_inserted_line(&mut self, st: &St, arg: Option<&Value>) -> Result<(), Signal> {
        let save = st.borrow().code_inserted_save.clone();
        match save {
            CodeInsertedSave::Off => {}
            CodeInsertedSave::Saved(view) => return self.update_after_code_inserted_save(st, arg, &view),
            _ => {
                st.borrow_mut().forget_code_inserted_line();
                return unsupported(
                    "a further CurrPage.Update after CurrPage.SaveRecord or Update(true) moved the page from its new line onto the \
                     row the table field inserted (not measured)",
                );
            }
        }
        let leave = match arg {
            None => Leave::Update,
            Some(Value::Boolean(false)) => Leave::UpdateFalse,
            Some(Value::Boolean(true)) => Leave::UpdateTrue,
            _ => return Ok(()),
        };
        if !self.code_inserted_cell(st, leave)? {
            return Ok(());
        }
        self.leave_code_inserted_line(st, leave != Leave::UpdateFalse);
        if leave == Leave::UpdateTrue {
            st.borrow_mut().code_inserted_save = CodeInsertedSave::Pending { asked: true };
        }
        self.ask_code_inserted_refresh(st)
    }

    /// `CurrPage.SaveRecord()` in a measured cell (`code_inserted_cell`, `Leave::SaveRecord`): the page leaves the new line for the
    /// stored row, and the save goes on as the modify of that row (`P.MOD(X/);T.M(X/);P.SR;`, ListNewLineSaveRecordUpdate.al:445
    /// `setname`). The `CurrPage.Update(false)` the cell makes next asks the refresh (`update_after_code_inserted_save`). A second
    /// SaveRecord before it is not measured, and refused.
    pub(super) fn save_record_on_code_inserted_line(&mut self, st: &St) -> Result<(), Signal> {
        if !matches!(st.borrow().code_inserted_save, CodeInsertedSave::Off) {
            st.borrow_mut().forget_code_inserted_line();
            return unsupported(
                "a further CurrPage.SaveRecord after CurrPage.SaveRecord or Update(true) moved the page from its new line onto the \
                 row the table field inserted (not measured)",
            );
        }
        // A SetValue on the row of the page with code: its save, then the `CurrPage.Update(false)` after it, as at the landing
        // (ListNewLineFactboxesCode.al:1407 `setvalue=T.VV(>ST-07);P.VV(ST-07);T.M(Station/ST-07);P.SR;...`).
        let typed = st
            .borrow()
            .code_inserted_row
            .as_ref()
            .is_some_and(|r| matches!(r.factbox, FactboxStep::CodedTyped { .. }));
        if typed {
            super::code_inserted_coded::coded_assume(st);
            if st.borrow().coded_objects == super::state::CodedObjects::Extra {
                self.extra_code_typed_save(st)?;
            }
            st.borrow_mut().code_inserted_save = CodeInsertedSave::Pending { asked: false };
            return Ok(());
        }
        if !self.code_inserted_cell(st, Leave::SaveRecord)? {
            return Ok(());
        }
        self.leave_code_inserted_line(st, true);
        st.borrow_mut().code_inserted_save = CodeInsertedSave::Pending { asked: false };
        Ok(())
    }

    /// The `CurrPage.Update` after a `CurrPage.SaveRecord` that left the line (`save_record_on_code_inserted_line`): `Update(false)`
    /// asks the refresh `Update()` asks (ListNewLineSaveRecordUpdate.al:445 `setname`: `P.SR;P.V.end;P.AGR(X);P.AGR(A);P.AGR(X);
    /// P.AGR(X);P.AGCR(X);`) when the page's record, its view and the table are as the save left them (`view_kept`); `Update()`
    /// and `Update(true)`, and an `Update(false)` after a change, are not measured there, and refused.
    fn update_after_code_inserted_save(
        &mut self,
        st: &St,
        arg: Option<&Value>,
        view: &SavedView,
    ) -> Result<(), Signal> {
        // Update(true) after the save is measured on the page without triggers (ListNewLineExtraCode.al:476) and on the later
        // SetValue of the page-with-code shape (ListNewLineExtraCodeCodedSecondSetValue.al:804). On the page with triggers and a
        // FactBox it stays refused: BC's answer there is measured 3 of 3 on BC 29.1 only by the neighbor app's
        // tests/neighbors/8734-new-line-extra-code-factbox/src/Neighbors.al:478, which no corpus cell pins; at the landing of
        // the page with code no cell measures it.
        let typed_extra = {
            let s = st.borrow();
            s.coded_objects == super::state::CodedObjects::Extra
                && matches!(
                    s.code_inserted_row.as_ref().map(|r| r.factbox),
                    Some(FactboxStep::CodedTyped { .. })
                )
        };
        // The later SetValue's cells keep Rec as the save left it up to the Update (another table written, :746, or the row
        // modified through another record, :782, at Update(false)), and Update(true) follows the save with nothing written
        // (:804); a field of Rec set, or a write before Update(true), is not measured (review of al-runner PR 8847, 01:00Z).
        if typed_extra {
            let rec_kept = Self::same_values(&self.page_rec(st).borrow().fields, &view.rec.fields);
            let update_true = matches!(arg, Some(Value::Boolean(true)));
            let measured = match update_true {
                true => rec_kept && self.write_mark() == view.writes,
                false => rec_kept && self.extra_typed_other_writes_inserted(st, view)?,
            };
            if !measured {
                st.borrow_mut().forget_code_inserted_line();
                return unsupported(
                    "a field of Rec set, a row of another table modified or deleted, or a write before CurrPage.Update(true), \
                     between CurrPage.SaveRecord and the CurrPage.Update after it in the later SetValue on a List page of the \
                     shape of the page with code (not measured)",
                );
            }
        }
        if view.extra
            && matches!(arg, Some(Value::Boolean(true)))
            && (typed_extra || matches!(self.code_inserted_page(st)?, Some((Triggers::Quiet, _))))
        {
            return self.extra_code_update_true(st, view);
        }
        if !matches!(arg, Some(Value::Boolean(false))) {
            st.borrow_mut().forget_code_inserted_line();
            return unsupported(
                "CurrPage.Update() or Update(true) after CurrPage.SaveRecord moved the page from its new line onto the row the \
                 table field inserted (not measured)",
            );
        }
        if !self.view_kept(st, view)? {
            st.borrow_mut().forget_code_inserted_line();
            return unsupported(
                "the page's record, its view or the table changed between CurrPage.SaveRecord, which moved the page from its new \
                 line onto the row the table field inserted, and the CurrPage.Update(false) after it (not measured)",
            );
        }
        // At the landing only: the later SetValue of a control with code has its own cells for what it writes between its save
        // and its Update (ListNewLineExtraCodeCodedSecondSetValue.al).
        let landing = !matches!(
            st.borrow().code_inserted_row.as_ref().map(|r| r.factbox),
            Some(FactboxStep::CodedTyped { .. })
        );
        let coded_extra = landing
            && st.borrow().coded_objects == super::state::CodedObjects::Extra
            && matches!(self.code_inserted_page(st)?, Some((Triggers::Coded, _)));
        if coded_extra && !self.extra_coded_between_kept(st, view)? {
            st.borrow_mut().forget_code_inserted_line();
            return unsupported(
                "a field of Rec set, or another table written, between CurrPage.SaveRecord and the CurrPage.Update(false) after \
                 it on a List page of the shape of the page with code (not measured)",
            );
        }
        let asked = match view.extra {
            // The effect gate notes what the page, the table and the store hold at the Update (`extra_code_unchanged`).
            true => self.extra_code_at_update(st)?,
            false => view.clone(),
        };
        st.borrow_mut().code_inserted_save = CodeInsertedSave::Asked(Box::new(asked));
        if view.extra {
            super::code_inserted_extra::extra_code_assume();
        }
        if matches!(self.code_inserted_page(st)?, Some((Triggers::Coded, _))) {
            super::code_inserted_coded::coded_assume(st);
        }
        self.ask_code_inserted_refresh(st)
    }

    /// The page leaves the new line for the row the table field inserted; without `save` the row it shows is xRec from here on.
    fn leave_code_inserted_line(&mut self, st: &St, save: bool) {
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
    /// starts from it (`previous_on_code_inserted_row`).
    pub(super) fn ask_code_inserted_refresh(&mut self, st: &St) -> Result<(), Signal> {
        let key = self.row_key(&self.page_rec(st))?;
        let mut s = st.borrow_mut();
        s.refresh.land_on_code_inserted_line();
        let calls = s.test_calls.len();
        // A SetValue on the row of the page with code keeps the count of its steps (`coded_refresh`).
        let factbox = match s.code_inserted_row.as_ref().map(|r| r.factbox) {
            Some(typed @ FactboxStep::CodedTyped { .. }) => typed,
            _ => FactboxStep::Off,
        };
        s.code_inserted_row = Some(CodeInsertedRow {
            key,
            calls,
            read_before_leave: false,
            factbox,
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
        let kind = self.code_inserted_page(st)?.map(|(t, _)| t);
        if matches!(kind, Some(Triggers::Coded)) {
            return self.coded_refresh(st, without_curr, without_parts);
        }
        if matches!(kind, Some(Triggers::FactboxExtra)) {
            return self.extra_code_factbox_refresh(st, without_curr, without_parts);
        }
        let factbox = matches!(kind, Some(Triggers::FactboxHost));
        self.reread_view_from_row(st, without_curr, without_parts, !factbox)?;
        if factbox {
            let row_before = self.view_row_count(st)? > 1;
            if let Some(row) = st.borrow_mut().code_inserted_row.as_mut() {
                row.read_before_leave = row_before;
                row.factbox = FactboxStep::Landed;
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
        // The Next() `refuse_off_factbox_row` lets through after Previous() answered false moves as on main (:683 `nextlog`).
        if row.factbox == FactboxStep::PreviousNo {
            return Ok(None);
        }
        let previous = how == PageMethod::Previous
            && st.borrow().cursor == Cursor::Row
            && Self::same_key(&row.key, &self.row_key(&self.page_rec(st))?);
        if row.factbox != FactboxStep::Off {
            // `refuse_off_factbox_row` let only the measured calls through: the Value SetValue, then this Previous.
            if !(previous && row.factbox == FactboxStep::Typed && self.typed_since_landing(st)?) {
                return unsupported(MOVE_OFF_FACTBOX_ROW);
            }
            if row.read_before_leave {
                self.read_code_inserted_row_again(st)?;
                return Ok(None);
            }
            if self.view_row_count(st)? != 1 {
                return unsupported(MOVE_OFF_FACTBOX_ROW);
            }
            // On the empty view Previous() answers false (:677 `prev=F/X`); only reads and Next() follow it (:683).
            st.borrow_mut().code_inserted_row = Some(CodeInsertedRow {
                factbox: FactboxStep::PreviousNo,
                ..row
            });
            return Ok(Some(false));
        } else if !(previous && self.only_fields_since(st, &row)) {
            return Ok(None);
        }
        if self.view_row_count(st)? != 1 {
            return Ok(None);
        }
        Ok(Some(false))
    }

    /// The page closes (`close_with`) on the row its line became. On the FactBox host the close is measured after a SetValue that
    /// changed the row and control reads only: `Close()` under OpenEdit and `OK().Invoke()` under both opens (`Close()` in the
    /// ModalPageHandler raises on BC). Any other close there is refused. With a row before, the host reads the row again before
    /// the close saves it (ListNewLineSaveRecordUpdate.al:784 `close=P.AGR(X/);T.M(X/V);`, :801 and :846 with OK); on an empty view
    /// it does not (:816, :831, :862 `close=T.M(X/V);`). Other pages close as before.
    pub(super) fn code_inserted_row_at_close(&mut self, st: &St) -> Result<(), Signal> {
        // The cells' error before the landing (ListNewLineExtraCodeCodedTableTriggers.al:746), or the task error after it
        // (ListNewLineExtraCodeCodedTaskError.al:589), leaves the handler: a handler that caught it and closes the page is not
        // measured (review of al-runner PR 8840, 01:32Z).
        if Self::tree_root(st).0.borrow().coded_line == CodedLine::Failed {
            return unsupported(
                "a List page with code and FactBoxes closed after an error of its first SetValue that the handler caught (not \
                 measured)",
            );
        }
        if !matches!(st.borrow().code_inserted_save, CodeInsertedSave::Off) {
            st.borrow_mut().forget_code_inserted_line();
            return unsupported(
                "the page closed after CurrPage.SaveRecord or Update(true) moved it from its new line onto the row the table field \
                 inserted, before the validation ended (not measured)",
            );
        }
        let Some(row) = st.borrow_mut().code_inserted_row.take() else {
            return Ok(());
        };
        // Close() in the ModalPageHandler of a Page.RunModal raises on BC; OK() is measured there.
        let measured = match row.factbox {
            FactboxStep::Typed => !st.borrow().modal,
            FactboxStep::OkAsked => true,
            _ => false,
        };
        match row.factbox {
            FactboxStep::Off => Ok(()),
            // The page with code on its empty view: OK() runs nothing (ListNewLineFactboxesCode.al:1407 `ok=`).
            FactboxStep::CodedOk => Ok(()),
            _ if measured && st.borrow().cursor == Cursor::Row && self.typed_since_landing(st)? => {
                if row.read_before_leave {
                    self.read_code_inserted_row_again(st)?;
                }
                Ok(())
            }
            _ => unsupported(MOVE_OFF_FACTBOX_ROW),
        }
    }

    /// The FactBox host, whose refresh read another row last (`code_inserted_refresh`), reads the row it stands on again before
    /// it leaves it: OnAfterGetRecord on the row as stored, after which Rec holds the edits typed since again, which the save then
    /// writes (ListNewLineSaveRecordUpdate.al:692 `prevlog=P.AGR(X/);T.M(X/V);`, :784 `close=P.AGR(X/);T.M(X/V);`).
    fn read_code_inserted_row_again(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let Some(stored) = self.stored_row(&rec)? else {
            return unsupported(MOVE_OFF_FACTBOX_ROW);
        };
        let typed = std::mem::replace(&mut rec.borrow_mut().fields, stored.clone().into());
        let read = self.page_trigger(st, "onaftergetrecord", vec![]);
        let unchanged = Self::same_values(&rec.borrow().fields, &stored);
        rec.borrow_mut().fields = typed;
        read?;
        if !unchanged {
            return unsupported(
                "the OnAfterGetRecord of a List page with a CardPart FactBox changed Rec when it read the row its new line \
                 became again before the save (not measured)",
            );
        }
        Ok(())
    }

    /// Whether Rec holds an edit the test typed since the landing: a field differs from the stored row (the SetValue on Value,
    /// ListNewLineSaveRecordUpdate.al:404, writes 'V' over the empty Value). A SetValue of the value already there is not
    /// measured.
    fn typed_since_landing(&mut self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let Some(stored) = self.stored_row(&rec)? else {
            return Ok(false);
        };
        Ok(!Self::same_values(&rec.borrow().fields, &stored))
    }

    /// Whether the test's calls on `st` since the landing on `row` are control reads and writes, then this `Previous()`.
    fn only_fields_since(&self, st: &St, row: &CodeInsertedRow) -> bool {
        Self::test_calls(st).is_some_and(|c| match c.get(row.calls..).unwrap_or_default() {
            [fields @ .., TestCall::Page(PageMethod::Previous)] => {
                fields.iter().all(|t| matches!(t, TestCall::Read | TestCall::Write(_)))
            }
            _ => false,
        })
    }

    /// The test's `call` on `st` or one of its parts while the FactBox host stands on the row its new line became
    /// (`FactboxStep`): only the calls ListNewLineSaveRecordUpdate.al:394-418 and :739-829 make are let through, reads of the
    /// host's and the part's controls, then one SetValue of a host control without code (`control_without_code`, :404), then
    /// `Previous()` (:411, `previous_on_code_inserted_row`), `Close()` or `OK()` and its `Invoke` (`code_inserted_row_at_close`); after
    /// a `Previous()` that answered false, reads and `Next()` (:683). Every other call is refused. Calls of page code (`None`) go
    /// on.
    pub(super) fn refuse_off_factbox_row(&self, st: &St, call: Option<TestCall>) -> Result<(), Signal> {
        let (root, part) = Self::tree_root(st);
        let Some(step) = root.borrow().code_inserted_row.as_ref().map(|r| r.factbox) else {
            return Ok(());
        };
        let Some(call) = call else {
            return Ok(());
        };
        let next = match (step, call, part) {
            (FactboxStep::Off, _, _) => return Ok(()),
            (_, TestCall::Read, _) => step,
            (FactboxStep::Landed, TestCall::Write(i), None) if self.control_without_code(&root, i) => {
                FactboxStep::Typed
            }
            (FactboxStep::Typed, TestCall::Page(PageMethod::Previous | PageMethod::Close), None) => step,
            (FactboxStep::Typed, TestCall::Page(PageMethod::OK), None) => FactboxStep::OkAsked,
            (FactboxStep::OkAsked, TestCall::Other(PageMethod::Invoke), None) => step,
            // The page with triggers and a FactBox: the cells measure reads, one SetValue and OK only (`ExtraLanded`).
            (FactboxStep::ExtraLanded, TestCall::Write(i), None) if self.control_without_code(&root, i) => {
                FactboxStep::ExtraTyped
            }
            (FactboxStep::ExtraTyped, TestCall::Page(PageMethod::OK), None) => FactboxStep::OkAsked,
            (FactboxStep::CodedExtraTyped, TestCall::Page(PageMethod::OK), None) => FactboxStep::OkAsked,
            (FactboxStep::PreviousNo, TestCall::Page(PageMethod::Next), None) => step,
            // The page with code: the SetValues of the measured control (`coded_later_control`) its objects are measured with
            // (`CodedObjects::later_set_value`, ListNewLineFactboxesCode.al:1407 and :1576 `setvalue2`; on a page of its shape
            // with other code, ListNewLineExtraCodeCodedSecondSetValue.al:746), then OK() and its Invoke (:1407 `ok=`,
            // `CodedObjects::ok_after`).
            (FactboxStep::Coded { steps }, TestCall::Write(i), None)
                if root.borrow().coded_objects.later_set_value(steps) && self.coded_later_control(&root, i) =>
            {
                FactboxStep::CodedTyped { steps }
            }
            (FactboxStep::Coded { steps }, TestCall::Page(PageMethod::OK), None)
                if root.borrow().coded_objects.ok_after(steps) =>
            {
                FactboxStep::CodedOk
            }
            (FactboxStep::CodedOk, TestCall::Other(PageMethod::Invoke), None) => step,
            // A page of the shape of the page with code: one SetValue of a control without code, then reads and OK only
            // (ListNewLineExtraCodeCodedPage.al:762-809; review of al-runner PR 8834, 21:06Z: not the FactBox host's
            // Previous and Close).
            (FactboxStep::Coded { steps: 0 }, TestCall::Write(i), None)
                if root.borrow().coded_objects == super::state::CodedObjects::Extra
                    && self.control_without_code(&root, i) =>
            {
                FactboxStep::CodedExtraTyped
            }
            _ => {
                root.borrow_mut().forget_code_inserted_line();
                return unsupported(match step {
                    FactboxStep::ExtraLanded | FactboxStep::ExtraTyped => MOVE_OFF_EXTRA_ROW,
                    FactboxStep::CodedExtraTyped => MOVE_OFF_CODED_EXTRA_ROW,
                    _ => MOVE_OFF_FACTBOX_ROW,
                });
            }
        };
        if let Some(row) = root.borrow_mut().code_inserted_row.as_mut() {
            row.factbox = next;
        }
        Ok(())
    }

    /// Whether control `idx` of `st` is bound to a field of Rec outside the primary key, and neither the control nor the field
    /// runs code when it is written: no trigger, no TableRelation (the Value control, ListNewLineSaveRecordUpdate.al:234, of
    /// field :40).
    fn control_without_code(&self, st: &St, idx: usize) -> bool {
        let engine = self.engine();
        let Some(control) = page_of(&engine, st).controls.get(idx) else {
            return false;
        };
        let ir::Binding::Field(f) = control.binding else {
            return false;
        };
        let rec = self.page_rec(st);
        let Ok(table) = self.table(&rec) else {
            return false;
        };
        let field = engine.program.objects[table.object as usize]
            .table
            .as_ref()
            .and_then(|t| t.fields.get(f));
        control.triggers.is_empty()
            && control.ext_triggers.is_empty()
            && !table.keys[0].contains(&f)
            && field.is_some_and(|d| {
                d.on_validate.is_none()
                    && d.ext_before_validate.is_empty()
                    && d.ext_after_validate.is_empty()
                    && !d.relation
            })
    }
}

/// The refusal of a call that leaves the FactBox host's row other than as measured.
const MOVE_OFF_FACTBOX_ROW: &str = "a call on a List page with a CardPart FactBox, on the row its new line became, other than control \
                                    reads, one SetValue, then Previous(), Close() or OK() (not measured)";

/// The refusal of a call that leaves the row of the page with triggers and a FactBox other than as measured.
const MOVE_OFF_EXTRA_ROW: &str = "a call on a List page with page triggers and a FactBox, on the row its new line became, other \
                                  than control reads, one SetValue, then OK() (not measured)";

/// The refusal of a call that leaves the row of a page of the shape of the page with code other than as measured.
const MOVE_OFF_CODED_EXTRA_ROW: &str = "a call on a List page of the shape of the page with code and FactBoxes, on the row its new \
                                        line became, other than control reads, one SetValue of a control without code, then OK() \
                                        (not measured)";
