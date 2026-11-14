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
    pub(super) fn caption_empty_runs_pinned(&self, st: &St, exprs: &[(usize, String)]) -> Option<usize> {
        use al_ir::PageType;
        let page_type = {
            let s = st.borrow();
            if s.host.is_some() || s.modal || !matches!(s.mode, PageMode::View | PageMode::Edit) {
                return None;
            }
            self.engine.object_facts[s.object as usize].page_type.clone()
        };
        if let Some(runs) = self.caption_empty_runs_six_fields(st, page_type.as_ref(), exprs) {
            return Some(runs);
        }
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

    /// Runs `f` with every dialog refused (`Session::dispatch_ui_returning`): the CaptionClass runs of a page open whose empty-record
    /// runs are not pinned, the row runs of that open included (a dialog in a row run of such an open is not measured either).
    pub(super) fn with_caption_dialogs_refused<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, Signal>,
    ) -> Result<T, Signal> {
        let outer = std::mem::replace(&mut self.pages.caption_runs_unpinned, true);
        let r = f(self);
        self.pages.caption_runs_unpinned = outer;
        r
    }

    /// The cells of record/captions/CaptionClassStateFreeCalls.al (table 62412 at :218-232: six fields, four of them with
    /// CaptionClass expressions of different texts), three cycles alike on BC 27, 28 and 29:
    /// - a Card opened with OpenEdit whose five field controls show Id and the four expression fields: 11 runs each (:907, :934, :980; page 62415
    ///   at :444-461);
    /// - a CardPart opened alone with OpenView whose one field control shows one of them: 6 runs each (:1013; page 62416 at :463-478).
    ///
    /// The caller has checked: no host, not modal, OpenView or OpenEdit.
    fn caption_empty_runs_six_fields(
        &self,
        st: &St,
        page_type: Option<&al_ir::PageType>,
        exprs: &[(usize, String)],
    ) -> Option<usize> {
        use al_ir::PageType;
        let distinct = exprs.len() == 4
            && exprs
                .iter()
                .enumerate()
                .all(|(i, (_, e))| exprs[..i].iter().all(|(_, o)| o.trim() != e.trim()));
        let table = self.page_rec(st).borrow().table as usize;
        let fields = self.engine.tables[table]
            .as_ref()
            .map_or(0, |t| t.fields.iter().filter(|f| f.id < 2_000_000_000).count());
        if !distinct || fields != 6 {
            return None;
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        let t = self.engine.tables[table].as_ref()?;
        let bound: Vec<_> = (0..page.controls.len())
            .filter_map(|idx| self.control_field(st, idx))
            .collect();
        let shown = bound
            .iter()
            .filter(|f| exprs.iter().any(|(pos, _)| t.fields[*pos].id == f.id))
            .count();
        let mode = st.borrow().mode;
        // (field controls, of them over an expression field): Id and the four (:454-458), the one cue (:474).
        match (page_type?, mode, bound.len(), shown) {
            (PageType::Card, PageMode::Edit, 5, 4) => Some(11),
            (PageType::Other(kind), PageMode::View, 1, 1) if kind == "cardpart" => Some(6),
            _ => None,
        }
    }

    /// The empty-record runs of a page open whose number is not pinned (the corpus saw 240, 480 and 492 calls for one 12-field
    /// Card open, record/captions/QdCaptionClassAreas.al:657). The number matters only when an expression leaves something
    /// behind that AL code can read later: such an open is refused (`effects.rs`). Every open that runs them is a flagged pass at
    /// best (owner decision 2026-10-21). A dialog that a run raises is a trace too, so it refuses there (the run-time half of
    /// `effects.rs`: the walk lets an expression reach Message and Confirm, as long as no run takes that branch;
    /// record/captions/CaptionClassStateFreeCalls.al:168-181 NotTakenCc with an empty record, al-runner issue 7391). An error ends
    /// the open at the first run whatever the number, as on main (format-evaluate/autoformat/TestCaptionClassResolverFailure.al:116
    /// for an error raised while a page open resolves a caption).
    pub(super) fn run_caption_empty_unpinned(
        &mut self,
        t: &al_data::TableDef,
        exprs: &[(usize, String)],
        empty: &Rec,
    ) -> Result<(), Signal> {
        if let Some(why) = self.caption_class_lasting_effect(t, exprs) {
            return unsupported(why);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageOpenCaptionClassEmptyRecordCount);
        self.with_caption_dialogs_refused(|this| {
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
            return self
                .with_caption_dialogs_refused(|this| this.fill_field_captions_with_row_unguarded(st, t, exprs, runs));
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
