//! When the DataCaptionExpression of a Worksheet, or of a Card with DelayedInsert, runs as AL code opens the page with
//! `Page.Run(Id, Rec)` or `Page.RunModal(Id, Rec)` and a TestPage handler answers it (al-runner issue 6924, APPD). Measured by
//! corpus PR 1392, pages/properties/DataCaptionExprWorksheet.al (its CI passed on all three BC majors the corpus runs); the cells
//! run under the assumption `data-caption-run-at-open`. Such a Worksheet in another shape refuses when its view is empty
//! (`data_caption_open_neighbor`); every other page keeps main's path (`data_caption_run.rs`, `caption_expr_list.rs`).

use super::pinned_shape::{NextEnd, PinnedEnd};
use super::*;
use crate::ui::UiKind;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::{FieldFilter, FilterExpr};

/// What the handler may do after the measured open: read the caption and the controls (DataCaptionExprWorksheet.al:627, :633,
/// :639, :645), and the page closes.
const OPENED_END: PinnedEnd = PinnedEnd {
    reason: "a call of the test on a page whose DataCaptionExpression ran at the open as DataCaptionExprWorksheet.al measured, \
             other than Caption, a control's value read or Close (not pinned)",
    reads: true,
    caption: true,
    part_reads: false,
    one_action: false,
    first_ends: false,
    part_visible: false,
    part_moves: false,
    no_writes: false,
    next: NextEnd::Refused,
};

/// The page of a measured cell (`DataCaptionExprWorksheet.al`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// PageType = Worksheet with DelayedInsert = true (:106-112).
    WorksheetDelayedInsert,
    /// PageType = Worksheet without DelayedInsert (:177-182).
    Worksheet,
    /// PageType = Card with DelayedInsert = true (:247-253).
    CardDelayedInsert,
}

/// What the expression did before the handler ran, in a measured cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Runs {
    /// Twice; the handler reads the globals it assigned and the caption with its value.
    Twice,
    /// Once; the handler reads the globals as they were before and the caption with its value.
    Once,
    /// Not at all; the caption stands alone.
    None,
    /// Once (main's Card path runs it before OnAfterGetCurrRecord and holds back its globals); it does not run again at the
    /// handler's first call.
    CardOnce,
}

/// The measured cells: (page, run modally, rows in the view, page editable after OnOpenPage) and what the expression did.
/// "Editable" is CurrPage.Editable := true in OnOpenPage or not set (:412 and :503 alike); "not editable" is
/// CurrPage.Editable := false in OnOpenPage.
const CELLS: [(Shape, bool, i32, bool, Runs); 10] = [
    // :412 item filter, :426 vendor filter, :503 Editable not set, :515 Page.Run(Page::X), :541 opened by the test directly.
    (Shape::WorksheetDelayedInsert, false, 0, true, Runs::Twice),
    // :439 Page.Run(0), :528 Page.Run(Page::X).
    (Shape::WorksheetDelayedInsert, false, 1, true, Runs::Once),
    // :452.
    (Shape::WorksheetDelayedInsert, false, 0, false, Runs::None),
    // :465.
    (Shape::WorksheetDelayedInsert, false, 1, false, Runs::Once),
    // :559 Page.RunModal(0, Rec) as a statement.
    (Shape::WorksheetDelayedInsert, true, 0, true, Runs::Twice),
    // :478.
    (Shape::Worksheet, false, 0, false, Runs::None),
    // :572.
    (Shape::Worksheet, false, 0, true, Runs::Twice),
    // :586.
    (Shape::Worksheet, false, 1, true, Runs::Once),
    // :491 Editable not set, :599 (caption without its value, the controls empty).
    (Shape::CardDelayedInsert, false, 0, true, Runs::CardOnce),
    // :613 (caption with its value, the controls empty).
    (Shape::CardDelayedInsert, false, 1, true, Runs::CardOnce),
];

/// Why a page is not in a measured cell: the fact of the cell it misses.
type Miss = &'static str;

/// The record AL code handed to `Page.Run`, as it was before the page opened, in the measured shape (`Session::data_caption_handed`).
pub(super) struct HandedRecord {
    filters: Vec<FieldFilter>,
    /// The store's change counter before OnInit.
    writes: u64,
}

impl Session {
    /// The record `record` that AL code hands to page `st` with `bind`'s handler bound, when it is the probe's: AL code's static
    /// run with a record and no field number (DataCaptionExprWorksheet.al:362-368), the test did not open the page; a stored
    /// record with no marks, the primary key in ascending order, filter group 0 active and one filter of group 0, a SetRange to
    /// one Code value that is not empty (:349, :357-359; `Item No.: I9`, `Vendor No.: V9`, `I1`); every field holding the value a
    /// fresh record variable of the table holds (the probe hands a fresh variable after Reset, :341-359, and its expression reads
    /// Rec's fields, :146).
    pub(super) fn data_caption_handed(
        &mut self,
        st: &St,
        record: Option<&Rec>,
        bind: Option<UiKind>,
    ) -> Result<HandedRecord, Miss> {
        let Some(r) = record.map(|r| r.borrow()) else {
            return Err("AL code handed no record");
        };
        if bind.is_none() || !st.borrow().static_run_record_only || self.pages.test_open {
            return Err("the page is not one AL code ran with Page.Run(Id, Rec) while no TestPage opened");
        }
        let one_code_value = match r.filters.as_slice() {
            [f] => {
                f.group == 0
                    && matches!(&f.expr, FilterExpr::Range(Some(Value::Code(a)), Some(Value::Code(b))) if a == b && !a.is_empty())
            }
            _ => false,
        };
        if !one_code_value {
            return Err("the handed record's filters are not one SetRange to one Code value that is not empty");
        }
        let measured = !r.temporary
            && r.marks.is_empty()
            && !r.marked_only
            && r.key == 0
            && r.adhoc.is_none()
            && r.ascending
            && r.field_descending == 0
            && r.filter_group == 0;
        if !measured {
            return Err("the handed record is temporary, marked, on another key or order, or in another filter group");
        }
        if !Self::same_values(&r.fields, &self.initial_fields(r.table as usize)) {
            return Err("the handed record's fields hold values (the probe handed a fresh record)");
        }
        Ok(HandedRecord {
            filters: r.filters.clone(),
            writes: self.store.change_counter(),
        })
    }

    /// The page `st` that AL code opened with `bind`'s handler bound and the record `handed`: in a measured cell, the expression
    /// runs as the probe saw it before the handler, and only the handler's caption and control reads and the close follow
    /// (`OPENED_END`). A page of the cells' neighborhood (`data_caption_open_neighbor`) in no cell refuses, naming the fact it
    /// misses; every other page keeps main's path.
    pub(super) fn data_caption_at_handled_open(
        &mut self,
        st: &St,
        bind: Option<UiKind>,
        handed: Result<HandedRecord, Miss>,
    ) -> Result<(), Signal> {
        let cell = match handed {
            Ok(handed) => self.data_caption_open_cell(st, bind, &handed)?,
            Err(miss) => Err(miss),
        };
        let runs = match cell {
            Ok(runs) => runs,
            Err(miss) if self.data_caption_open_neighbor(st, bind)? => {
                return Err(Signal::unsupported(format!(
                    "a Worksheet with DataCaptionExpression that AL code ran with a record and a handler answered, on an empty \
                     view: {miss} (the expression runs at the open only in the shape DataCaptionExprWorksheet.al of corpus PR \
                     1392 measured; al-runner issue 6924)"
                )));
            }
            Err(_) => return Ok(()),
        };
        assume(Assumption::DataCaptionRunAtOpen);
        match runs {
            Runs::Twice => {
                self.data_caption_expression_value(st)?;
                let value = self.data_caption_expression_value(st)?;
                st.borrow_mut().data_caption_value = Some(value);
            }
            Runs::Once => self.run_data_caption_held_back(st)?,
            Runs::None => st.borrow_mut().data_caption_value = None,
            Runs::CardOnce => st.borrow_mut().data_caption_due = false,
        }
        let mut s = st.borrow_mut();
        s.data_caption_at_open = runs != Runs::CardOnce;
        s.pinned_end = Some(OPENED_END);
        Ok(())
    }

    /// The caption of a Worksheet whose expression ran at the open (`data_caption_at_open`): the page caption, ` - ` and the
    /// value that run left (:420 `cap=DCW WS DI - Cap I9/`, :446 `- Cap I1/`), or the caption alone where it did not run (:459).
    /// `None` when the page took no measured cell. An empty value or one with trailing blanks is not measured.
    pub(super) fn data_caption_open_caption(&self, st: &St, base: &str) -> Option<Result<String, Signal>> {
        let s = st.borrow();
        if !s.data_caption_at_open {
            return None;
        }
        Some(match &s.data_caption_value {
            None => Ok(base.to_string()),
            Some(v) if !v.is_empty() && v.trim_end() == v => Ok(format!("{base} - {v}")),
            Some(_) => unsupported(
                "the caption of a Worksheet whose DataCaptionExpression ran at the open and returned an empty value or trailing blanks (not pinned)",
            ),
        })
    }

    /// The measured cell `st` is in, every fact required as DataCaptionExprWorksheet.al has it:
    /// - the run: a PageHandler bound to AL code's static `Page.Run(Id, Rec)` (:362, :364, :366, :550; page 0 and the page's id
    ///   alike, both measured: :412 and :439 with 0, :515 and :528 with the page's id), or a ModalPageHandler to `Page.RunModal(0,
    ///   Rec)` as a statement (:368); a top-level page; the record as `data_caption_handed` requires;
    /// - the page: a `Shape`, its own DataCaptionExpression, no property but the plain ones, DataCaptionExpression and
    ///   DelayedInsert; triggers OnInit, OnOpenPage and OnQueryClosePage only (:154-174); no extension, no subscriber; controls
    ///   bound to fields of Rec or to page variables, no control trigger (:120-130);
    /// - what OnInit and OnOpenPage did (`data_caption_open_left`);
    /// - 0 or 1 rows in the view; main's own state: on the Card the expression ran once and is due again; on the Worksheet it has
    ///   not run.
    fn data_caption_open_cell(
        &mut self,
        st: &St,
        bind: Option<UiKind>,
        handed: &HandedRecord,
    ) -> Result<Result<Runs, Miss>, Signal> {
        let run_modal = {
            let s = st.borrow();
            let run = match (bind, s.modal) {
                (Some(UiKind::Page), false) => Some(false),
                (Some(UiKind::ModalPage), true) if s.statement_run && !s.lookup && s.lookup_page_from_table => {
                    Some(true)
                }
                _ => None,
            };
            if s.opened_by_test || s.host.is_some() || !s.open {
                return Ok(Err("the page is not open at the top level"));
            }
            run
        };
        let Some(run_modal) = run_modal else {
            return Ok(Err(
                "a run other than Page.Run with a PageHandler or Page.RunModal(0, Rec) as a statement with a ModalPageHandler",
            ));
        };
        let shape = match self.data_caption_open_shape(st) {
            Ok(shape) => shape,
            Err(miss) => return Ok(Err(miss)),
        };
        let editable = match self.data_caption_open_left(st, handed) {
            Ok(editable) => editable,
            Err(miss) => return Ok(Err(miss)),
        };
        let rec = self.page_rec(st);
        let rows = self.stored_rows_passing(&rec, handed.filters.clone())?;
        let main_state = {
            let s = st.borrow();
            match shape {
                Shape::CardDelayedInsert => s.data_caption_due && s.data_caption_value.is_some(),
                _ => !s.data_caption_due && s.data_caption_value.is_none() && s.data_caption_pending.is_empty(),
            }
        };
        if !main_state {
            return Ok(Err(
                "the expression ran before the handler in another way than the probe's page",
            ));
        }
        Ok(CELLS
            .iter()
            .find(|c| c.0 == shape && c.1 == run_modal && c.2 == rows && c.3 == editable)
            .map(|c| c.4)
            .ok_or("a page type, run, row count and Editable that no measured cell combines"))
    }

    /// Whether page `st` is in the neighborhood of the measured cells where main's path is not an answer: a Worksheet (the
    /// measured page type, DataCaptionExprWorksheet.al:81, :177) with a DataCaptionExpression of its own, that AL code ran with
    /// `Page.Run(Id, Rec)` or `Page.RunModal(Id, Rec)` and no field number while `bind`'s handler is bound, at the top level, and
    /// whose view is empty after OnOpenPage. There BC ran the expression twice before the handler in every measured cell with
    /// the page editable (:412, :426, :503, :515, :541, :559, :572), and main's path runs it only at a caption read.
    fn data_caption_open_neighbor(&mut self, st: &St, bind: Option<UiKind>) -> Result<bool, Signal> {
        let run = {
            let s = st.borrow();
            matches!(bind, Some(UiKind::Page | UiKind::ModalPage))
                && s.static_run_record_only
                && s.open
                && s.host.is_none()
        };
        let worksheet = {
            let engine = self.engine();
            let object = st.borrow().object as usize;
            engine.object_facts[object].page_type == Some(al_ir::PageType::Worksheet)
                && engine.program.objects[object]
                    .properties
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case("DataCaptionExpression"))
        };
        if !(run && worksheet && self.has_source(st)) {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let filters = rec.borrow().filters.clone();
        Ok(self.stored_rows_passing(&rec, filters)? == 0)
    }

    /// Whether the page is editable after OnOpenPage, when OnInit and OnOpenPage left what the probe's left: Rec with the handed
    /// record's filters, key and order, filter group 0 active; no write to the store; the page variables the controls show still
    /// empty (:369 `OnOpenPage(item=)`); `CurrPage.Editable` assigned once in OnOpenPage (`'true'` :412, `'false'` :452) or not at
    /// all (:503), and no Editable set on the page before it ran. Otherwise the fact it misses.
    fn data_caption_open_left(&self, st: &St, handed: &HandedRecord) -> Result<bool, Miss> {
        let rec = self.page_rec(st);
        let kept = {
            let r = rec.borrow();
            crate::records::same_filters(&r.filters, &handed.filters)
                && r.key == 0
                && r.adhoc.is_none()
                && r.ascending
                && r.field_descending == 0
                && r.filter_group == 0
                && r.marks.is_empty()
                && !r.marked_only
        };
        if !kept {
            return Err("OnInit or OnOpenPage changed Rec's filters, key, order, filter group or marks");
        }
        if self.store.change_counter() != handed.writes {
            return Err("the page's open wrote to the database");
        }
        let s = st.borrow();
        let Some(inst) = s.instance.as_ref().map(|i| i.borrow()) else {
            return Err("the page has no instance");
        };
        let engine = self.engine();
        let empty_vars = page_of(&engine, st).controls.iter().all(|c| match c.binding {
            ir::Binding::Var(slot) => {
                matches!(inst.globals.get(slot), Some(Value::Text(t) | Value::Code(t)) if t.is_empty())
            }
            _ => true,
        });
        if !empty_vars {
            return Err("a control shows a page variable that is not an empty Text or Code after OnOpenPage");
        }
        if s.editable_preset.is_some() {
            return Err("Editable was set on the page variable before the run");
        }
        match s.open_editable {
            OpenEditable::Unset => Ok(true),
            OpenEditable::InOpenPage(on) => Ok(on),
            OpenEditable::Other => Err("CurrPage.Editable was assigned in OnInit or more than once"),
        }
    }

    /// The page's `Shape`, when its code and properties are those of the probe's pages; otherwise the fact it misses.
    fn data_caption_open_shape(&self, st: &St) -> Result<Shape, Miss> {
        if !self.has_source(st)
            || !self.page_declares_within(
                st,
                &["oninit", "onopenpage", "onqueryclosepage"],
                &["datacaptionexpression", "delayedinsert"],
            )
        {
            return Err(
                "the page has a trigger other than OnInit, OnOpenPage and OnQueryClosePage, a property other than the \
                        plain ones, DataCaptionExpression and DelayedInsert, an extension, a subscriber, or a control property \
                        that is not a literal",
            );
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let props = &engine.program.objects[object].properties;
        let prop = |name: &str| {
            props
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.trim().to_ascii_lowercase())
        };
        // The expression calls a procedure of the page with no argument (:111 `DataCaptionExpression = GetCaption();`).
        const EXPRESSION: Miss = "the DataCaptionExpression is not a call of a procedure of the page without arguments";
        let expression = prop("datacaptionexpression").ok_or(EXPRESSION)?;
        let procedure = expression.strip_suffix("()").ok_or(EXPRESSION)?;
        if !engine.program.objects[object]
            .routines
            .iter()
            .any(|r| r.name.eq_ignore_ascii_case(procedure))
        {
            return Err(EXPRESSION);
        }
        let controls = page_of(&engine, st).controls.iter().all(|c| {
            c.triggers.is_empty()
                && match c.kind {
                    ir::ControlKind::Repeater | ir::ControlKind::Group => c.binding == ir::Binding::None,
                    ir::ControlKind::Field => matches!(c.binding, ir::Binding::Field(_) | ir::Binding::Var(_)),
                    _ => false,
                }
        });
        if !controls {
            return Err(
                "a control has a trigger (OnValidate, OnLookup, ...) or is not a field of Rec or a page variable",
            );
        }
        let delayed = prop("delayedinsert");
        match (
            &engine.object_facts[object].page_type,
            delayed.as_deref(),
            self.is_list(st),
        ) {
            (Some(al_ir::PageType::Worksheet), Some("true"), true) => Ok(Shape::WorksheetDelayedInsert),
            (Some(al_ir::PageType::Worksheet), None, true) => Ok(Shape::Worksheet),
            (Some(al_ir::PageType::Card), Some("true"), false) => Ok(Shape::CardDelayedInsert),
            _ => Err("a page type and DelayedInsert value no measured cell has"),
        }
    }
}
