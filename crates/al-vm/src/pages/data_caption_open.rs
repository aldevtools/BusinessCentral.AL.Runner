//! When the DataCaptionExpression of a Worksheet, or of a Card with DelayedInsert, runs as AL code opens the page with
//! `Page.Run(Id, Rec)` or `Page.RunModal(Id, Rec)` and a TestPage handler answers it (al-runner issue 6924, APPD). Measured by
//! corpus PR 1392, pages/properties/DataCaptionExprWorksheet.al (BC 28.5 and 29.0); the cells run under the assumption
//! `data-caption-run-at-open`. Every other page keeps main's path (`data_caption_run.rs`, `caption_expr_list.rs`).

use super::pinned_shape::{NextEnd, PinnedEnd};
use super::*;
use crate::ui::UiKind;
use al_builtins::assumptions::{Assumption, assume};

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

impl Session {
    /// The page `st` that AL code opened with `bind`'s handler bound: in a measured cell, the expression runs as the probe saw
    /// it before the handler, and only the handler's caption and control reads and the close follow (`OPENED_END`).
    pub(super) fn data_caption_at_handled_open(&mut self, st: &St, bind: Option<UiKind>) -> Result<(), Signal> {
        let Some(runs) = self.data_caption_open_cell(st, bind)? else {
            return Ok(());
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
    /// - the run: AL code's static `Page.Run(Id, Rec)` (:362, :364, :366, :550; page 0 and the page's id alike, both
    ///   measured: :412 and :439 with 0, :515 and :528 with the page's id) with a PageHandler bound, or `Page.RunModal(0, Rec)` as a statement
    ///   (:368) with a ModalPageHandler; the test did not open the page; a top-level page;
    /// - the page: a `Shape`, its own DataCaptionExpression, no property but the plain ones, DataCaptionExpression and
    ///   DelayedInsert; triggers OnInit, OnOpenPage and OnQueryClosePage only (:154-174); no extension, no subscriber; controls
    ///   bound to fields of Rec or to page variables, no control trigger (:120-130);
    /// - the record: a stored table, no marks, one filter of filter group 0 (:349, :357-359, :549); 0 or 1 rows in the view;
    /// - main's own state: on the Card the expression ran once and is due again; on the Worksheet it has not run.
    fn data_caption_open_cell(&mut self, st: &St, bind: Option<UiKind>) -> Result<Option<Runs>, Signal> {
        let run_modal = {
            let s = st.borrow();
            let run = match (bind, s.modal) {
                (Some(UiKind::Page), false) => Some(false),
                (Some(UiKind::ModalPage), true) if s.statement_run && !s.lookup && s.lookup_page_from_table => {
                    Some(true)
                }
                _ => None,
            };
            if !(s.static_run_with_record && !s.opened_by_test && s.host.is_none() && s.open) {
                return Ok(None);
            }
            run
        };
        let (Some(run_modal), Some(shape)) = (run_modal, self.data_caption_open_shape(st)) else {
            return Ok(None);
        };
        let rec = self.page_rec(st);
        let filters = {
            let r = rec.borrow();
            if r.temporary || !r.marks.is_empty() || r.marked_only || r.filters.len() != 1 || r.filters[0].group != 0 {
                return Ok(None);
            }
            r.filters.clone()
        };
        let rows = self.stored_rows_passing(&rec, filters)?;
        let editable = {
            let s = st.borrow();
            match (s.editable_off, s.editable_off_in_open) {
                (false, _) => true,
                (true, true) => false,
                (true, false) => return Ok(None),
            }
        };
        let main_state = {
            let s = st.borrow();
            match shape {
                Shape::CardDelayedInsert => s.data_caption_due && s.data_caption_value.is_some(),
                _ => !s.data_caption_due && s.data_caption_value.is_none() && s.data_caption_pending.is_empty(),
            }
        };
        if !main_state {
            return Ok(None);
        }
        Ok(CELLS
            .iter()
            .find(|c| c.0 == shape && c.1 == run_modal && c.2 == rows && c.3 == editable)
            .map(|c| c.4))
    }

    /// The page's `Shape`, when its code and properties are those of the probe's pages.
    fn data_caption_open_shape(&self, st: &St) -> Option<Shape> {
        if !self.has_source(st)
            || !self.page_declares_within(
                st,
                &["oninit", "onopenpage", "onqueryclosepage"],
                &["datacaptionexpression", "delayedinsert"],
            )
        {
            return None;
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
        prop("datacaptionexpression")?;
        let controls = page_of(&engine, st).controls.iter().all(|c| {
            c.triggers.is_empty()
                && match c.kind {
                    ir::ControlKind::Repeater | ir::ControlKind::Group => c.binding == ir::Binding::None,
                    ir::ControlKind::Field => matches!(c.binding, ir::Binding::Field(_) | ir::Binding::Var(_)),
                    _ => false,
                }
        });
        if !controls {
            return None;
        }
        let delayed = prop("delayedinsert");
        match (
            &engine.object_facts[object].page_type,
            delayed.as_deref(),
            self.is_list(st),
        ) {
            (Some(al_ir::PageType::Worksheet), Some("true"), true) => Some(Shape::WorksheetDelayedInsert),
            (Some(al_ir::PageType::Worksheet), None, true) => Some(Shape::Worksheet),
            (Some(al_ir::PageType::Card), Some("true"), false) => Some(Shape::CardDelayedInsert),
            _ => None,
        }
    }
}
