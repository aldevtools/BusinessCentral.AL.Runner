//! How many times a page open runs the CaptionClass expressions of its source table's fields with an empty record (al-runner
//! issue 5870), where the corpus pins it.

use super::*;

impl Session {
    /// The page type whose CaptionClass runs a Worksheet page shares: it runs them as a List (pages/lt-y61-page-probes/
    /// Y61CaptionClassCount.al:492 G615_Worksheet_4DifferentFields, :639 G615_Worksheet_1Field). A ListPlus with its field in a
    /// group runs them as a Card on the open, also once with the row on GoToRecord (:663 G615_ListPlus_1Field); its moves are not
    /// built, so it is `None` like every other page.
    pub(super) fn caption_runs_like(&self, st: &St) -> Option<al_ir::PageType> {
        use al_ir::PageType;
        let s = st.borrow();
        match self.engine.object_facts[s.object as usize].page_type.as_ref()? {
            PageType::Worksheet if !s.modal && matches!(s.mode, PageMode::View | PageMode::Edit) => {
                Some(PageType::List)
            }
            _ => None,
        }
    }

    /// How many times each CaptionClass expression of the source table runs with an empty record when page `st` opens: a page of
    /// its own (no host, not modal) opened with OpenView or OpenEdit. Pinned by writer SX (record/captions/
    /// SxCaptionClassCount.al:190-369): one expression field, Card 8 and List 7; two with the same expression text, 9 and 8 each.
    /// And by writer Y61 (testpage/Y61CaptionClassCount.al, three cycles alike on every major): on a List each field
    /// runs 9 times for three fields with the same text (:540), 10 for four fields with different or the same texts (:468, :516),
    /// 21 for fifteen fields with different texts (:564); a Worksheet as a List (:492 four fields, :639 one). `None` for any other shape: the counts are pinned per number of fields only.
    fn caption_empty_runs_pinned(&self, st: &St, exprs: &[(usize, String)]) -> Option<usize> {
        use al_ir::PageType;
        let page_type = {
            let s = st.borrow();
            if s.host.is_some() || s.modal || !matches!(s.mode, PageMode::View | PageMode::Edit) {
                return None;
            }
            self.engine.object_facts[s.object as usize].page_type.clone()
        };
        let card = match page_type {
            Some(PageType::Card) => true,
            Some(PageType::List) => false,
            Some(PageType::Worksheet) => self.caption_runs_like(st)? != PageType::List,
            _ => return None,
        };
        let same_text = exprs.iter().all(|(_, e)| e.trim() == exprs[0].1.trim());
        // A Card over a table with three fields, one of them with an expression, runs it 9 times (record/captions/
        // CaptionExpressionPlainText.al:368 CountCard_AsIntegerExpression_RunsAtOpen `open empty=9 rows=1`, two cycles, BC 27.5,
        // 28.5 and 29.0); SX's table has two fields (SxCaptionClassCount.al:19).
        let fields = {
            let rec = self.page_rec(st);
            let table = rec.borrow().table as usize;
            self.engine.tables[table]
                .as_ref()
                .map_or(0, |t| t.fields.iter().filter(|f| f.id < 2_000_000_000).count())
        };
        match (card, exprs.len()) {
            (true, 1) if fields == 3 => Some(9),
            (true, 1) => Some(8),
            (true, 2) if same_text => Some(9),
            (false, 1) => Some(7),
            (false, n @ (2 | 3)) if same_text => Some(6 + n),
            (false, n @ (4 | 15)) => Some(6 + n),
            _ => None,
        }
    }

    /// The empty-record runs of the open of page `st`; `Ok(true)` when the open's row run ran too (`caption_state_free_pin`).
    /// How many times: writer SX pinned it for a Card or a List opened with OpenView or OpenEdit over a table with one CaptionClass
    /// expression field (Card 8, List 7) or two with the same expression (Card 18, List 16 in all; corpus
    /// record/captions/SxCaptionClassCount.al:190 SX_CardOne_OpenEdit, :236 SX_CardTwo_OpenEdit, :282 SX_ListOne_OpenEdit, :328
    /// SX_ListTwo_OpenEdit and their OpenView twins, the same in three cycles on every supported major; al-runner issue 5870),
    /// and the other cells of `caption_empty_runs_pinned`.
    pub(super) fn run_caption_open_empty_runs(
        &mut self,
        st: &St,
        t: &al_data::TableDef,
        exprs: &[(usize, String)],
        empty: &Rec,
    ) -> Result<bool, Signal> {
        if self.run_caption_open_state_free(st, t, exprs, empty)? {
            return Ok(true);
        }
        match self.caption_empty_runs_pinned(st, exprs) {
            Some(runs) => {
                for _ in 0..runs {
                    for (pos, expr) in exprs {
                        self.field_caption_class_value(t, *pos, Some(empty), expr)?;
                    }
                }
            }
            // The open's row run and those of later moves refuse a dialog too (`PageState::caption_dialogs_refused`).
            None => self.run_caption_empty_unpinned(st, t, exprs, empty)?,
        }
        Ok(false)
    }

    /// Runs `f` with `dialogs` in force for the dialogs and errors the CaptionClass runs inside it raise
    /// (`Session::dispatch_ui_returning`, `Ctx::error_text_raise`).
    fn with_caption_dialogs<T>(
        &mut self,
        dialogs: CaptionDialogs,
        f: impl FnOnce(&mut Self) -> Result<T, Signal>,
    ) -> Result<T, Signal> {
        let outer = std::mem::replace(&mut self.pages.caption_dialogs, dialogs);
        let r = f(self);
        self.pages.caption_dialogs = outer;
        r
    }

    /// The refusal of a dialog `what` (`Message`, `Confirm`, `Error`, ...) raised while `pages.caption_dialogs` is in force; `None`
    /// lets it run. `message_or_error`: a Message, or an Error with a text.
    pub(crate) fn caption_dialog_refusal(&self, what: &str, message_or_error: bool) -> Option<String> {
        match (self.pages.caption_dialogs, message_or_error) {
            (CaptionDialogs::Free, _) | (CaptionDialogs::OpenRowMessageOrError, true) => None,
            _ => Some(format!(
                "{what} raised by a CaptionClass expression on a page where BC's runs with that dialog are not measured \
                 (al-runner issue 7391)"
            )),
        }
    }

    /// The cells of record/captions/CaptionClassStateFreeCalls.al with their number of empty-record runs, three cycles alike on
    /// BC 27, 28 and 29. Every fact is the probe's, by equality:
    /// - the source table has six fields, of them the third to sixth with CaptionClass expressions of four different texts
    ///   (table 62412 at :218-232);
    /// - the page declares PageType and SourceTable, the Card also UsageCategory = None (:444-448, :463-466); no trigger, no
    ///   action, no page extension and no event subscriber of the page;
    /// - the test opened it itself on a row (`SeedCountRow` before the open; one row at :896, :924, :996, three at :947-949);
    /// - a Card opened with OpenEdit whose five field controls show the first field and the four expression fields in field order,
    ///   directly in the content area, with ApplicationArea = All only (:454-458): 11 runs each (:907, :934, :980); its open's row run
    ///   lets a Message (:889-915) and an Error with a text (:917-937) through;
    /// - a CardPart opened alone with OpenView whose one cuegroup holds one field control over the first expression field
    ///   (:470-476): 6 runs each (:1013).
    fn caption_state_free_pin(&self, st: &St, exprs: &[(usize, String)]) -> Option<StateFreePin> {
        use al_ir::{ControlKind, PageType};
        let (card_open, part_open, on_row, object) = {
            let s = st.borrow();
            (
                s.test_open_edit(),
                s.test_open_view(),
                s.cursor == Cursor::Row,
                s.object as usize,
            )
        };
        let positions: Vec<usize> = exprs.iter().map(|(pos, _)| *pos).collect();
        let distinct = exprs
            .iter()
            .enumerate()
            .all(|(i, (_, e))| exprs[..i].iter().all(|(_, o)| o.trim() != e.trim()));
        let table = self.page_rec(st).borrow().table as usize;
        let t = self.engine.tables[table].as_ref()?;
        if positions != [2, 3, 4, 5] || !distinct || t.fields.iter().filter(|f| f.id < 2_000_000_000).count() != 6 {
            return None;
        }
        if !on_row || !self.unextended_unsubscribed(st) {
            return None;
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        if !page.triggers.is_empty() || !page.ext_triggers.is_empty() || !page.action_areas.is_empty() {
            return None;
        }
        let mut props: Vec<(String, String)> = engine.program.objects[object]
            .properties
            .iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_ascii_lowercase()))
            .filter(|(k, _)| k != "sourcetable")
            .collect();
        props.sort();
        let plain_field = |idx: usize, parent: Option<usize>| {
            let c = &page.controls[idx];
            c.kind == ControlKind::Field
                && c.parent == parent
                && c.owner.is_none()
                && c.triggers.is_empty()
                && c.ext_triggers.is_empty()
                && matches!(c.props.as_slice(), [(n, al_ir::PropVal::Lit(v))]
                    if n == "applicationarea" && v.trim().eq_ignore_ascii_case("All"))
        };
        let field_pos = |idx: usize| {
            let id = self.control_field(st, idx)?.id;
            t.fields.iter().position(|f| f.id == id)
        };
        let page_type = engine.object_facts[object].page_type.as_ref()?;
        let n = page.controls.len();
        match page_type {
            PageType::Card
                if card_open
                    && props
                        == [
                            ("pagetype".into(), "card".into()),
                            ("usagecategory".into(), "none".into()),
                        ]
                    && n == 5
                    && (0..n).all(|i| plain_field(i, None))
                    && (0..n).map(field_pos).collect::<Option<Vec<_>>>()? == [0, 2, 3, 4, 5] =>
            {
                Some(StateFreePin {
                    runs: 11,
                    open_row: CaptionDialogs::OpenRowMessageOrError,
                })
            }
            PageType::Other(kind)
                if kind == "cardpart"
                    && part_open
                    && props == [("pagetype".into(), "cardpart".into())]
                    && n == 2
                    && page.controls[0].kind == ControlKind::CueGroup
                    && page.controls[0].parent.is_none()
                    && page.controls[0].props.is_empty()
                    && page.controls[0].triggers.is_empty()
                    && plain_field(1, Some(0))
                    && field_pos(1)? == 2 =>
            {
                Some(StateFreePin {
                    runs: 6,
                    open_row: CaptionDialogs::Refused,
                })
            }
            _ => None,
        }
    }

    /// The open of a page in a cell of `caption_state_free_pin`: the empty-record runs and the row run, with every dialog and
    /// error refused but those of the open's row run the probe measured. Later row runs of the page refuse them too
    /// (`PageState::caption_dialogs_refused`).
    fn run_caption_open_state_free(
        &mut self,
        st: &St,
        t: &al_data::TableDef,
        exprs: &[(usize, String)],
        empty: &Rec,
    ) -> Result<bool, Signal> {
        let Some(pin) = self.caption_state_free_pin(st, exprs) else {
            return Ok(false);
        };
        st.borrow_mut().caption_dialogs_refused = true;
        self.with_caption_dialogs(CaptionDialogs::Refused, |this| {
            for _ in 0..pin.runs {
                for (pos, expr) in exprs {
                    this.field_caption_class_value(t, *pos, Some(empty), expr)?;
                }
            }
            Ok(())
        })?;
        self.with_caption_dialogs(pin.open_row, |this| {
            this.fill_field_captions_with_row_unguarded(st, t, exprs, 1)
        })?;
        Ok(true)
    }

    /// The empty-record runs of a page open whose number is not pinned (the corpus saw 240, 480 and 492 calls for one 12-field
    /// Card open, record/captions/QdCaptionClassAreas.al:657). The number matters only when an expression leaves something
    /// behind that AL code can read later: such an open is refused (`effects.rs`). Every open that runs them is a flagged pass at
    /// best (owner decision 2026-10-21). A dialog or an Error with a text that a run raises refuses, here and in the page's later
    /// row runs (`PageState::caption_dialogs_refused`): the run-time half of `effects.rs`, whose walk lets an expression reach
    /// them as long as no run takes that branch (record/captions/CaptionClassStateFreeCalls.al:168-181 NotTakenCc with an empty
    /// record, al-runner issue 7391).
    fn run_caption_empty_unpinned(
        &mut self,
        st: &St,
        t: &al_data::TableDef,
        exprs: &[(usize, String)],
        empty: &Rec,
    ) -> Result<(), Signal> {
        if let Some(why) = self.caption_class_lasting_effect(t, exprs) {
            return unsupported(why);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageOpenCaptionClassEmptyRecordCount);
        st.borrow_mut().caption_dialogs_refused = true;
        self.with_caption_dialogs(CaptionDialogs::Refused, |this| {
            for _ in 0..20 {
                for (pos, expr) in exprs {
                    this.field_caption_class_value(t, *pos, Some(empty), expr)?;
                }
            }
            Ok(())
        })
    }

    /// Why the empty-record runs of a page open are refused: the CaptionClass expression of a field of `t` can leave
    /// something behind (`Engine::lasting_effect`), so how many times BC runs it, which is not pinned, can be read later.
    /// Writes only to variables of SingleInstance codeunits are kept (`effects::Effect::Kept`): refusing them costs
    /// baseline cases.
    fn caption_class_lasting_effect(&self, t: &al_data::TableDef, exprs: &[(usize, String)]) -> Option<String> {
        let table = self.engine.program.objects[t.object as usize].table.as_ref()?;
        exprs.iter().find_map(|(pos, _)| {
            let (object, routine) = table.fields.get(*pos)?.caption_class?;
            let crate::effects::Effect::Refused(effect) = self.engine.lasting_effect(object, routine)? else {
                return None;
            };
            Some(format!(
                "the CaptionClass expression of field {} of table {} {effect}, and how many times BC runs it with an empty \
                 record during a page open is not pinned (al-runner issue 5870)",
                t.fields[*pos].name,
                self.engine.program.objects[t.object as usize].name
            ))
        })
    }

    /// Runs the CaptionClass expression of each control over such a field with the current row, `runs` times, and keeps the
    /// last caption for `Caption()`. On a page whose open ran the empty-record runs without a pinned number, a dialog these runs
    /// raise refuses (`PageState::caption_dialogs_refused`): how many row runs BC makes on such a page is not measured either.
    pub(super) fn fill_field_captions_with_row(
        &mut self,
        st: &St,
        t: &al_data::TableDef,
        exprs: &[(usize, String)],
        runs: usize,
    ) -> Result<(), Signal> {
        if st.borrow().caption_dialogs_refused {
            return self.with_caption_dialogs(CaptionDialogs::Refused, |this| {
                this.fill_field_captions_with_row_unguarded(st, t, exprs, runs)
            });
        }
        self.fill_field_captions_with_row_unguarded(st, t, exprs, runs)
    }

    /// [`Session::fill_field_captions_with_row`] without the dialog guard: runs the CaptionClass expression of each control over such a field with the current row, `runs` times, and keeps the
    /// last caption for `Caption()`.
    fn fill_field_captions_with_row_unguarded(
        &mut self,
        st: &St,
        t: &al_data::TableDef,
        exprs: &[(usize, String)],
        runs: usize,
    ) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let engine = self.engine();
        let page = page_of(&engine, st);
        for _ in 0..runs {
            for (idx, c) in page.controls.iter().enumerate() {
                if c.prop("captionclass").is_some() {
                    continue;
                }
                let Some(field) = self.control_field(st, idx) else {
                    continue;
                };
                let Some((pos, expr)) = exprs.iter().find(|(pos, _)| t.fields[*pos].id == field.id) else {
                    continue;
                };
                let class = self.field_caption_class_value(t, *pos, Some(&rec), expr)?;
                let caption = self.control_expression_caption(&t.fields[*pos].name, &class)?;
                let mut s = st.borrow_mut();
                s.field_captions.retain(|(i, _)| *i != idx);
                s.field_captions.push((idx, caption));
            }
        }
        st.borrow_mut().field_captions_row = Some(rec.borrow().fields.clone());
        Ok(())
    }
}

/// Which dialogs and errors the CaptionClass runs in progress may raise (`Session::caption_dialog_refusal`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CaptionDialogs {
    /// No guarded run is in progress: a dialog runs as anywhere else (the pinned counts of writers SX and Y61, as on main).
    #[default]
    Free,
    /// Every dialog (Message, Confirm, any other) and every Error with a text refuses: the runs of a page whose empty-record count
    /// is not pinned (the walk of `effects.rs` let the expression reach them only on that condition), and every run of a
    /// `caption_state_free_pin` page but the open's row run of its Card.
    Refused,
    /// The open's row run of the Card of `caption_state_free_pin`: a Message and an Error with a text run, as the probe measured
    /// (record/captions/CaptionClassStateFreeCalls.al:891-936); any other dialog refuses.
    OpenRowMessageOrError,
}

/// A cell of `Session::caption_state_free_pin`.
struct StateFreePin {
    /// Empty-record runs of each expression.
    runs: usize,
    /// What the open's row run may raise.
    open_row: CaptionDialogs,
}
