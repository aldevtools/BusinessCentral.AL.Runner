use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

const FIELD_ARGUMENT_WITH_HANDLER: &str = "Page.Run or Page.RunModal with a field argument and a handler for the page (what the handler sees and what the field takes is not pinned)";

impl Session {
    /// The caption a CaptionClass text stands for. Area 3 is the literal text. Other numeric areas
    /// go to the platform's translation (`caption_class_translate`). An area that is not a number, or no
    /// area at all, goes to no built-in resolver: the System Application's Caption Class codeunit
    /// raises OnResolveCaptionClass for it, and when no subscriber claims it the text stays as
    /// written (corpus handlers/TestPageCaptionClass.al:90
    /// TestPageField_Caption_UnresolvedCaptionClass_ReturnsExpressionUnchanged;
    /// autoformat/TestCaptionClassResolverFailure.al: an error the subscriber raises reaches the
    /// open, and a subscriber that claims nothing changes nothing). The language handed to the
    /// event is the session language (types/PSCaptionClass.al).
    pub(super) fn caption_class_text(&mut self, st: &St, text: &str) -> Result<String, Signal> {
        self.refuse_caption_class_translate()?;
        if let Some(rest) = text.strip_prefix("3,") {
            if rest.is_empty() {
                // P84Captions.al Page_NoSubscriber c06: a control whose CaptionClass is '3,' reads the caption of another
                // control on the same field ('Ctl Plain'), not its own; which one BC takes is not pinned.
                return unsupported(
                    "CaptionClass '3,' with nothing after the comma on a page control: which caption BC reads is not pinned",
                );
            }
            return Ok(rest.to_string());
        }
        let (area, expr) = text.split_once(',').unwrap_or(("", text));
        // An empty area before the comma is not resolved either (P84Captions.al: c14 ',x', no event call).
        if text.contains(',') && (area.trim().parse::<u32>().is_ok() || area.is_empty()) {
            // A numeric area other than 3 (dimension, VAT, county, currency captions of Base Application) goes to the platform's
            // translation as FieldCaption does: Base Application's subscribers resolve it (record/lt-qd-captionclass-areas/
            // QdCaptionClassAreas.al:422-481 Page_ControlCaptions `lit20`, `lit53`, `pageExpr`), a text no subscriber resolves stays
            // as written, and a text the session translated before raises no event (pages/lt-page-report-events/P84Captions.al:
            // Page_NoSubscriber c01..c13, Page_Log_*, Page_Resolve_A/B after the table reads of the same texts).
            return self.caption_class_translate(text);
        }
        let _ = st;
        self.raise_resolve_caption_class(area, expr, text)
    }

    /// `CaptionClassTranslate(Text)`: area 3 gives the text after the first comma without an event ('3,One,Two' is
    /// `One,Two`, '3,' is empty); a text with no comma and the empty text come back unchanged without an event; any other
    /// area raises OnResolveCaptionClass of the Caption Class codeunit with the area, the rest after the first comma and the
    /// session language, and comes back unchanged when no subscriber resolves it (corpus types/PSCaptionClass.al:5-8,
    /// CaptionClassTranslate_Expressions).
    /// The session keeps what it translated: the same text a second time comes back as before and raises no event, claimed or
    /// not, until `SelectLatestVersion` (types/PSCaptionClass.al:94-115). Whether the kept caption holds after GlobalLanguage
    /// changed is not pinned: refused.
    pub(crate) fn caption_class_translate(&mut self, text: &str) -> Result<String, Signal> {
        self.refuse_caption_class_translate()?;
        let Some((area, expr)) = text.split_once(',') else {
            return Ok(text.to_string());
        };
        if area == "3" {
            return Ok(expr.to_string());
        }
        if let Some((_, language, caption)) = self.caption_translations.iter().find(|(t, _, _)| t == text) {
            if *language != self.global_language {
                return unsupported(
                    "CaptionClassTranslate of a text translated before GlobalLanguage changed (whether BC translates it again is not pinned)",
                );
            }
            let caption = caption.clone();
            return self.refuse_setup_caption(area, text, caption);
        }
        if matches!(area.trim(), "1" | "101") && !self.gl_setup_written {
            self.setup_caption_read_early = true;
        }
        let caption = self.raise_resolve_caption_class(area, expr, text)?;
        self.caption_translations
            .push((text.to_string(), self.global_language, caption.clone()));
        self.refuse_setup_caption(area, text, caption)
    }

    /// Base Application resolves areas 1 (dimensions) and 101 (currencies) from General Ledger Setup. The corpus database
    /// holds CRONUS's setup (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:254 `Amount (GBP)`, :256 `Department Code`);
    /// without Microsoft's test data the runtime's setup row is the one the install code writes, so a caption those areas
    /// resolved is refused. A text they leave as written does not read the setup (pages/lt-page-report-events/P84Captions.al:
    /// '1,1,x' and the other area 1 texts).
    /// A test that wrote General Ledger Setup before any caption of these areas was resolved in the session is answered for what
    /// it wrote: area 101 type 0 (the LCY Code) and area 1 type 1 reference 1 (Global Dimension 1 Code and its Dimension row)
    /// (QdCaptionClassAreas.al:267 `lit101` 'Amount (GBP)', :269 `lit11` 'Department Code', :311 `expr101`, after
    /// misc/lt-qk-setup-rows/QkSetupRows.al WriteGLSetupCaptions). The other types read fields that test does not write.
    fn refuse_setup_caption(&self, area: &str, text: &str, caption: String) -> Result<String, Signal> {
        let written = self.gl_setup_written && !self.setup_caption_read_early && {
            let expr = text.split_once(',').map_or("", |(_, e)| e);
            match area.trim() {
                "101" => expr.starts_with("0,"),
                "1" => expr == "1,1" || expr.starts_with("1,1,"),
                _ => false,
            }
        };
        if self.engine.test_data.is_none() && matches!(area.trim(), "1" | "101") && caption != text && !written {
            return unsupported(format!(
                "CaptionClass `{text}`: Base Application resolves area {area} from General Ledger Setup, whose corpus database row the runtime's database does not hold"
            ));
        }
        Ok(caption)
    }

    /// Every caption class the client translates raises `CaptionClassTranslate` of the platform codeunit `UI Helper Triggers`,
    /// area 3 included (events/lt-global-triggers-statements/P114GlobalTriggerEvents.al:301, :304, :323-327). The runtime does
    /// not raise it, so a translation while a subscriber listens is refused.
    pub(crate) fn refuse_caption_class_translate(&self) -> Result<(), Signal> {
        let test = self.test_object.and_then(|t| self.engine.program.objects.get(t));
        match self.engine.events.unraised_subscriber(
            "ui helper triggers",
            Some("captionclasstranslate"),
            test,
            self.test_object,
        ) {
            Some((_, sub)) => unsupported(format!(
                "a caption class is translated while {sub} subscribes to CaptionClassTranslate of UI Helper Triggers, which the runtime does not raise"
            )),
            None => Ok(()),
        }
    }

    pub(super) fn raise_resolve_caption_class(&mut self, area: &str, expr: &str, text: &str) -> Result<String, Signal> {
        self.note_event(crate::events::CAPTION_CLASS, "onresolvecaptionclass", None);
        if !self
            .engine
            .events
            .is_subscribed(crate::events::CAPTION_CLASS, "onresolvecaptionclass")
        {
            return Ok(text.to_string());
        }
        let names: Vec<String> = ["CaptionArea", "CaptionExpr", "Language", "Caption", "Resolved"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let mut args = vec![
            Value::Text(area.to_string().into()),
            Value::Text(expr.to_string().into()),
            Value::Integer(self.global_language),
            Value::Text(String::new().into()),
            Value::Boolean(false),
        ];
        let call = crate::dispatch::EventCall {
            object: crate::events::CAPTION_CLASS,
            event: "onresolvecaptionclass",
            element: None,
            isolated: false,
            names: &names,
            sender: None,
        };
        self.raise(&call, &mut args)?;
        Ok(match (&args[4], &args[3]) {
            (Value::Boolean(true), Value::Text(t) | Value::Code(t)) => t.to_string(),
            _ => text.to_string(),
        })
    }

    /// The expressions every row population evaluates, whatever the control's Visible
    /// (corpus autoformat/TestAutoFormatExpressionTiming.al:5-12): AutoFormatExpression and
    /// CaptionClass. They run after OnOpenPage and before OnAfterGetRecord.
    pub(super) fn populate_expressions(&mut self, st: &St) -> Result<(), Signal> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        for (idx, c) in page.controls.iter().enumerate() {
            for name in ["autoformatexpression", "captionclass"] {
                match c.prop(name) {
                    Some(routine @ (ir::PropVal::Routine(_) | ir::PropVal::Ext(..))) => {
                        let v = self
                            .prop_routine_value(st, idx, routine)?
                            .expect("a routine-valued property");
                        if name == "captionclass" {
                            self.refuse_caption_class_translate()?;
                            // Area 3 raises no event, so '3,' with nothing after it (a matrix column whose caption nobody set)
                            // costs the open nothing; only reading that caption is refused.
                            if let Value::Text(t) | Value::Code(t) = &v {
                                if !t.starts_with("3,") {
                                    // A text other than the one the control resolved before, at a fill other than the one after
                                    // SetValue (`refresh_caption_classes`, misc/W2CaptionClass.al:174-178): whether BC resolves it
                                    // there (after an action, on the next row) is not pinned.
                                    if super::format::raises_resolve(t)
                                        && st
                                            .borrow()
                                            .control_captions
                                            .iter()
                                            .any(|(i, old, _)| *i == idx && old != t.as_str())
                                    {
                                        return unsupported(
                                            "a CaptionClass expression that gives another text at a fill other than after SetValue (whether BC resolves it again there is not pinned)",
                                        );
                                    }
                                    self.fill_caption_class(st, idx, t)?;
                                }
                            }
                        }
                    }
                    // An area no built-in resolver claims goes to the Caption Class codeunit's
                    // OnResolveCaptionClass event (corpus autoformat/TestCaptionClassResolverFailure.al:1-12).
                    Some(ir::PropVal::Lit(l)) if name == "captionclass" => {
                        self.refuse_caption_class_translate()?;
                        let c = unquote(l);
                        if !c.starts_with("3,") {
                            self.fill_caption_class(st, idx, &c)?;
                        }
                    }
                    _ => {}
                }
            }
        }
        self.fill_field_caption_expressions(st)
    }

    /// A page open runs the CaptionClass expression of every field of the source table that has one, also those without a
    /// control, 20 times with an empty record, then once for each control over such a field with the current row; `Caption()`
    /// of the control reads that caption and runs nothing (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:422-481
    /// Page_ControlCaptions, a Card opened with OpenView: `open_calls = id0=240 ... rest=20@3;...`, `exprId = Id 3 call 245`
    /// twice; corpus bump 37 asserts only that empty-record calls happened, their number differs between BC runs). An empty
    /// result and `3,` read as the empty text on a control (`exprEmpty`, `expr3`). The other rows of the open and a Next() run
    /// nothing (`caption_expressions_pinned` names the pinned page shapes). Other page types and open modes, and the fills
    /// after the open other than Next(), mark the assumption page-open-caption-class-expressions.
    fn fill_field_caption_expressions(&mut self, st: &St) -> Result<(), Signal> {
        if !self.has_source(st) {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let t = self.table(&rec)?;
        let exprs: Vec<(usize, String)> = (0..t.fields.len())
            .filter_map(|pos| {
                let raw = self.field_property(&t, pos, "CaptionClass")?;
                crate::recmeta::string_literal(&raw)
                    .is_none()
                    .then(|| (pos, raw.trim().to_string()))
            })
            .collect();
        if exprs.is_empty() {
            return Ok(());
        }
        if let Some(why) = exprs
            .iter()
            .find_map(|(pos, _)| self.field_caption_class_refusal(&t, *pos))
        {
            return unsupported(why);
        }
        let pinned = self.caption_expressions_pinned(st);
        // An action of a Document page whose host holds a SetValue, with CurrPage.Update in its trigger: BC runs the host's
        // expressions three times, then the part's once (QdCaptionClassAreas.al:1015-1049 QO_Doc_OpenEdit_HostSteps
        // `update = rest=60@1;60@1;60@1;70@1;71@1;`, `update_exprA = Line 1`). The save before the trigger runs the host's once and
        // not the part's; the trigger's refresh runs the host's twice, then the part's. Without CurrPage.Update the part's
        // captions are not run again: not pinned (`action_save_captions_done`).
        let host_stage = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, _)| h.upgrade())
            .map(|h| h.borrow().field_captions_action_save);
        let own_stage = st.borrow().field_captions_action_save;
        // `staged`: this fill is one of those, so it marks nothing; `runs`: how many times the host's run.
        let (staged, runs) = match (pinned && st.borrow().field_captions_opened, own_stage, host_stage) {
            (true, 1, _) => (true, 1),
            (true, 2, _) => {
                st.borrow_mut().field_captions_action_save = 3;
                (true, 2)
            }
            (true, _, Some(1)) => {
                st.borrow_mut().field_captions_fill_first = false;
                return Ok(());
            }
            (true, _, Some(3)) => (true, 1),
            _ => (false, 1),
        };
        let first = std::mem::take(&mut st.borrow_mut().field_captions_fill_first);
        if !st.borrow().field_captions_opened {
            st.borrow_mut().field_captions_opened = true;
            if !pinned {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageOpenCaptionClassExpressions);
            }
            let Value::Record(empty) = self.default_value(&RtType::Record(t.object, false)) else {
                return unsupported("an empty record of the page's source table");
            };
            // How many times is no oracle (it differs between BC runs): every open that runs them is a flagged pass at best
            // (owner decision 2026-10-21).
            al_builtins::assumptions::assume(
                al_builtins::assumptions::Assumption::PageOpenCaptionClassEmptyRecordCount,
            );
            for _ in 0..20 {
                for (pos, expr) in &exprs {
                    self.field_caption_class_value(&t, *pos, Some(&empty), expr)?;
                }
            }
        } else {
            // The other rows the open reads and a Next() run no expression, and Caption() keeps the open's value
            // (QdCaptionClassAreas.al:591-630 QF_List_OpenView `rest=20@3;23@3;24@3;`, `next_calls = id0>0=No fields0= rest=`,
            // `next_exprId = Id 3` on row 5; QF_List_OpenEdit, QF_ListPart_OnCard, QF_Card_Next).
            // A refill after a SetValue of the host's link field that finds no row (a new line only) runs nothing
            // (QdCaptionClassAreas.al:1068-1100 QO_Doc_OpenNew `no_setvalue = empty>0=No ... rest=`, `no_exprA = Line 0`). After a
            // CurrPage.Update of the host the part's captions follow (Microsoft's ERM Purchase Order
            // PurchaseOrderChangePricesInclVATRefreshesPage, on a document without lines): that fill runs them.
            let empty_refill = first && {
                let s = st.borrow();
                s.field_captions_link_refill && (s.cursor != Cursor::Row || s.draft)
            };
            let quiet = empty_refill || (!first && (!st.borrow().field_captions_settled || st.borrow().moving_next));
            if !staged && (!pinned || !(quiet || first)) {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageOpenCaptionClassExpressions);
            }
            if quiet {
                return Ok(());
            }
        }
        self.fill_field_captions_with_row(st, &t, &exprs, runs)
    }

    /// Runs the CaptionClass expression of each control over such a field with the current row, `runs` times, and keeps the
    /// last caption for `Caption()`.
    fn fill_field_captions_with_row(
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
                let caption = match crate::recmeta::expression_class(&t.fields[*pos].name, &class)? {
                    crate::recmeta::ExprCaption::FieldCaption => String::new(),
                    crate::recmeta::ExprCaption::Text(c) => c,
                    crate::recmeta::ExprCaption::Translate => self.caption_class_translate(&class)?,
                };
                let mut s = st.borrow_mut();
                s.field_captions.retain(|(i, _)| *i != idx);
                s.field_captions.push((idx, caption));
            }
        }
        Ok(())
    }

    /// A move that saved the changed row runs the CaptionClass expressions once for each control with the saved row, before it
    /// moves on, and `Caption()` then reads those (QdCaptionClassAreas.al:986-1020 QO_Doc_OpenEdit_PartSteps `part_next =
    /// rest=70@1;71@1;`, :1110-1137 QO_Card_OpenEdit `next = rest=20@3;...`, corpus bump 45: a SetValue, then Next()). Another
    /// move after a change is not pinned.
    fn caption_expressions_after_save(&mut self, st: &St, next: bool) -> Result<(), Signal> {
        if !st.borrow().field_captions_opened {
            return Ok(());
        }
        // Pinned for a Card and a ListPart (on a Document page); a Document page's own Next() after a change is not.
        let shape = {
            use al_ir::PageType;
            let facts = &self.engine.object_facts;
            let s = st.borrow();
            match (facts[s.object as usize].page_type.as_ref(), s.host.is_some()) {
                (Some(PageType::Card), false) => true,
                (Some(PageType::Other(t)), true) => t == "listpart",
                _ => false,
            }
        };
        if !next || !shape {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageOpenCaptionClassExpressions);
            return Ok(());
        }
        st.borrow_mut().field_captions_fill_first = true;
        let r = self.fill_field_caption_expressions(st);
        st.borrow_mut().field_captions_fill_first = false;
        r
    }

    /// The page shapes whose CaptionClass expression runs are pinned (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al): a
    /// Card opened with OpenView (Page_ControlCaptions, QF_Card_Next), a List opened with OpenView, OpenEdit or by Page.Run
    /// (QF_List_OpenView, QF_List_OpenEdit, QF_List_PageRun) and a ListPart on a Card opened with OpenView (QF_ListPart_OnCard).
    fn caption_expressions_pinned(&self, st: &St) -> bool {
        use al_ir::PageType;
        let facts = &self.engine.object_facts;
        let page_type = |s: &St| facts[s.borrow().object as usize].page_type.as_ref();
        let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        let mode = st.borrow().mode;
        match (page_type(st), host) {
            // A Card opened with OpenEdit too (QdCaptionClassAreas.al:1110-1137 QO_Card_OpenEdit, corpus bump 45).
            (Some(PageType::Card), None) => matches!(mode, PageMode::View | PageMode::Edit),
            // A Document page and its ListPart opened with OpenEdit or OpenNew (QdCaptionClassAreas.al:958-1110, corpus bump 45).
            (Some(PageType::Document), None) => matches!(mode, PageMode::Edit | PageMode::New),
            (Some(PageType::List), None) => matches!(mode, PageMode::View | PageMode::Edit) && !st.borrow().modal,
            (Some(PageType::Other(t)), Some(h)) if t == "listpart" => {
                h.borrow().host.is_none()
                    && match page_type(&h) {
                        Some(PageType::Card) => h.borrow().mode == PageMode::View,
                        Some(PageType::Document) => matches!(h.borrow().mode, PageMode::Edit | PageMode::New),
                        _ => false,
                    }
            }
            _ => false,
        }
    }

    /// The FlowFields the page shows are calculated when a row loads. A temporary source row
    /// reads them from the database tables (corpus handlers/TestPageRunTempRecordStoredRow_Tests.al:
    /// TempRecordFlowFieldIsCalculatedFromTheDatabase).
    pub(super) fn calc_row_flowfields(&mut self, st: &St) -> Result<(), Signal> {
        if !self.has_source(st) {
            return Ok(());
        }
        let engine = self.engine();
        let rec = self.page_rec(st);
        let table = rec.borrow().table as usize;
        let Some(def) = engine.program.objects[table].table.as_ref() else {
            return Ok(());
        };
        // A card opened for viewing does not calculate a FlowField whose control says `Visible = false`
        // (pages/lt-cc-page-leads/CcPageLeads.al:722, FlowField_VisibleFalse_OnOpenView: `hidden=0`). Other page types and
        // open modes are not pinned (docs/probes-needed.md): they keep calculating it, as before, because refusing them loses
        // a baseline case (handlers/TestPartSubPageViewPrecompiled.al:104, a Base Application part with a hidden FlowField)
        // and the Microsoft pages that hide one.
        let hidden = |c: &ir::Control| matches!(c.prop("visible"), Some(ir::PropVal::Lit(v)) if v.trim().eq_ignore_ascii_case("false"));
        let view_card = st.borrow().mode == PageMode::View
            && self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::Card);
        let mut positions = Vec::new();
        for c in &page_of(&engine, st).controls {
            let ir::Binding::Field(p) = c.binding else { continue };
            if def.fields[p].calc.is_none() {
                continue;
            }
            if view_card && hidden(c) {
                continue;
            }
            positions.push(p);
        }
        positions.sort_unstable();
        positions.dedup();
        if positions.is_empty() {
            return Ok(());
        }
        if !rec.borrow().temporary {
            return self.calc_fields(&rec, &positions);
        }
        let probe = {
            let mut p = rec.borrow().clone();
            p.temporary = false;
            p.auto_calc.clear();
            Rc::new(RefCell::new(p))
        };
        for p in positions {
            self.check_calc_field(table, p)?;
            let v = self.calc_value(&probe, p, &mut Vec::new())?;
            rec.borrow_mut().fields[p] = v;
        }
        Ok(())
    }

    /// Populates the current row: the control expressions, then OnAfterGetRecord.
    pub(super) fn populate_current(&mut self, st: &St) -> Result<(), Signal> {
        st.borrow_mut().dirty = false;
        self.calc_row_flowfields(st)?;
        if !st.borrow().read_keeps_xrec {
            self.set_xrec(st);
        }
        self.populate_expressions(st)?;
        self.page_trigger(st, "onaftergetrecord", vec![])?;
        Ok(())
    }

    /// PageType = List (a worksheet or ListPlus page with a repeater is not one; corpus
    /// handlers/TestPageControlEditableRoutes.al:Worksheet_RunModal_LookupMode).
    pub(super) fn is_list_page(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        self.engine.object_facts[object].page_type == Some(al_ir::PageType::List)
    }

    pub(super) fn is_list(&self, st: &St) -> bool {
        let engine = self.engine();
        page_of(&engine, st)
            .controls
            .iter()
            .any(|c| c.kind == ir::ControlKind::Repeater)
    }

    pub(super) fn row_key(&self, rec: &Rec) -> Result<Vec<Value>, Signal> {
        let table = self.table(rec)?;
        Ok(table.primary_key(&rec.borrow().fields))
    }

    /// A repeater populates a window of rows at a time; a row inside the window it already
    /// holds is not populated again (corpus autoformat/TestAutoFormatExpressionTiming.al:5-12:
    /// a failing row 20 of 120 fails the open, row 30 does not, and a move to row 120
    /// populates a new window). The window is 20 rows: the corpus pins it only between 20 and 24.
    pub(super) const WINDOW: usize = 20;

    /// The window of a part that reads its rows last to first, once each, without reading the first row again. Pinned
    /// for 2 and 3 rows (pages/lt-part-reads/PartReadsOpen.al:OpenView_SixParts_ReadsPerPartLogged `open-Same`;
    /// misc/lt-probes-needed-7/PnGPartTriggers.al:OpenOrderOfHostWithSixParts); one row or more than 3 is refused.
    fn populate_window_backwards(
        &mut self,
        st: &St,
        start: Vec<Value>,
        why: al_builtins::value::ReadBackwards,
    ) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let mut rows: Vec<al_builtins::value::WindowRow> = Vec::new();
        loop {
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            rows.push(al_builtins::value::WindowRow {
                key,
                stored: stored.clone(),
                shown: stored,
            });
            if rows.len() > 3 || !self.nav_next(st, 1)? {
                break;
            }
        }
        // At open 2 and 3 rows are pinned (see `view_names_link_field`); after a move back 1, 2 and 3
        // (pages/lt-part-reads/PartReadsOpen.al:358-379).
        let after_move = why == al_builtins::value::ReadBackwards::AfterMove;
        if after_move && rows.len() > 3 {
            // Kept, not pinned: a part of more than 3 rows after its host moved back reads first to last, as every
            // refill did before the backward order was pinned (keeps misc/lt-probes-needed-7/PnGPartRows.al:
            // PendingPartRowSavedWhenHostMoves, which does not log those reads).
            self.ret_used = true;
            self.record_op(RecordOp::Get, &rec, &mut start.clone())?;
            return Ok(false);
        }
        if (rows.len() == 1 && !after_move) || rows.len() > 3 {
            return unsupported(
                "a part viewing its linked field with a number of rows other than 2 or 3 (reads not pinned)",
            );
        }
        for row in rows.iter_mut().rev() {
            rec.borrow_mut().fields = row.stored.clone();
            self.populate_current(st)?;
            row.shown = rec.borrow().fields.clone();
        }
        self.ret_used = true;
        self.record_op(RecordOp::Get, &rec, &mut start.clone())?;
        rec.borrow_mut().fields = rows[0].shown.clone();
        let first_stored = rows[0].stored.clone();
        self.set_xrec_fields(st, first_stored);
        st.borrow_mut().window = rows;
        Ok(true)
    }

    pub(super) fn populate_window(&mut self, st: &St) -> Result<(), Signal> {
        st.borrow_mut().window_from_start = false;
        st.borrow_mut().window_eof = false;
        st.borrow_mut().window_whole = false;
        st.borrow_mut().view_read_to_end = false;
        st.borrow_mut().window_read_up = false;
        let rec = self.page_rec(st);
        let start = self.row_key(&rec)?;
        let mut keys = Vec::new();
        let mut walked = 0;
        let mut end_reached = false;
        let opening = std::mem::take(&mut st.borrow_mut().landing_first);
        let backwards = st.borrow_mut().read_backwards.take();
        if let Some(why) = backwards
            && self.populate_window_backwards(st, start.clone(), why)?
        {
            return Ok(());
        }
        for i in 0..Self::WINDOW {
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            self.populate_current(st)?;
            keys.push(al_builtins::value::WindowRow {
                key,
                stored,
                shown: rec.borrow().fields.clone(),
            });
            if i + 1 == Self::WINDOW {
                break;
            }
            if !self.nav_next(st, 1)? {
                end_reached = true;
                break;
            }
            walked += 1;
        }
        // A view that ends inside the window: the client puts its first row back without asking
        // the page, reads it again, then asks OnNextRecord(-1) and OnFindRecord(`=`) and reads it
        // once more (corpus pageext/PCCurrRecordReads.al:List_ReadsAfterEachAction and
        // FindList_OnFindRecordAndOnNextRecord, `open` of a list of 3 rows: five reads). Only the
        // count is pinned, for a table of 3 rows.
        let find_page = self.declares(st, "onnextrecord") || self.declares(st, "onfindrecord");
        let view_ended_at_open = end_reached && opening;
        // The same tail for a full window (misc/lt-probes-needed-6/PnF06FindListMoves.al:FindList_Ascending_25Rows
        // `...;R20;R1;N(-1);F(=);R1;C`), for an ascending view.
        let end_reached = (end_reached || (find_page && keys.len() == Self::WINDOW)) && opening;
        if end_reached && find_page && self.declares(st, "onaftergetrecord") {
            // A descending view that opens on its last row steps back over the rows above it before OnAfterGetCurrRecord
            // (misc/lt-probes-needed-6/PnF06FindListMoves.al:304, :318: `R1;N(1);N(-1);R2;N(-1);R3;...;F(=);R1;C`). Pinned when
            // the first row read is the last of the view; other descending opens are refused.
            if !rec.borrow().ascending && self.declares(st, "onaftergetcurrrecord") {
                if keys.len() != 1 || !self.declares(st, "onnextrecord") || st.borrow().host.is_some() {
                    return unsupported(
                        "a list that serves a descending view from OnFindRecord and OnNextRecord, with OnAfterGetCurrRecord, opening on a row that is not the last of the view (the step back is not pinned)",
                    );
                }
                return self.read_rows_up_from_the_last(st, keys);
            }
            if let Some(first) = keys.first() {
                rec.borrow_mut().fields = first.stored.clone();
            }
            // A view of one row is not read again before OnNextRecord(-1) (pages/lt-ai-list-order/AiListOrderTests.al:535
            // Next_OneRow_Nav `F:=><=Yes;R:R1001;N:1=0;N:-1=0`, :565 Next_OneRow_NavCurr; pages/lt-an-testpage-640/
            // AnTestPageTests.al:405 `F=><>A;RA;N1>0A;N-1>0A;`); two rows are (AiListOrderTests.al:600 Next_TwoRows_Nav).
            if keys.len() > 1 {
                self.page_trigger(st, "onaftergetrecord", vec![])?;
            }
            // After an open read that ran to the end of the view inside the window, Next on the last row asks the page
            // nothing (AiListOrderTests.al:600, :630 Next_TwoRows_Nav/_NavCurr `next2_log` empty; :843
            // OpenOnRow_SetPositionInOnOpenPage_LastRow).
            st.borrow_mut().view_read_to_end = view_ended_at_open && keys.len() < Self::WINDOW;
            let stepped_back = self.declares(st, "onnextrecord") && self.nav_next(st, -1)?;
            // The closing OnFindRecord(`=`) and its read belong to the OnAfterGetCurrRecord that follows; a
            // list without that trigger ends after OnNextRecord(-1) when that answers 0 (pages/lt-f3-pages/F3ListTests.al:24
            // Nav_TwelveRows_OpenLastPreviousFirstNext `open`, a list of 12 rows with OnFindRecord, OnNextRecord and
            // OnAfterGetRecord; pages/lt-ai-list-order/AiListOrderTests.al:909 Paging_Open_Nav, 25 rows: the same end after
            // a full window; pages/lt-f3-pages/F3PartTests.al:23 FilterView_OneHost_OpenAndMoves, a part of 3 rows).
            let with_current = self.declares(st, "onaftergetcurrrecord");
            let mut reread = with_current;
            if !with_current && stepped_back {
                if keys.len() < Self::WINDOW {
                    return self.read_rows_before_the_first(st, keys);
                }
                // A full window and rows before its first one: not pinned. Base Application's "Base Calendar Entries
                // Subform" (page 7604) keeps its position in a global that OnAfterGetRecord sets, and Microsoft's
                // ERM Base Calendar BaseCalendarEntriesShowCorrespondingRecords (codeunit 134233) needs that position back
                // on the first row once the page has opened: assumed to end as a list with OnAfterGetCurrRecord does
                // (misc/lt-probes-needed-6/PnF06FindListMoves.al:FindList_Ascending_25Rows `R20;R1;N(-1);F(=);R1;C`).
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ListOpenFullWindowRowsBefore);
                reread = true;
            }
            if reread && self.declares(st, "onfindrecord") {
                self.nav_find(st, "=")?;
            }
            if let Some(row) = keys.first() {
                rec.borrow_mut().fields = row.shown.clone();
                self.set_xrec_fields(st, row.stored.clone());
            }
            st.borrow_mut().window = keys;
            if reread {
                self.page_trigger(st, "onaftergetrecord", vec![])?;
            }
            return Ok(());
        }
        // Back on the row the cursor is on. A page that serves its rows from triggers steps back
        // through them: the row is not necessarily in the table.
        if self.declares(st, "onnextrecord") || self.declares(st, "onfindrecord") {
            if !self.declares(st, "onaftergetrecord") && !self.declares(st, "onaftergetcurrrecord") && walked > 0 {
                // A find page with no row trigger puts its first row back and probes once (pages/lt-testpage-list-rows/ListRowsOpenNone.al:
                // OpenView_FindNext5Rows_ReadsAtOpen `F0;N2;N3;N4;N5;N5;N1`, _FindNext30Rows `...;N20;N1`).
                // The client puts its first row back and asks OnNextRecord(-1) once, which answers 0 there.
                if let Some(first) = keys.first() {
                    rec.borrow_mut().fields = first.stored.clone();
                }
                self.nav_next(st, -1)?;
            } else {
                for _ in 0..walked {
                    if !self.nav_next(st, -1)? {
                        return unsupported("OnNextRecord did not step back over the rows it served");
                    }
                }
            }
        } else {
            self.ret_used = true;
            self.record_op(RecordOp::Get, &rec, &mut start.clone())?;
        }
        // The row buffer the page keeps for the row it stands on is what OnAfterGetRecord left.
        if let Some(row) = keys.first() {
            rec.borrow_mut().fields = row.shown.clone();
            self.set_xrec_fields(st, row.stored.clone());
        }
        st.borrow_mut().window = keys;
        // The row the cursor stands on is read once more after the window: a list of 60 rows
        // raises OnAfterGetRecord 21 times when it opens, rows 1 to 20 and then row 1 again
        // (corpus handlers/TestPageRuntimeContracts.al:121-136 TestPage_List_OpenReads21Rows).
        // A view of one row needs no repositioning read: the window ends on the row the cursor stands on (misc/
        // lt-probes-needed-7/PnGNewRecord.al:HandedRecordWithOnlyPrimaryKeyValue, one row read twice, against HandedRecordNoFilters,
        // two rows read four times).
        // A list without OnAfterGetCurrRecord reads one row more, not two: the window and row 1 again, whether the view ends inside the
        // window or not (pages/lt-testpage-list-rows/ListRowsOpenAG.al:OpenView_OnlyRec5Rows_ReadsAtOpen `1-5,1`,
        // OpenView_OnlyRec30Rows_ReadsAtOpen `1-20,1`; with both triggers: `1-5,1,1` then the current-record read).
        let agcr = self.declares(st, "onaftergetcurrrecord");
        let single = !agcr && st.borrow().host.is_none() && end_reached;
        // A view of one row without OnAfterGetCurrRecord is read once, on a page and in a part (pages/lt-ai-list-order/
        // AiListOrderTests.al:655 Next_OneRow_Plain `open` = `R:R1001`; pages/lt-part-subpageview-filter/PartViewFilter.al:317,
        // :336 `open-ConstTen` = `AGR(H1/10)` once).
        let one_row_read_once = walked == 0 && !agcr && end_reached;
        if (walked > 0 || st.borrow().host.is_some()) && !single && !one_row_read_once {
            self.page_trigger(st, "onaftergetrecord", vec![])?;
        }
        // A part with OnAfterGetCurrRecord whose view ends inside the window reads its first row twice more, as a list does
        // (pages/lt-an-testpage-640/AnTestPageTests.al:168, :210 An02_Part_ConstLink_FiveRows, _FieldLink_FiveRows:
        // `P.R10;...;P.R50;P.R10;P.R10;P.C10`); with one row once (:147, :189 `P.R10;P.R10;P.C10`).
        if st.borrow().host.is_some() && agcr && end_reached && walked > 0 {
            self.page_trigger(st, "onaftergetrecord", vec![])?;
        }
        // A list that declares OnAfterGetCurrRecord reads its first row once more before that trigger, whatever its length, and so
        // does a list whose view ends inside the window; a part does not (misc/lt-probes-needed-7/PnGPartTriggers.al:
        // OpenOrderOfHostAndPart: two rows read as 10000, 20000 and 10000; pages/W1Navigation.al:GoToKey_List_FiftyRows_RowsReadByAFarJump
        // `open`: rows 1 to 20, row 1, row 1). A list without the trigger reads 21 times: handlers/TestPageRuntimeContracts.al:
        // TestPage_List_OpenReads21Rows.
        if (end_reached || (opening && self.declares(st, "onaftergetcurrrecord")))
            && st.borrow().host.is_none()
            && !one_row_read_once
        {
            self.page_trigger(st, "onaftergetrecord", vec![])?;
        }
        Ok(())
    }

    /// The open read of a descending find list with OnAfterGetCurrRecord whose first row read is the last of the view: the client
    /// asks OnNextRecord(-1) and reads each row above, at most a window of 20 (no OnNextRecord(-1) after the 20th), then reads the
    /// row it opened on, asks OnFindRecord(`=`) and reads it again; OnAfterGetCurrRecord follows (misc/lt-probes-needed-6/
    /// PnF06FindListMoves.al:304 FindList_Descending_3Rows `F(=><);R1;N(1);N(-1);R2;N(-1);R3;N(-1);R1;F(=);R1;C`, :318
    /// _25Rows `...;R20;N(-1);R21;R1;F(=);R1;C`). The window holds the rows in client order, the cursor on its last row.
    fn read_rows_up_from_the_last(
        &mut self,
        st: &St,
        forward: Vec<al_builtins::value::WindowRow>,
    ) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let mut above = Vec::new();
        let mut whole = true;
        if self.nav_next(st, -1)? {
            loop {
                let key = self.row_key(&rec)?;
                let stored = rec.borrow().fields.clone();
                self.populate_current(st)?;
                above.push(al_builtins::value::WindowRow {
                    key,
                    stored,
                    shown: rec.borrow().fields.clone(),
                });
                if above.len() == Self::WINDOW {
                    whole = false;
                    break;
                }
                if !self.nav_next(st, -1)? {
                    break;
                }
            }
        }
        let first = forward[0].clone();
        rec.borrow_mut().fields = first.stored.clone();
        // A view of one row: nothing above it, so the row is not read again before `=` (PnF06FindListMoves.al:348
        // PW_FindList_Descending_1Rows `F(=><);R1;N(1);N(-1);F(=);R1;C`; two rows read it again, :362).
        if !above.is_empty() {
            self.populate_current(st)?;
        }
        self.nav_find(st, "=")?;
        above.reverse();
        above.extend(forward);
        {
            let mut s = st.borrow_mut();
            s.window = above;
            s.window_read_up = true;
            s.window_whole = whole;
            s.view_read_to_end = true;
        }
        self.page_trigger(st, "onaftergetrecord", vec![])?;
        let shown = rec.borrow().fields.clone();
        let mut s = st.borrow_mut();
        if let Some(last) = s.window.last_mut() {
            last.shown = shown;
        }
        drop(s);
        self.set_xrec_fields(st, first.stored);
        Ok(())
    }

    /// `First` on a descending find list whose window was read up from its last row (see `read_rows_up_from_the_last`) and
    /// holds a full block: the client reads the top row it holds again, then asks OnNextRecord(-1) and reads each row above until
    /// it answers 0, asks OnFindRecord(`=`) for the top row and reads it again (misc/lt-probes-needed-6/PnF06FindListMoves.al:318
    /// FindList_Descending_25Rows `desc25_first` = `R21;N(-1);R22;...;R25;N(-1);F(=);R25;C`). When there is no row above, the top
    /// row is read once more before `=` (:404 PW_FindList_Descending_21Rows `desc21_first` = `R21;N(-1);R21;F(=);R21;C`). A second
    /// full block is refused.
    fn first_above_the_window(&mut self, st: &St) -> Result<Option<bool>, Signal> {
        let before = self.xrec_before_move(st);
        let rec = self.page_rec(st);
        let top = st.borrow().window[0].stored.clone();
        rec.borrow_mut().fields = top;
        self.populate_current(st)?;
        let mut new_rows = Vec::new();
        while self.nav_next(st, -1)? {
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            self.populate_current(st)?;
            new_rows.push(al_builtins::value::WindowRow {
                key,
                stored,
                shown: rec.borrow().fields.clone(),
            });
            if new_rows.len() == Self::WINDOW {
                return unsupported(
                    "First on a descending list with a second full block of rows above the window (not pinned)",
                );
            }
        }
        if new_rows.is_empty() {
            self.populate_current(st)?;
        }
        new_rows.reverse();
        {
            let mut s = st.borrow_mut();
            new_rows.append(&mut s.window);
            s.window = new_rows;
            s.window_whole = true;
        }
        self.nav_find(st, "=")?;
        self.populate_current(st)?;
        let shown = rec.borrow().fields.clone();
        if let Some(w) = st.borrow_mut().window.first_mut() {
            w.shown = shown;
        }
        self.current_record_trigger(st, before)?;
        Ok(Some(true))
    }

    /// The end of the open read of a list with OnFindRecord, OnNextRecord and OnAfterGetRecord (no OnAfterGetCurrRecord) whose
    /// view ended inside the window and whose OnNextRecord(-1) from the first row read answered a step: the client reads each
    /// row it steps back to and asks OnNextRecord(-1) again until it answers 0; the cursor stays on the row read first, and the
    /// rows the client holds now start at the first row of the view (pages/lt-ai-list-order/AiListOrderTests.al:777
    /// OpenOnRow_SetPositionInOnOpenPage `...;R:R1006;N:1=0;R:R1004;N:-1=-1;R:R1003;N:-1=-1;R:R1002;N:-1=-1;R:R1001;N:-1=0`,
    /// `current` R1004, `first_log` empty). `forward` holds the rows read from the cursor on. Pinned for a page of its own with 6
    /// rows; a part, and more rows than a window in all, are refused.
    fn read_rows_before_the_first(
        &mut self,
        st: &St,
        forward: Vec<al_builtins::value::WindowRow>,
    ) -> Result<(), Signal> {
        if st.borrow().host.is_some() {
            return unsupported(
                "the open read of a part with OnFindRecord and OnNextRecord that has rows before the first one read (not pinned)",
            );
        }
        let rec = self.page_rec(st);
        let mut before = Vec::new();
        loop {
            if before.len() + forward.len() >= Self::WINDOW {
                return unsupported(
                    "the open read of a list that steps back over more rows than a window holds (not pinned)",
                );
            }
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            self.populate_current(st)?;
            before.push(al_builtins::value::WindowRow {
                key,
                stored,
                shown: rec.borrow().fields.clone(),
            });
            if !self.nav_next(st, -1)? {
                break;
            }
        }
        before.reverse();
        let first = forward[0].clone();
        rec.borrow_mut().fields = first.shown;
        self.set_xrec_fields(st, first.stored);
        before.extend(forward);
        st.borrow_mut().window = before;
        Ok(())
    }

    /// A GoToKey that goes past the rows a list has read reads on in blocks: the last two rows of the
    /// block before, then 20 new rows (fewer at the end of the view). The cursor stands on the first
    /// unread row `X` and the window holds every row before it. Returns whether the block was full, the key
    /// of its last row, and leaves the cursor on `X` again with the whole block in the window.
    /// Pinned on a real service tier by corpus handlers/TestPageSystemPartListWindowContracts.al:194
    /// (List_Window_AfterGoToKeyOutsideTheRowsRead), :210 (List_Window_SmallTable) and
    /// handlers/TestPageRuntimeContracts.al:331.
    pub(super) fn read_window_block(&mut self, st: &St) -> Result<(bool, Vec<Value>), Signal> {
        let rec = self.page_rec(st);
        let start = self.row_key(&rec)?;
        let prefix = st.borrow().window.len().min(2);
        let trigger_rows = self.declares(st, "onnextrecord") || self.declares(st, "onfindrecord");
        if prefix > 0 && !self.nav_next(st, -(prefix as i32))? {
            return unsupported("a list that cannot step back over the rows it has read");
        }
        let mut walked = 0usize;
        let mut new_rows = 0usize;
        let mut last_key = start.clone();
        for i in 0..prefix + Self::WINDOW {
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            self.populate_current(st)?;
            let row = al_builtins::value::WindowRow {
                key: key.clone(),
                stored,
                shown: rec.borrow().fields.clone(),
            };
            {
                let mut s = st.borrow_mut();
                match s.window.iter().position(|w| Self::same_key(&w.key, &key)) {
                    Some(at) => s.window[at] = row,
                    None => s.window.push(row),
                }
            }
            if i >= prefix {
                new_rows += 1;
                last_key = key;
            }
            if i + 1 == prefix + Self::WINDOW {
                break;
            }
            if !self.nav_next(st, 1)? {
                break;
            }
            walked += 1;
        }
        // Back on `X`, which is `prefix` rows into the walk.
        if trigger_rows {
            for _ in prefix..walked {
                if !self.nav_next(st, -1)? {
                    return unsupported("OnNextRecord did not step back over the rows it served");
                }
            }
        } else {
            self.ret_used = true;
            self.record_op(RecordOp::Get, &rec, &mut start.clone())?;
        }
        Ok((new_rows == Self::WINDOW, last_key))
    }

    /// The last two rows of the window are read once more: what a read that reached the end of the view
    /// does (corpus handlers/TestPageSystemPartListWindowContracts.al:194, handlers/TestPageRuntimeContracts.al:331).
    /// The cursor is on the last row of the view and stays there.
    pub(super) fn reread_last_rows(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let n = st.borrow().window.len();
        if n >= 2 {
            if !self.nav_next(st, -1)? {
                return unsupported("a list that cannot step back over its last row");
            }
            self.populate_current(st)?;
            self.nav_next(st, 1)?;
        }
        self.populate_current(st)?;
        let _ = rec;
        st.borrow_mut().window_eof = true;
        Ok(())
    }

    pub(super) fn in_window(&self, st: &St, key: &[Value]) -> bool {
        st.borrow().window.iter().any(|k| Self::same_key(&k.key, key))
    }

    pub(super) fn same_key(a: &[Value], b: &[Value]) -> bool {
        a.len() == b.len()
            && a.iter().zip(b).all(|(a, b)| {
                ops::compare(a, b)
                    .map(|o| o == std::cmp::Ordering::Equal)
                    .unwrap_or(false)
            })
    }

    /// The cursor has landed on a stored row: populate it as the page type does, then
    /// OnAfterGetCurrRecord.
    pub(super) fn arrive(&mut self, st: &St) -> Result<(), Signal> {
        let before = self.xrec_before_move(st);
        // A stored row is not the started record the cursor may have left (corpus
        // handlers/TestPageOpenNewPartLink.al:OpenNew_LinesInsertedInCode_LastThenWriteLandsOnTheLastLine).
        st.borrow_mut().draft = false;
        st.borrow_mut().stale = false;
        if self.is_list(st) {
            let key = self.row_key(&self.page_rec(st))?;
            if !self.in_window(st, &key) {
                self.populate_window(st)?;
            } else {
                // A row of the window shows the buffer OnAfterGetRecord left, as long as the stored
                // row is still the one that buffer was made from.
                let rec = self.page_rec(st);
                let row = st
                    .borrow()
                    .window
                    .iter()
                    .find(|w| Self::same_key(&w.key, &key))
                    .filter(|w| Self::same_values(&w.stored, &rec.borrow().fields))
                    .cloned();
                match row {
                    Some(w) => {
                        rec.borrow_mut().fields = w.shown;
                        self.set_xrec_fields(st, w.stored);
                    }
                    None => self.set_xrec(st),
                }
            }
        } else {
            self.populate_current(st)?;
        }
        self.current_record_trigger(st, before)
    }

    /// The xRec the page holds before a move: the row the cursor stood on.
    pub(super) fn xrec_before_move(&self, st: &St) -> Option<Vec<Value>> {
        (self.has_source(st) && st.borrow().instance.is_some()).then(|| self.page_xrec_fields(st))
    }

    /// OnAfterGetCurrRecord sees as xRec the row the cursor came from (a move from row 1 to row 2 logs `AGCR(R2, R1)`, the first
    /// row of an opened page has a blank xRec); once it has run xRec is the row itself (misc/lt-probes-needed-7/PnGGoToScan.al:
    /// GoToKeyFromTheFirstRow).
    pub(super) fn current_record_trigger(&mut self, st: &St, before: Option<Vec<Value>>) -> Result<(), Signal> {
        if let Some(prev) = before {
            self.set_xrec_fields(st, prev);
        }
        let r = self.page_trigger(st, "onaftergetcurrrecord", vec![]);
        self.set_xrec(st);
        r?;
        self.page_event(st, "onaftergetcurrrecordevent", None, None)?;
        Ok(())
    }

    /// Moves the cursor; the Boolean the move methods return.
    pub(super) fn page_move(&mut self, st: &St, how: PageMethod) -> Result<Value, Signal> {
        if !self.has_source(st) {
            return unsupported(format!("{how}() on a page without a source table"));
        }
        if self.open_new_list_untouched(st) {
            return self.move_on_open_new_list(how);
        }
        // A list opened for view over an empty view holds no row: First answers No and reads nothing, also after the test stored a
        // row the view admits (misc/lt-probes-needed-7/PnGShowRow.al:494 QG_ViewEmptyList_RowStoredByTest_ThenFirst `first` No,
        // `first_no` and `first_log` empty; pages/W1ShowRow.al:277 FilteredList_InsertedRowTheViewHides). An action shows the row
        // (`empty_page_after_action`). The other moves are not pinned there.
        let empty_view_list = {
            let s = st.borrow();
            s.open
                && s.cursor == Cursor::None
                && s.mode == PageMode::View
                && s.window.is_empty()
                && s.host.is_none()
                && s.parts.is_empty()
        } && self.is_list(st)
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord");
        if empty_view_list && how == PageMethod::First {
            return Ok(Value::Boolean(false));
        }
        let answer_used = self.ret_used;
        // A list row whose OnValidate refused an entry: Next() raises nothing, runs no OnModify and stores nothing of the row
        // (pages/lt-ah-pages/PwCloseRevalidate.al:270 PW_List_RefusedOnValidate_ThenNext_ThenClose). Where the cursor lands
        // and what Next() answers are not pinned, and neither are the other moves.
        let refused_entry = {
            let s = st.borrow();
            !s.trigger_errors.is_empty() && !s.errors.is_empty() && s.dirty
        };
        if refused_entry && self.is_list(st) {
            if how != PageMethod::Next || answer_used {
                return unsupported(format!(
                    "{how}() on a list row whose OnValidate refused an entry (only Next() with its answer unused is pinned)"
                ));
            }
            let mut s = st.borrow_mut();
            s.draft = false;
            s.dirty = false;
            s.unpinned_state = Some(
                "a list after Next() dropped a row whose OnValidate refused an entry (where it stands is not pinned)"
                    .into(),
            );
            return Ok(Value::Boolean(false));
        }
        // Last and First on a part whose whole view the client holds (`part_last_first`).
        if st.borrow().after_new.is_none() {
            if let Some(r) = self.part_last_first(st, how)? {
                return Ok(Value::Boolean(r));
            }
        }
        // Leaving a row saves it.
        let saves_row = {
            let s = st.borrow();
            s.cursor == Cursor::Row && s.dirty && !s.draft
        } && !Self::same_values(&self.page_rec(st).borrow().fields, &self.page_xrec_fields(st));
        let anchor = st.borrow_mut().after_new.take();
        let writes = self.write_mark();
        let (left, origin) = self.traced(st, |s| s.leave_row(st, how));
        let left = match (left, origin.as_deref()) {
            // corpus pages/lt-cc-page-leads/CcPageLeads.al:598-608 TriggerError_OnModifyRecord: Next() raises nothing. What it
            // answers is not pinned: refused when the caller uses the answer.
            (Err(Signal::Error(e)), Some("onmodifyrecord")) if how == PageMethod::Next => {
                self.swallowed_trigger_error(st, "OnModifyRecord", "Next()", &e, writes)?;
                // What GetLastErrorText reads after the swallowed error is not pinned.
                self.last_error = al_builtins::native::unpinned_error();
                if answer_used {
                    return unsupported("the answer of Next() after an error in OnModifyRecord (not pinned)");
                }
                return Ok(Value::Boolean(false));
            }
            (Err(Signal::Error(_)), Some("onmodifyrecord")) => {
                return unsupported(format!("an error in OnModifyRecord on {how}() (only Next() is pinned)"));
            }
            (left, _) => left?,
        };
        if saves_row {
            self.caption_expressions_after_save(st, how == PageMethod::Next)?;
        }
        if let Left::Stay = left {
            return Ok(Value::Boolean(false));
        }
        // A new line saved in the middle of a list: Next() goes to the row below the one it was
        // started under, not to the row after its own key (corpus
        // handlers/ProbeTestPageForms.al:227-253, where a row with the last key, added below row 2,
        // is followed by row 3 and not by the blank line).
        let cursor_now = st.borrow().cursor;
        // The client keeps a saved new line where it was typed, below its anchor. Where First or Previous go when that is not the
        // row's place in the key order is not pinned (record/lt-loadfields-pagesource-validate/P81PageTempSource.al:238-242
        // RunModal_TempRecord_NewRow: `N1`, typed below `T1`, is the second row the client shows): refused. Last goes to the last
        // row by key (misc/lt-probes-needed-7/PnGTempSource.al:101 NewRowOfTheTemporarySource `last-no = C`); Next is handled below.
        if let (PageMethod::First | PageMethod::Previous, Some(anchor), Cursor::Row) = (how, &anchor, cursor_now) {
            if self.saved_row_out_of_key_order(st, anchor.clone())? {
                return unsupported(format!(
                    "{how}() after a row saved by New() that is not next in key order to the row it was started below (the client's row order is not implemented)"
                ));
            }
        }
        if let (PageMethod::Next, Some(anchor), Cursor::Row) = (how, anchor, cursor_now) {
            if self.next_below_anchor(st, anchor)? {
                st.borrow_mut().cursor = Cursor::Row;
                self.arrive(st)?;
                return Ok(Value::Boolean(true));
            }
        }
        let before = self.page_rec(st).borrow().fields.clone();
        st.borrow_mut().moving_next = how == PageMethod::Next;
        let (r, origin) = self.traced(st, |s| s.page_move_inner(st, how));
        st.borrow_mut().moving_next = false;
        if let (Err(Signal::Error(e)), Some(trigger @ ("onnextrecord" | "onfindrecord"))) = (&r, origin.as_deref()) {
            let raised = self.move_trigger_error(st, how, trigger, e, before);
            self.mark_closed(st);
            return Err(raised?);
        }
        self.torn_down_on_error(st, r).map(Value::Boolean)
    }

    /// An AL error of OnNextRecord on `Next()` of a card opened for editing: "Page <caption> has to close.", the caption with
    /// the key of the row the page stood on (corpus pages/lt-cc-page-leads/CcPageLeads.al:559-569 TriggerError_OnNextRecord
    /// `Page LT CC Err Card - 1 has to close.`), and the page is closed. The same error of another move, of OnFindRecord, on
    /// another page type or mode, or after the trigger moved the record, is not pinned: its text and code cannot be read.
    fn move_trigger_error(
        &mut self,
        st: &St,
        how: PageMethod,
        trigger: &str,
        e: &al_builtins::native::AlError,
        before: Vec<Value>,
    ) -> Result<Signal, Signal> {
        let unpinned = || Signal::Error(Box::new(al_builtins::native::unpinned_error()));
        let pinned_shape = how == PageMethod::Next
            && trigger == "onnextrecord"
            && self.is_card(st)
            && st.borrow().mode == PageMode::Edit
            && !st.borrow().modal
            && st.borrow().host.is_none()
            && !self.is_request_page(st);
        if !pinned_shape || !Self::same_values(&self.page_rec(st).borrow().fields, &before) {
            return Ok(unpinned());
        }
        let caption = self.test_page_caption(st)?;
        Ok(self.has_to_close(st, e, caption))
    }

    /// Whether the row the page stands on was saved by New() below `anchor` and is not the row after `anchor` in key order. The
    /// client keeps such a row where it was started (record/lt-loadfields-pagesource-validate/P81PageTempSource.al:238-242
    /// RunModal_TempRecord_NewRow: N1 started below T1 shows as row 2), so key order and the client's order differ. A page that
    /// serves its rows from OnFindRecord or OnNextRecord is not asked.
    fn saved_row_out_of_key_order(&mut self, st: &St, anchor: Vec<Value>) -> Result<bool, Signal> {
        if self.declares(st, "onnextrecord") || self.declares(st, "onfindrecord") {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        if self.stored_row(&rec)?.is_none() {
            return Ok(false);
        }
        let saved = rec.borrow().fields.clone();
        let table = self.table(&rec)?;
        let saved_key = table.primary_key(&saved);
        rec.borrow_mut().fields = anchor;
        self.ret_used = true;
        let mut pk = table.primary_key(&rec.borrow().fields);
        let found = matches!(self.record_op(RecordOp::Get, &rec, &mut pk)?, Value::Boolean(true));
        let next_is_saved = found && {
            self.ret_used = true;
            matches!(self.record_op(RecordOp::Next, &rec, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0)
                && Self::same_key(&table.primary_key(&rec.borrow().fields), &saved_key)
        };
        rec.borrow_mut().fields = saved;
        Ok(found && !next_is_saved)
    }

    /// A list page of its own opened with OpenNew whose cursor still stands on the untouched new row.
    pub(super) fn open_new_list_untouched(&self, st: &St) -> bool {
        let untouched = {
            let s = st.borrow();
            s.mode == PageMode::New && s.host.is_none() && s.draft && !s.dirty
        };
        untouched && self.is_list(st)
    }

    /// A move on a list opened with OpenNew, before anything is typed: First and Next answer No, run no trigger and leave the
    /// cursor on the new row (pages/lt-page-report-events/P84ListTemp.al:396-401 TestPage_Real_OpenNew `tp_new3_count = 0`,
    /// `tp_new3_last_id = 0`; pages/lt-opennew-findrecord/OpenNewFindRecord.al:227-229 List_OpenNew_TableWithRows
    /// `first_row_after_new = No`, `close_trace` empty; pages/lt-filter-change-new-line/FilterChangeNewLine.al:191-195
    /// OpenNew_FilterSetFilter_ThenNext_AndFirst: First, Next and Previous answer No, no trigger runs). Last is not pinned.
    fn move_on_open_new_list(&mut self, how: PageMethod) -> Result<Value, Signal> {
        match how {
            PageMethod::First | PageMethod::Next | PageMethod::Previous => Ok(Value::Boolean(false)),
            _ => unsupported(format!(
                "{how}() on a list opened with OpenNew before a row is saved (not pinned)"
            )),
        }
    }

    /// An error escaping a row population tears the page down. On a card the move reads as
    /// "not open"; on a list it raises the error itself (corpus
    /// autoformat/TestAutoFormatExpressionTiming.al, RowFormat_List_LastOntoAFailingRowBeyondTheWindow).
    pub(super) fn torn_down_on_error<T>(&mut self, st: &St, r: Result<T, Signal>) -> Result<T, Signal> {
        match r {
            Err(Signal::Error(e)) => {
                self.mark_closed(st);
                if self.is_list(st) {
                    Err(Signal::Error(e))
                } else {
                    Err(error_code(NOT_OPEN, "TestPageNotOpened"))
                }
            }
            other => other,
        }
    }

    /// Moves the page's record to the row after `anchor` in key order, when there is one and it is
    /// not the row just saved; else leaves the record as it was and answers false.
    pub(super) fn next_below_anchor(&mut self, st: &St, anchor: Vec<Value>) -> Result<bool, Signal> {
        if self.declares(st, "onnextrecord") || self.declares(st, "onfindrecord") {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let saved = rec.borrow().fields.clone();
        rec.borrow_mut().fields = anchor;
        self.ret_used = true;
        let mut pk = self.table(&rec)?.primary_key(&rec.borrow().fields);
        let found = matches!(self.record_op(RecordOp::Get, &rec, &mut pk)?, Value::Boolean(true));
        let moved = found && {
            self.ret_used = true;
            matches!(self.record_op(RecordOp::Next, &rec, &mut [Value::Integer(1)])?, Value::Integer(n) if n != 0)
        };
        let table = self.table(&rec)?;
        let same = moved && Self::same_values(&table.primary_key(&rec.borrow().fields), &table.primary_key(&saved));
        if moved && !same {
            return Ok(true);
        }
        rec.borrow_mut().fields = saved;
        Ok(false)
    }

    /// A move of a list whose whole view the client has read (fewer rows than a window) is served
    /// from the client's rows: the new row is read, a page with OnFindRecord is asked `=` for it,
    /// the row is read again, and OnAfterGetCurrRecord ends the move; `Last` on a page without
    /// OnFindRecord starts with an OnAfterGetCurrRecord (corpus pageext/PCCurrRecordReads.al:
    /// List_ReadsAfterEachAction `next`, `last`, `first`; FindList_OnFindRecordAndOnNextRecord
    /// `next`, `last`, `previous`, `first`, pinned for a table of 3 rows). `None` when the move is not
    /// one of those.
    pub(super) fn move_in_client_rows(&mut self, st: &St, how: PageMethod) -> Result<Option<bool>, Signal> {
        let find_page = self.declares(st, "onfindrecord");
        if !(self.declares(st, "onaftergetrecord") || self.filtered_find_list_without_row_triggers(st))
            || !self.is_list(st)
            || st.borrow().cursor != Cursor::Row
            || st.borrow().draft
            || st.borrow().stale
            || !st.borrow().window_from_start
            || st.borrow().window.is_empty()
            || (st.borrow().window.len() > Self::WINDOW && !st.borrow().window_whole && !st.borrow().window_read_up)
        {
            return Ok(None);
        }
        if st.borrow().window_read_up && find_page && st.borrow().host.is_none() {
            let rec = self.page_rec(st);
            let key = self.row_key(&rec)?;
            let at_last = st.borrow().window.last().is_some_and(|w| Self::same_key(&w.key, &key));
            // Last on the row the list opened on, the last of the view, reads nothing (misc/lt-probes-needed-6/
            // PnF06FindListMoves.al:304, :318 `desc3_last`, `desc25_last` empty).
            if how == PageMethod::Last && at_last {
                return Ok(Some(true));
            }
            if how == PageMethod::First && !st.borrow().window_whole {
                return self.first_above_the_window(st);
            }
            // First on the top row of a view the client holds whole reads nothing either (PnF06FindListMoves.al:362
            // PW_FindList_Descending_2Rows `desc2_first` empty, after Previous landed on the top row).
            let at_first = st.borrow().window.first().is_some_and(|w| Self::same_key(&w.key, &key));
            if how == PageMethod::First && at_first {
                return Ok(Some(true));
            }
            // Previous on a view of one row asks nothing (PnF06FindListMoves.al:348 PW_FindList_Descending_1Rows `desc1_prev`
            // empty); with more rows it is not pinned and refused.
            if how == PageMethod::Previous && at_first && st.borrow().window.len() == 1 {
                return Ok(Some(false));
            }
            if how == PageMethod::Previous && at_first {
                return unsupported(
                    "Previous on the top row of a descending list with OnFindRecord that holds more than one row (not pinned)",
                );
            }
        }
        if how == PageMethod::Last && st.borrow().window.len() == Self::WINDOW {
            return self.last_past_the_window(st, find_page);
        }
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        let window = st.borrow().window.clone();
        let Some(at) = window.iter().position(|w| Self::same_key(&w.key, &key)) else {
            return Ok(None);
        };
        // After a refresh that read the view to its end, Next on the last row answers false without asking the page (pages/lt-f3-pages/
        // F3ListTests.al:72 Nav_ActionChangesViewThenUpdate `after_walk` is empty); after an open or a GoToKey that read to the end,
        // see `PageState::view_read_to_end`.
        let read_to_end =
            (st.borrow().window_eof && !self.declares(st, "onaftergetcurrrecord") && window.len() < Self::WINDOW)
                || st.borrow().view_read_to_end;
        if how == PageMethod::Next && at + 1 == window.len() && read_to_end {
            // An editable, insert-allowed page walks on from its last row onto the blank new-row line (handlers/
            // TestPageNewRowLine_Tests.al:64-87), a part with OnFindRecord too (pagefindrecord/QcFindViewTests.al:505
            // DocPart_OpenEdit_LinesOfH1 `part_walk = 10000,20000,0`); the page is asked nothing (AiListOrderTests.al:600). Whether a
            // page with OnNextRecord is asked before the line is not pinned.
            if st.borrow().new_line {
                if self.declares(st, "onnextrecord") {
                    return unsupported(
                        "Next from the last row onto the new-row line of a page with OnNextRecord (whether the page is asked is not pinned)",
                    );
                }
                self.refuse_read_only_part(st, "Next() onto the new-row line")?;
                self.land_on_new_line(st)?;
                return Ok(Some(true));
            }
            return Ok(Some(false));
        }
        // First on the first row right after the refresh of CurrPage.Update settled the list there reads nothing: on a list with
        // OnFindRecord and OnNextRecord (pages/lt-ai-list-order/AiListOrderTests.al:174 Same_Nav_OnOpenRow, :203
        // Same_NavCurr_OnOpenRow `after_first` empty; pages/lt-f3-pages/F3ListTests.al:72 Nav_ActionChangesViewThenUpdate
        // `after_walk` empty), and on a list whose action's filter left its row (AiListOrderTests.al:339
        // DropCurrent_RowsAfter_Plain, :373 DropCurrent_RowsAfter_PlainCurr). Only a field read may come between (`settled_first_row`).
        // In any other state First on the first row of a list with OnFindRecord and OnNextRecord is not pinned; it keeps reading
        // the row as before, for baseline cases 86441 NextList_OpenAndWalk, NextList_NextLog and FreeList_OpenAndWalk (First after
        // the open).
        if how == PageMethod::First && at == 0 && std::mem::take(&mut st.borrow_mut().settled_first_row) {
            return Ok(Some(true));
        }
        let target = match how {
            PageMethod::First => 0,
            PageMethod::Last => window.len() - 1,
            PageMethod::Next if at + 1 < window.len() => at + 1,
            PageMethod::Previous if at > 0 => at - 1,
            _ => return Ok(None),
        };
        // Only a list that offers the blank line: a view-mode list goes straight to the last row (misc/lt-probes-needed-6/
        // PnF06FindListMoves.al:FindList_PlainDescending `plain3_last` = `R1;R1;C`).
        if how == PageMethod::Last && !find_page && st.borrow().new_line && st.borrow().host.is_none() {
            self.land_on_new_line(st)?;
            let mut s = st.borrow_mut();
            s.draft = false;
            s.cursor = Cursor::Row;
        }
        self.read_client_row(st, target)?;
        Ok(Some(true))
    }

    /// `Last` on a list whose window of 20 rows is full: the client reads the last row it holds again, then the rows after it in
    /// one block, and settles on the last of them (a page with OnFindRecord asks `=` for it). Pinned for 25 rows by
    /// misc/lt-probes-needed-6/PnF06FindListMoves.al:FindList_Ascending_25Rows `asc25_last`
    /// (`R20;N(1);R21;...;N(1);R25;N(1);F(=);R25;C`) and FindList_PlainDescending `plain25_last` (`R6;R5;R4;R3;R2;R1;R1;C`). A
    /// block of 20 new rows, a descending view of a page with OnFindRecord, and a list with the blank line are not pinned.
    fn last_past_the_window(&mut self, st: &St, find_page: bool) -> Result<Option<bool>, Signal> {
        let rec = self.page_rec(st);
        if st.borrow().new_line || st.borrow().window_eof || (find_page && !rec.borrow().ascending) {
            return Ok(None);
        }
        let before = self.xrec_before_move(st);
        self.read_block_past_the_window(st, find_page)?;
        // Without OnAfterGetCurrRecord the move ends with the block: `=` is not asked and the row is not read again
        // (pages/lt-ai-list-order/AiListOrderTests.al:958 Paging_Last_Nav `last_log` ends `R:R1025;N:1=0`).
        if !self.declares(st, "onaftergetcurrrecord") {
            return Ok(Some(true));
        }
        if find_page {
            self.nav_find(st, "=")?;
        }
        self.populate_current(st)?;
        self.current_record_trigger(st, before)?;
        Ok(Some(true))
    }

    /// The client reads the last row it holds again, then the rows after it in one block (up to 20, the failing last call of
    /// OnNextRecord included) and ends on the last of them with the window holding the whole view. Used by `Last` and by `New` on a
    /// list longer than a window (pages/lt-testpage-list-rows/ListRowsActionNew.al:NewRowThenAction_Ping30Rows_ReadsLogged: 12 reads
    /// after `New()` on 30 rows, the last row of the window, the ten rows after it, and the blank line). A second full block, and a view
    /// of exactly one window, are refused.
    pub(super) fn read_block_past_the_window(&mut self, st: &St, _find_page: bool) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let last = st.borrow().window[Self::WINDOW - 1].stored.clone();
        rec.borrow_mut().fields = last;
        self.populate_current(st)?;
        let mut new_rows = 0;
        loop {
            if !self.nav_next(st, 1)? {
                break;
            }
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            self.populate_current(st)?;
            let shown = rec.borrow().fields.clone();
            st.borrow_mut()
                .window
                .push(al_builtins::value::WindowRow { key, stored, shown });
            new_rows += 1;
            if new_rows == Self::WINDOW {
                return unsupported(
                    "a list with a second full block of rows past the window (the reads are not pinned)",
                );
            }
        }
        st.borrow_mut().window_eof = true;
        st.borrow_mut().window_whole = true;
        if new_rows == 0 {
            return unsupported("a list of exactly one window (the reads past it are not pinned)");
        }
        Ok(())
    }

    /// A move onto a row the client holds reads that row from the table again, so a change made elsewhere since the window
    /// was read shows (pages/W1Update.al:836-840 List_BufferLeftByOnAfterGetRecord_AfterTheStoredRowChanged `name_row2 =
    /// changed`; misc/lt-probes-needed-7/PnGRefresh.al:268-269 BufferEditOfOnAfterGetRecordOnAList). Pinned for a page that
    /// reads its rows from the table; a page with OnFindRecord or OnNextRecord keeps the row it holds (what the client asks
    /// such a page for is not pinned). A held row the table no longer has is refused (not pinned).
    pub(super) fn reread_held_row(&mut self, st: &St, target: usize) -> Result<(), Signal> {
        if self.declares(st, "onfindrecord") || self.declares(st, "onnextrecord") {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let mut key = st.borrow().window[target].key.clone();
        self.ret_used = true;
        if !matches!(self.record_op(RecordOp::Get, &rec, &mut key)?, Value::Boolean(true)) {
            let stored = st.borrow().window[target].stored.clone();
            rec.borrow_mut().fields = stored;
            return unsupported("a move onto a row the client holds that the table no longer has (not pinned)");
        }
        let fresh = rec.borrow().fields.clone();
        if let Some(w) = st.borrow_mut().window.get_mut(target) {
            w.stored = fresh;
        }
        Ok(())
    }

    /// The reads of a move onto row `target` of the rows the client holds (see `move_in_client_rows`).
    pub(super) fn read_client_row(&mut self, st: &St, target: usize) -> Result<(), Signal> {
        let before = self.xrec_before_move(st);
        let rec = self.page_rec(st);
        let stored = st.borrow().window[target].stored.clone();
        rec.borrow_mut().fields = stored;
        // A list with OnAfterGetRecord alone reads nothing when the cursor moves onto a row the client holds (pages/lt-testpage-list-rows/
        // ListRowsOpenAG.al:OpenView_OnlyRec5Rows_ReadsAtOpen `after-next-total = 0`).
        // The same holds with OnFindRecord and OnNextRecord declared: pages/lt-f3-pages/F3ListTests.al:24
        // Nav_TwelveRows_OpenLastPreviousFirstNext (`after_last`, `after_previous3`, `after_first`, `after_next2` all empty).
        // A list (or list part) with OnFindRecord, a filtered SourceTableView and no row trigger asks the page nothing either
        // (corpus branch al-runtime-rs-staging-qc eca98051, pagefindrecord/QcFindViewTests.al:97 ViewList_FirstNextLastPrevious,
        // :236 NextList_NextLog, :540 DocPart_OpenEdit_LinesOfH2: the logs are empty).
        if !self.declares(st, "onaftergetcurrrecord")
            && (self.declares(st, "onaftergetrecord") || self.filtered_find_list_without_row_triggers(st))
        {
            let shown = st.borrow().window[target].shown.clone();
            rec.borrow_mut().fields = shown;
            self.set_xrec(st);
            return Ok(());
        }
        self.reread_held_row(st, target)?;
        self.populate_current(st)?;
        if self.declares(st, "onfindrecord") {
            self.nav_find(st, "=")?;
        }
        self.populate_current(st)?;
        let shown = rec.borrow().fields.clone();
        if let Some(w) = st.borrow_mut().window.get_mut(target) {
            w.shown = shown;
        }
        self.current_record_trigger(st, before)
    }

    /// GoToKey / GoToRecord on a list that declares OnAfterGetCurrRecord and whose whole view the client holds (fewer rows than a
    /// window). The client does not search its rows: from a row other than the first it goes to the first row, then steps to the
    /// next rows one at a time until the key is there, each step a move (`r;r;c`, the row read twice and the current-record
    /// trigger); a key it does not find ends with a move back to the row it started from. Pinned by pages/W1Navigation.al:
    /// GoToKey_List_FiveRows_WhichRowsAreRead, GoToRecord_List_WhichRowsAreRead, GoToKey_List_FilteredOutKey and
    /// misc/lt-probes-needed-7/PnGGoToScan.al (GoToKeyFromTheFirstRow, GoToKeyFromALaterRow, GoToKeyMiss, GoToRecordFromTheFirstRow).
    /// `None` when the page is not of that kind. A key equal to the row the cursor stands on is found by the same scan
    /// (PnGGoToScan.al:GoToKeyFromALaterRow `hit-same`); a result that is not used and a key not found raise.
    /// GoToKey on a list that serves its rows from OnFindRecord and OnNextRecord and has no row trigger: a row the client holds is
    /// reached without asking the page; beyond the rows held, the next rows are read in one block with OnNextRecord (up to 20, the
    /// failing last call included). Pinned by pages/lt-testpage-list-rows/ListRowsOpenNone.al:OpenView_FindNext5Rows_ReadsAtOpen
    /// (`to-last-total = 0`) and OpenView_FindNext30Rows_ReadsAtOpen (`N21;...;N30;N30`). A key that is not there, and a block of 20 new
    /// rows, are not pinned. `None` when the page is not of that kind.
    ///
    /// A list with OnFindRecord only, a filtered SourceTableView and no row trigger reaches a row it holds the same way, and a key it
    /// does not hold, with the whole view held, answers false and stays without asking the page (corpus branch
    /// al-runtime-rs-staging-qc eca98051, pagefindrecord/QcFindViewTests.al:135 ViewList_GoToKey `gotokey_2 = No:1`, empty logs).
    fn go_to_key_find_without_row_triggers(&mut self, st: &St, want: &[Value]) -> Result<Option<bool>, Signal> {
        let find_only_view = !self.declares(st, "onnextrecord") && self.filtered_find_list_without_row_triggers(st);
        if !(self.declares(st, "onfindrecord") && self.declares(st, "onnextrecord") || find_only_view)
            || self.declares(st, "onaftergetrecord")
            || self.declares(st, "onaftergetcurrrecord")
            || !self.is_list(st)
            || st.borrow().host.is_some()
            || st.borrow().cursor != Cursor::Row
            || st.borrow().draft
            || st.borrow().stale
            || !st.borrow().window_from_start
            || st.borrow().window.is_empty()
            || st.borrow().window.len() > Self::WINDOW
        {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        let held = st.borrow().window.iter().position(|w| Self::same_key(&w.key, want));
        if let Some(i) = held {
            let shown = st.borrow().window[i].shown.clone();
            rec.borrow_mut().fields = shown;
            self.set_xrec(st);
            return Ok(Some(true));
        }
        if st.borrow().window.len() < Self::WINDOW || st.borrow().window_eof {
            if find_only_view {
                return Ok(Some(false));
            }
            return Ok(None);
        }
        if find_only_view {
            return unsupported(
                "GoToKey past the rows held by a list with OnFindRecord and a filtered SourceTableView (not pinned)",
            );
        }
        let last = st.borrow().window[Self::WINDOW - 1].stored.clone();
        rec.borrow_mut().fields = last;
        let mut new_rows = 0;
        while self.nav_next(st, 1)? {
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            st.borrow_mut().window.push(al_builtins::value::WindowRow {
                key,
                stored: stored.clone(),
                shown: stored,
            });
            new_rows += 1;
            if new_rows == Self::WINDOW {
                return unsupported("GoToKey past a second block of 20 rows (the reads are not pinned)");
            }
        }
        st.borrow_mut().window_eof = true;
        st.borrow_mut().window_whole = true;
        match st.borrow().window.iter().position(|w| Self::same_key(&w.key, want)) {
            Some(i) => {
                let shown = st.borrow().window[i].shown.clone();
                rec.borrow_mut().fields = shown;
                drop(rec);
                self.set_xrec(st);
                Ok(Some(true))
            }
            None => unsupported("GoToKey for a key the list does not hold (the reads are not pinned)"),
        }
    }

    fn go_to_key_in_client_rows(&mut self, st: &St, want: &[Value]) -> Result<Option<bool>, Signal> {
        if !self.declares(st, "onaftergetcurrrecord")
            || !self.declares(st, "onaftergetrecord")
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
            || !self.is_list(st)
            || st.borrow().host.is_some()
            || st.borrow().cursor != Cursor::Row
            || st.borrow().draft
            || st.borrow().stale
            || !st.borrow().window_from_start
            || st.borrow().window.is_empty()
        {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        let Some(origin) = st.borrow().window.iter().position(|w| Self::same_key(&w.key, &key)) else {
            return Ok(None);
        };
        let at_key = |s: &Self, i: usize| -> Option<bool> { s_window_key(s, st, i).map(|k| Self::same_key(&k, want)) };
        // A key equal to the first row the list stands on: the reads are not pinned, the generic move stays.
        if origin == 0 && at_key(self, 0) == Some(true) {
            return Ok(None);
        }
        let mut found = false;
        if origin != 0 {
            let r = self.arrive_held(st, 0);
            self.torn_down_on_error(st, r)?;
            found = at_key(self, 0) == Some(true);
        }
        let mut idx = 1;
        while !found && idx < st.borrow().window.len() {
            let r = self.arrive_held(st, idx);
            self.torn_down_on_error(st, r)?;
            found = at_key(self, idx) == Some(true);
            idx += 1;
        }
        if !found {
            let r = self.arrive_held(st, origin);
            self.torn_down_on_error(st, r)?;
        }
        Ok(Some(found))
    }

    /// A move onto row `idx` of the rows the client holds: when it is the last one held and the view goes on, the client first
    /// reads the next block of rows (the row itself, then 20 new ones); then the move itself (`r;r;c`). Pinned for views of up
    /// to 50 rows by pages/W1Navigation.al:GoToKey_List_FiftyRows_RowsReadByAFarJump (rows 20, then 21 to 40, then 20 again; row 40,
    /// then 41 to 50, then 40 again). A second block of 20 rows is not pinned: misc/lt-probes-needed-7/PnGGoToScan.al:
    /// GoToKeyOverTheWindowBoundary counts other reads for 60 rows, so that shape is refused.
    fn arrive_held(&mut self, st: &St, idx: usize) -> Result<(), Signal> {
        let held = st.borrow().window.len();
        if idx + 1 == held && held >= Self::WINDOW && !st.borrow().window_eof {
            // The row the move lands on is the last the client holds and the view may go on: the client reads it, then up to 20 rows
            // after it, one call each (a block of fewer than 20 ends the view, a full block leaves it open); the move itself follows.
            // Pinned by pages/lt-testpage-list-rows/ListRowsWindow.al:GoToKeyLastThenRow2_Both41Rows, _Both60Rows (the probe read
            // `G60` at the end of a full block) and _Both61Rows, and by pages/W1Navigation.al:GoToKey_List_FiftyRows_RowsReadByAFarJump.
            let rec = self.page_rec(st);
            let last = st.borrow().window[idx].stored.clone();
            rec.borrow_mut().fields = last;
            self.populate_current(st)?;
            let mut new_rows = 0;
            while new_rows < Self::WINDOW && self.nav_next(st, 1)? {
                let key = self.row_key(&rec)?;
                let stored = rec.borrow().fields.clone();
                self.populate_current(st)?;
                let shown = rec.borrow().fields.clone();
                st.borrow_mut()
                    .window
                    .push(al_builtins::value::WindowRow { key, stored, shown });
                new_rows += 1;
            }
            if new_rows < Self::WINDOW {
                let mut s = st.borrow_mut();
                s.window_eof = true;
                s.window_whole = true;
            }
        }
        self.read_client_row(st, idx)
    }

    /// GoToKey on a list that declares OnAfterGetCurrRecord and no OnAfterGetRecord: from a row other than the first the client goes to the
    /// first row, then steps on row by row until the key is there, each arrival one current-record read; standing on the first row it
    /// does not read that row again (pages/lt-testpage-list-rows/ListRowsWindow.al:GoToKeyLastThenRow2_OnlyCurr41Rows `C2;...;C41`, then
    /// back to row 2: `C1;C2`). A key equal to the row the cursor stands on (not the first) and a miss are not pinned.
    fn go_to_key_current_only(&mut self, st: &St, want: &[Value]) -> Result<Option<bool>, Signal> {
        if !self.declares(st, "onaftergetcurrrecord")
            || self.declares(st, "onaftergetrecord")
            || self.declares(st, "onfindrecord")
            || self.declares(st, "onnextrecord")
            || !self.is_list(st)
            || st.borrow().host.is_some()
            || st.borrow().cursor != Cursor::Row
            || st.borrow().draft
            || st.borrow().stale
            || !st.borrow().window_from_start
            || st.borrow().window.is_empty()
        {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        let cur = self.row_key(&rec)?;
        let on_first = st.borrow().window.first().is_some_and(|w| Self::same_key(&w.key, &cur));
        if Self::same_key(&cur, want) && !on_first {
            return Ok(None);
        }
        let origin_fields = rec.borrow().fields.clone();
        let mut found = false;
        let mut at = true;
        if !on_first {
            at = self.nav_find(st, "-")?;
            if at {
                st.borrow_mut().cursor = Cursor::Row;
                let r = self.arrive(st);
                self.torn_down_on_error(st, r)?;
                found = Self::same_key(&self.row_key(&rec)?, want);
            }
        }
        while at && !found {
            at = self.nav_next(st, 1)?;
            if !at {
                break;
            }
            let r = self.arrive(st);
            self.torn_down_on_error(st, r)?;
            found = Self::same_key(&self.row_key(&rec)?, want);
        }
        if !found {
            rec.borrow_mut().fields = origin_fields;
            return unsupported(
                "GoToKey for a key the list does not hold, on a list with OnAfterGetCurrRecord only (the reads are not pinned)",
            );
        }
        Ok(Some(true))
    }

    /// GoToKey for a key past a full window of a list with OnFindRecord, OnNextRecord and OnAfterGetRecord (no OnAfterGetCurrRecord)
    /// that holds its first 20 rows: the client reads the last two rows it holds again, without asking the page, then asks
    /// OnNextRecord(1) and reads each row after them to the end of the view; the cursor lands on the key without another read
    /// (pages/lt-ai-list-order/AiListOrderTests.al:999 Paging_GoToKey_Nav `R:R1019;R:R1020;N:1=1;R:R1021;...;R:R1025;N:1=0`, 25
    /// rows). The walk on from there asks nothing (:1020 Paging_GoToKeyThenWalk_Nav). A second full block and a key the view does
    /// not have are refused (not pinned). `None` when the page or its window is not of that kind.
    fn go_to_key_past_a_full_find_window(&mut self, st: &St, want: &[Value]) -> Result<Option<bool>, Signal> {
        let shape = {
            let s = st.borrow();
            s.host.is_none()
                && s.window_from_start
                && !s.window_eof
                && !s.window_whole
                && s.window.len() == Self::WINDOW
        };
        if !shape
            || !self.declares(st, "onnextrecord")
            || self.declares(st, "onaftergetcurrrecord")
            || self.in_window(st, want)
        {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        for i in Self::WINDOW - 2..Self::WINDOW {
            let stored = st.borrow().window[i].stored.clone();
            rec.borrow_mut().fields = stored;
            self.populate_current(st)?;
            let shown = rec.borrow().fields.clone();
            st.borrow_mut().window[i].shown = shown;
        }
        let mut new_rows = 0;
        while self.nav_next(st, 1)? {
            let key = self.row_key(&rec)?;
            let stored = rec.borrow().fields.clone();
            self.populate_current(st)?;
            let shown = rec.borrow().fields.clone();
            st.borrow_mut()
                .window
                .push(al_builtins::value::WindowRow { key, stored, shown });
            new_rows += 1;
            if new_rows == Self::WINDOW {
                return unsupported(
                    "GoToKey past a second full block of rows of a find list (the reads are not pinned)",
                );
            }
        }
        {
            let mut s = st.borrow_mut();
            s.window_eof = true;
            s.window_whole = true;
            s.view_read_to_end = true;
        }
        let at = st.borrow().window.iter().position(|w| Self::same_key(&w.key, want));
        let Some(at) = at else {
            return unsupported(
                "GoToKey for a key a find list does not hold, past a full window (the reads are not pinned)",
            );
        };
        let shown = st.borrow().window[at].shown.clone();
        rec.borrow_mut().fields = shown;
        self.set_xrec(st);
        Ok(Some(true))
    }

    pub(super) fn page_move_inner(&mut self, st: &St, how: PageMethod) -> Result<bool, Signal> {
        self.refuse_descending_filtered_find_move(st)?;
        if let Some(r) = self.move_in_client_rows(st, how)? {
            return Ok(r);
        }
        let cursor = st.borrow().cursor;
        let landed = match how {
            PageMethod::First => self.nav_find(st, "-")?,
            PageMethod::Last => {
                let found = self.nav_find(st, "+")?;
                // The blank line is the last line the client shows: it is reached first, then the client settles on the
                // last row (misc/lt-probes-needed-3/PnC07TestPage.al:New_AtTheEnd and New_AfterMovingAwayFromNewRow, OnNewRecord
                // twice with the New(); New_InTheMiddle after First() has one).
                if found
                    && cursor == Cursor::Row
                    && st.borrow().new_line
                    && st.borrow().host.is_none()
                    && self.is_list(st)
                    && !self.declares(st, "onfindrecord")
                    && !self.declares(st, "onnextrecord")
                {
                    self.land_on_new_line(st)?;
                    self.nav_find(st, "+")?;
                    let mut s = st.borrow_mut();
                    s.draft = false;
                    s.cursor = Cursor::Row;
                }
                found
            }
            PageMethod::Next => match cursor {
                Cursor::Row => {
                    if self.nav_next(st, 1)? {
                        true
                    } else if st.borrow().new_line {
                        // corpus handlers/TestPageNewRowLine_Tests.al: Next() past the last data
                        // row lands on the blank line and answers true.
                        self.refuse_read_only_part(st, "Next() onto the new-row line")?;
                        self.land_on_new_line(st)?;
                        return Ok(true);
                    } else {
                        false
                    }
                }
                _ => false,
            },
            PageMethod::Previous => match cursor {
                Cursor::NewLine => self.nav_find(st, "+")?,
                Cursor::Row => self.nav_next(st, -1)?,
                Cursor::None => false,
            },
            _ => unreachable!(),
        };
        if landed {
            st.borrow_mut().cursor = Cursor::Row;
            self.arrive(st)?;
        }
        Ok(landed)
    }

    /// GoToKey / GoToRecord: a forward scan from the first row of the view, populating every row it
    /// passes, so a row whose population fails stops the search there (corpus
    /// autoformat/TestAutoFormatExpressionTiming.al, RowFormat_GoToKeyPastTheFailingRow_ReadsAsTheTestPageNotOpen).
    /// A key no row has leaves the page on the row it stood on
    /// (corpus handlers/TestPageGoToRecordNonKeyRefresh_Tests.al:1-15).
    pub(super) fn page_go_to_key(&mut self, st: &St, key: &mut [Value]) -> Result<Value, Signal> {
        if !self.has_source(st) {
            return unsupported("GoToKey on a page without a source table");
        }
        let result_used = self.ret_used;
        // A TestPart raises when the key has the wrong number of fields, and when the key is
        // missing and the result is not used (corpus testpart/TestTestPart.al:
        // TestPart_GoToKey_ErrorsWhenTheKeyFieldCountIsWrong, _ErrorsForAMissingKeyWhenTheReturnIsDiscarded).
        // Texts and codes: handlers/TestPageRuntimeContracts.al:300-312 (TestPart_GoToKey_Arity_AndMissingKey).
        // A page too raises for more values than the key has fields (pages/lt-page-runtime/P82TestPage.al:Navigation_GoToKey
        // `too-many-key-values`); fewer is pinned for a part only.
        {
            let rec = self.page_rec(st);
            let wanted = self.table(&rec)?.keys[0].len();
            if key.len() != wanted && (st.borrow().host.is_some() || key.len() > wanted) {
                return Err(error_code(
                    "The number of arguments does not match the number of primary key fields.",
                    "TestInvalidNumberOfKeyFieldValues",
                ));
            }
        }
        self.save_row(st)?;
        // The cursor leaves the new row: the row it was started below no longer decides where Next goes.
        st.borrow_mut().after_new = None;
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        // A lowercase text is no key of a Code field: the client compares as typed (pages/lt-page-runtime/P82TestPage.al:
        // Navigation_GoToKey `lowercase` = No for 'n2' against N2). A row that is not found, as for any other missing key.
        let lowercase_code = table.keys[0].iter().enumerate().any(|(n, &f)| {
            matches!(&table.fields[f].ty, RtType::Code(_))
                && matches!(key.get(n), Some(Value::Text(t)) if *t != t.to_uppercase())
        });
        if lowercase_code {
            if !result_used {
                return Err(error_code("The row does not exist on the TestPage.", "TestRowNotFound"));
            }
            return Ok(Value::Boolean(false));
        }
        let want: Result<Vec<Value>, Signal> = table.keys[0]
            .iter()
            .enumerate()
            .map(|(n, &f)| match key.get(n) {
                Some(v) => ops::convert(v.clone(), &table.fields[f].ty),
                None => Ok(self.default_value(&table.fields[f].ty)),
            })
            .collect();
        // A value that is no value of the key's type is no key of any row (pages/lt-page-runtime/P82TestPage.al:Navigation_GoToKey
        // `wrong-type`: GoToKey(1) over a Code key raises the row error).
        let want = match want {
            Ok(w) => w,
            Err(Signal::Error(_)) => {
                if !result_used {
                    return Err(error_code("The row does not exist on the TestPage.", "TestRowNotFound"));
                }
                return Ok(Value::Boolean(false));
            }
            Err(e) => return Err(e),
        };
        // GoToKey onto the first row while standing on it reads nothing and runs no trigger (pages/lt-testpage-list-rows/
        // ListRowsGoToFirst.al:GoToKeyFirstWhileOnIt_Both5Rows_ReadsLogged and _OnlyCurr30Rows_ReadsLogged: `first-on-first-total = 0`,
        // also the second time).
        if self.declares(st, "onaftergetcurrrecord")
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord")
            && self.is_list(st)
            && st.borrow().host.is_none()
            && st.borrow().cursor == Cursor::Row
            && !st.borrow().draft
            && !st.borrow().stale
            && st.borrow().window_from_start
        {
            let cur = self.row_key(&rec)?;
            let first_is_cur = st.borrow().window.first().is_some_and(|w| Self::same_key(&w.key, &cur));
            if first_is_cur && Self::same_key(&cur, &want) {
                return Ok(Value::Boolean(true));
            }
        }
        self.refuse_descending_filtered_find_move(st)?;
        if let Some(found) = self.go_to_key_find_without_row_triggers(st, &want)? {
            if !found && !result_used {
                return Err(error_code("The row does not exist on the TestPage.", "TestRowNotFound"));
            }
            return Ok(Value::Boolean(found));
        }
        if let Some(found) = self.go_to_key_current_only(st, &want)? {
            return Ok(Value::Boolean(found));
        }
        if let Some(found) = self.go_to_key_in_client_rows(st, &want)? {
            if !found && !result_used {
                return Err(error_code("The row does not exist on the TestPage.", "TestRowNotFound"));
            }
            return Ok(Value::Boolean(found));
        }
        // A list that serves its rows from OnFindRecord and holds its whole view goes to a row it
        // holds as a move does (corpus pageext/PCCurrRecordReads.al:FindList_OnFindRecordAndOnNextRecord
        // `gotokey_2`, a table of 3 rows).
        if self.declares(st, "onfindrecord")
            && self.declares(st, "onaftergetrecord")
            && self.is_list(st)
            && st.borrow().cursor == Cursor::Row
            && !st.borrow().draft
            && !st.borrow().stale
        {
            let held = {
                let s = st.borrow();
                (s.window_from_start && s.window.len() < Self::WINDOW)
                    .then(|| s.window.iter().position(|w| Self::same_key(&w.key, &want)))
                    .flatten()
            };
            if let Some(at) = held {
                // With OnAfterGetCurrRecord the client steps from the first row to the key one move at a time (pages/lt-ai-list-
                // order/AiListOrderTests.al:769 GoToRecord_BeforeAnyNavigation_NavCurr `R:R1002;F:==Yes;R:R1002;C:R1002;R:R1003;
                // ...;C:R1004`). Pinned from the first row onto a later one; from another row not. Onto the first row while
                // standing on it the row is read as by a move (not pinned, kept as before: pageupdate/TestPageUpdateGoneRow.al:492
                // List_DeletedByAction_PassThroughFind_OnlyRow does not observe those reads).
                if self.declares(st, "onaftergetcurrrecord") && at > 0 {
                    let cur = self.row_key(&rec)?;
                    let on_first = st.borrow().window.first().is_some_and(|w| Self::same_key(&w.key, &cur));
                    if !on_first {
                        return unsupported(
                            "GoToKey on a find list with OnAfterGetCurrRecord from a row other than the first (the reads are not pinned)",
                        );
                    }
                    for i in 1..=at {
                        self.read_client_row(st, i)?;
                    }
                    return Ok(Value::Boolean(true));
                }
                self.read_client_row(st, at)?;
                return Ok(Value::Boolean(true));
            }
            if let Some(found) = self.go_to_key_past_a_full_find_window(st, &want)? {
                return Ok(Value::Boolean(found));
            }
            // Other shapes with a full window of rows: what the client asks the page for is not pinned.
            if st.borrow().window.len() >= Self::WINDOW {
                return unsupported(
                    "GoToKey on a list that serves its rows from OnFindRecord, with a full window of rows (the rows the client asks for are not implemented)",
                );
            }
        }
        let before_fields = rec.borrow().fields.clone();
        let (before_cursor, before_draft) = (st.borrow().cursor, st.borrow().draft);
        let mut at = self.nav_find(st, "-")?;
        let mut found = false;
        // The last block this scan read: whether it was full and the key of its last row.
        let mut block: Option<(bool, Vec<Value>)> = None;
        // A Document host walks its rows from the first one, and its parts read their rows for each host row it passes whose
        // lines they do not already show (pages/lt-qj-part-propagation/QjPartPropagation.al:1183 QL_Both_GoToKey_Counts
        // `log-gotokey-h2-again` = H1 with the part's reads, then H2 with them; :1210 QL_Default_GoToKey_Counts). Pinned for a
        // Document page; another host keeps the earlier reads.
        let doc_host = {
            let object = st.borrow().object as usize;
            self.engine.object_facts[object].page_type == Some(al_ir::PageType::Document)
        } && !self.is_list(st)
            && st.borrow().host.is_none()
            && !st.borrow().parts.is_empty();
        let mut parts_show =
            (doc_host && before_cursor == Cursor::Row && !before_draft).then(|| table.primary_key(&before_fields));
        st.borrow_mut().walk_synced_parts = false;
        while at {
            st.borrow_mut().cursor = Cursor::Row;
            if self.is_list(st) && st.borrow().window_from_start && !st.borrow().window.is_empty() {
                let key = self.row_key(&rec)?;
                if !self.in_window(st, &key) {
                    let r = self.read_window_block(st);
                    block = Some(self.torn_down_on_error(st, r)?);
                }
            }
            let r = self.arrive(st);
            self.torn_down_on_error(st, r)?;
            let here = self.row_key(&rec)?;
            if here.len() == want.len()
                && here
                    .iter()
                    .zip(&want)
                    .all(|(a, b)| ops::compare(a, b).map(|o| o.is_eq()).unwrap_or(false))
            {
                found = true;
                break;
            }
            if let Some(shown) = &parts_show {
                if !Self::same_key(shown, &here) {
                    self.sync_parts(st)?;
                    parts_show = Some(here.clone());
                    st.borrow_mut().walk_synced_parts = true;
                }
            }
            at = self.nav_next(st, 1)?;
        }
        if found {
            // A full block whose last row is the target: the read goes on. At the end of the view it reads
            // the last two rows once more (handlers/TestPageRuntimeContracts.al:331, row 60). Where more
            // rows follow, nothing pins what is read.
            if let Some((true, last)) = &block {
                if Self::same_key(&self.row_key(&rec)?, last) {
                    if self.nav_next(st, 1)? {
                        self.nav_next(st, -1)?;
                        return unsupported(
                            "GoToKey onto the last row of a full block of rows read, with more rows after it (not pinned)",
                        );
                    }
                    let r = self.reread_last_rows(st);
                    self.torn_down_on_error(st, r)?;
                }
            }
        }
        if !found {
            // A miss reads on to the end of the view and then the last two rows once more
            // (handlers/TestPageSystemPartListWindowContracts.al:194, key 100).
            if block.is_some() && !st.borrow().window_eof {
                let r = self.reread_last_rows(st);
                self.torn_down_on_error(st, r)?;
            }
            // A card goes back to the row it started from by reading the rows from the first one again (pages/W1Navigation.al:
            // GoToKey_Card_WhichRowsAreRead `goto_99_miss`).
            if !self.is_list(st) && st.borrow().host.is_none() && before_cursor == Cursor::Row && !before_draft {
                let origin = table.primary_key(&before_fields);
                let mut at = self.nav_find(st, "-")?;
                while at {
                    st.borrow_mut().cursor = Cursor::Row;
                    let r = self.arrive(st);
                    self.torn_down_on_error(st, r)?;
                    if Self::same_key(&self.row_key(&rec)?, &origin) {
                        break;
                    }
                    at = self.nav_next(st, 1)?;
                }
            }
            rec.borrow_mut().fields = before_fields;
            let mut s = st.borrow_mut();
            s.cursor = before_cursor;
            s.draft = before_draft;
            drop(s);
            if st.borrow().host.is_some() && !result_used {
                return Err(error_code("The row does not exist on the TestPage.", "TestRowNotFound"));
            }
        }
        Ok(Value::Boolean(found))
    }

    /// `FindFirstField` / `FindNextField` / `FindPreviousField`: walks the rows of the view from the
    /// first row, the row after the current one, or the row before it, and stops on the first row
    /// whose control shows `value`. The Variant is type-sensitive: a Text never matches an Integer
    /// field (corpus handlers/TestPageAdvanced.al:302 Page_FindPreviousField_StringValue_*). A miss
    /// leaves the cursor where it was (docs/probes-needed.md).
    pub(super) fn page_find_field(&mut self, st: &St, how: PageMethod, args: &mut [Value]) -> Result<Value, Signal> {
        if !self.has_source(st) {
            return unsupported(format!("{how} on a page without a source table"));
        }
        let Some(Value::TestControl(c)) = args.first() else {
            return unsupported(format!("{how} without a control"));
        };
        let ControlTarget::Control(idx) = c.target else {
            return unsupported(format!("{how} on a built-in control"));
        };
        let wanted = args.get(1).cloned().unwrap_or(Value::Null);
        let result_used = self.ret_used;
        let origin = self.row_key(&self.page_rec(st))?;
        let (first_step, step) = match how {
            PageMethod::FindFirstField => (PageMethod::First, PageMethod::Next),
            PageMethod::FindNextField => (PageMethod::Next, PageMethod::Next),
            _ => (PageMethod::Previous, PageMethod::Previous),
        };
        let mut at = self.page_move(st, first_step)?;
        // A search that had no row to look at (FindPreviousField on the first row) raises nothing (misc/lt-probes-needed-3/
        // PnC07TestPage.al:FindField_MissWithUnusedResult `previous`).
        let searched = matches!(at, Value::Boolean(true));
        while matches!(at, Value::Boolean(true)) {
            let here = self.control_value(st, idx)?;
            if Self::variant_matches(&here, &wanted) {
                return Ok(Value::Boolean(true));
            }
            at = self.page_move(st, step)?;
        }
        let mut key = origin;
        self.page_go_to_key(st, &mut key)?;
        // A Find...Field whose result is not used raises when no row carries the value
        // (handlers/TestPagePlatformPageSourceTable_Tests.al:125-131, the whole message; controls/W2FindFieldAssign.al:
        // FindNextField_MissWithResultUnused and FindPreviousField_MissWithResultUnused, BC 27.5 and 29.0).
        if !result_used && searched {
            return Err(error_code("The row does not exist on the TestPage.", "TestRowNotFound"));
        }
        Ok(Value::Boolean(false))
    }

    pub(super) fn variant_matches(here: &Value, wanted: &Value) -> bool {
        let class = |v: &Value| match v {
            Value::Text(_) | Value::Code(_) => 1,
            Value::Integer(_) => 2,
            Value::BigInteger(_) => 3,
            Value::Decimal(_) => 4,
            Value::Boolean(_) => 5,
            Value::Date(..) => 6,
            _ => 0,
        };
        class(here) == class(wanted)
            && class(here) != 0
            && ops::compare(here, wanted).map(|o| o.is_eq()).unwrap_or(false)
    }

    /// A `Trap()` set on a TestPage that belongs to a routine ends with the routine; one set through
    /// a `var` parameter belongs to the caller (corpus handlers/TestPageTrapScope_Tests.al:1-15).
    pub(crate) fn end_trap_scope(&mut self, frame: &crate::Frame, params: usize, by_var: &[bool]) {
        // Every call ends here; without a trap there is nothing to look at.
        if self.pages.traps.is_empty() {
            return;
        }
        for trap in self.pages.traps.clone() {
            // The trap ends with the last variable holding the page. A by-var parameter hands its page
            // back to the caller, and a page another variable still holds keeps the trap alive (corpus
            // handlers/TestPageTrapSharedScope_Tests.al:87 TrapSharedByTwoLocalsSurvivesWhenOneIsAssignedOut,
            // handlers/TestPageTrapScope_Tests.al TrapAssignedOutOfALocalVariableSurvives).
            let mut local_holders = 0;
            let mut handed_back = false;
            for (i, v) in frame.locals.iter().enumerate() {
                if let Value::TestPage(st) = v {
                    if Rc::ptr_eq(st, &trap) {
                        local_holders += 1;
                        if i < params && by_var.get(i).copied().unwrap_or(false) {
                            handed_back = true;
                        }
                    }
                }
            }
            if local_holders == 0 || handed_back {
                continue;
            }
            let outside = Rc::strong_count(&trap).saturating_sub(1 + local_holders + 1);
            if outside == 0 {
                trap.borrow_mut().trapping = false;
                self.pages.traps.retain(|t| !Rc::ptr_eq(t, &trap));
            }
        }
    }

    /// A routine ends while one of its locals holds the last reference to an open TestPage whose end of scope is not pinned
    /// (see `PageClients::scope_unpinned`): the reason to refuse the call.
    pub(crate) fn end_unpinned_page_scope(
        &mut self,
        frame: &crate::Frame,
        params: usize,
        by_var: &[bool],
    ) -> Option<String> {
        if self.pages.scope_unpinned.is_empty() {
            return None;
        }
        self.pages.scope_unpinned.retain(|p| p.borrow().open);
        for page in self.pages.scope_unpinned.clone() {
            let mut local_holders = 0;
            let mut handed_back = false;
            for (i, v) in frame.locals.iter().enumerate() {
                if let Value::TestPage(st) = v {
                    if Rc::ptr_eq(st, &page) {
                        local_holders += 1;
                        if i < params && by_var.get(i).copied().unwrap_or(false) {
                            handed_back = true;
                        }
                    }
                }
            }
            if local_holders == 0 || handed_back {
                continue;
            }
            // The list and the clone above hold one reference each, and the page's own instances hold it as CurrPage.
            let own = {
                let s = page.borrow();
                let held = |i: &Rc<RefCell<al_builtins::value::Instance>>| {
                    i.borrow()
                        .globals
                        .iter()
                        .filter(|v| matches!(v, Value::TestPage(p) if Rc::ptr_eq(p, &page)))
                        .count()
                };
                s.instance.iter().map(held).sum::<usize>() + s.ext_instances.iter().map(|(_, i)| held(i)).sum::<usize>()
            };
            if Rc::strong_count(&page).saturating_sub(2 + local_holders + own) == 0 {
                self.pages.scope_unpinned.retain(|p| !Rc::ptr_eq(p, &page));
                let why = page.borrow().unpinned_state.clone().unwrap_or_default();
                return Some(format!("an open TestPage goes out of scope: {why}"));
            }
        }
        None
    }

    /// The `Trap()` waiting for page `object`, taken out of the list.
    pub(crate) fn take_trap(&mut self, object: u32) -> Option<Rc<RefCell<al_builtins::value::PageState>>> {
        let at = self.pages.traps.iter().position(|t| t.borrow().object == object)?;
        Some(self.pages.traps.remove(at))
    }

    // ---- pages opened by AL code ----------------------------------------------------------

    /// The built-in action name `close_with` is asked for, as the `Action` the page reports. A
    /// lookup-mode modal reads OK and Cancel as LookupOK and LookupCancel; a page closing itself
    /// (`CurrPage.Close()`) reads as OK in a plain modal and as LookupCancel in lookup mode (corpus
    /// handlers/TestPageModalQueryClose_Tests.al, TestPageModalSelfClose_Tests.al).
    pub(super) fn close_action(&self, st: &St, requested: &str) -> String {
        let (modal, lookup, object) = {
            let s = st.borrow();
            (s.modal, s.lookup, s.object as usize)
        };
        // A confirmation dialog stays Cancel even where the lookup form reads Cancel as LookupCancel (misc/lt-probes-needed-7/
        // PnGActionOrdinals.al:ModalResultsOfAConfirmationDialog `nothing-invoked`).
        if requested == "Cancel" && self.engine.object_facts[object].page_type_lower() == "confirmationdialog" {
            return "Cancel".into();
        }
        match (requested, modal, lookup) {
            ("OK", true, true) => "LookupOK".into(),
            ("Cancel", true, true) => "LookupCancel".into(),
            ("Close", true, true) => "LookupCancel".into(),
            ("Close", true, false) => "OK".into(),
            ("Close", false, _) => "OK".into(),
            (other, _, _) => other.into(),
        }
    }

    /// What a modal page reports when its handler invoked nothing: the page type decides (corpus
    /// handlers/TestPageBuiltInActions_Tests.al (i)-(k), TestPageModalQueryClose_Tests.al (b) and (e)).
    pub(super) fn unattended_action(&self, st: &St) -> &'static str {
        let (lookup, object) = {
            let s = st.borrow();
            (s.lookup, s.object as usize)
        };
        let page_type = self.engine.object_facts[object].page_type_lower().to_string();
        match page_type.as_str() {
            "standarddialog" | "promptdialog" | "confirmationdialog" => "Cancel",
            _ if lookup => "LookupCancel",
            _ => "OK",
        }
    }

    /// `Page.Run` / `Page.RunModal` (static and on a `Page` variable): opens the page for the
    /// calling AL, hands it to the bound handler (or to the `TestPage.Trap()` variable for a plain
    /// run), and for a modal run reports how it closed.
    pub(crate) fn run_page_ui(&mut self, st: &St, record: Option<Rec>, modal: bool) -> Result<Value, Signal> {
        self.run_page_ui_with(st, record, modal, false)
    }

    /// `Page.RunModal(Id, Rec)` leaves the row the lookup was closed on in `Rec` (Gen. Journal Management's template
    /// selection reads it). Pinned by pages/lt-page-runmodal-rec/PageRunModalRecAfterClose.al:P174_NonTemp_StartA_HandlerGoToKeyCThenOk and siblings.
    fn hand_back_selection(&mut self, st: &St, handed: &Option<Rec>, answer: &Value) {
        let statement = st.borrow().statement_run;
        let closed_on_row = matches!(answer, Value::Option(n, m) if m.get(*n as usize).is_some_and(|a| a == "LookupOK" || a == "LookupCancel"
            || (statement && (a == "OK" || a == "Cancel"))));
        let picked = st.borrow_mut().selection.take();
        if let (true, Some(r), Some(fields)) = (closed_on_row, handed, picked) {
            if r.borrow().fields.len() == fields.len() {
                // Called as a statement, whether Rec takes the row the page closed on is not pinned: when that row is not the
                // one Rec holds, any later use of Rec refuses (docs/probes-needed.md, lane-lookups-30).
                if statement {
                    if !Self::same_values(&r.borrow().fields, &fields) {
                        r.borrow_mut().unpinned_buffer = Some(
                            "Rec after Page.RunModal(Id, Rec) called as a statement, closed on another row (whether Rec takes that row is not pinned)",
                        );
                    }
                    return;
                }
                r.borrow_mut().fields = fields;
            }
        }
    }

    /// [`run_page_ui`](Self::run_page_ui); `field`: the call had a field argument (`Page.RunModal(Id, Rec, Rec.Field)`).
    /// With no handler for the page the field changes nothing: the unhandled-UI error is the same as without it
    /// (corpus errors/lt-last-error-and-events/P70_06RunModalField.al). What a handler sees and what the field
    /// takes when it answers OK is not pinned, so that is refused.
    pub(crate) fn run_page_ui_with(
        &mut self,
        st: &St,
        record: Option<Rec>,
        modal: bool,
        field: bool,
    ) -> Result<Value, Signal> {
        let handed = record.clone();
        let object = st.borrow().object;
        let mut st = st.clone();
        let mut trapped = false;
        if !modal {
            if let Some(trap) = self.take_trap(object) {
                trap.borrow_mut().trapping = false;
                // A variable that still holds an open page closes it first, as Close() does: OnQueryClosePage(OK), then OnClosePage
                // with the old record (pages/lt-f3-pages/F3TrappedAndModal.al:Trap_02_SameVariableTrappedAgain line 36).
                if trap.borrow().open {
                    let a = self.close_action(&trap, "Close");
                    self.close_with(&trap, &a, false, false)?;
                }
                // The trapped variable receives the page and keeps it open for the test. A `Page`
                // variable that was set up first hands its instance over
                // (handlers/TestPageOnInit_Tests.al:223 POI_SetterBeforeTrappedRun_...).
                self.adopt_instance(&st, &trap);
                st = trap;
                trapped = true;
            } else if !self.pages.traps.is_empty() {
                return unsupported("a Trap() for another page than the one Page.Run opens");
            }
        }
        if st.borrow().open {
            if !trapped {
                return unsupported("running a page that is already open");
            }
            self.mark_closed(&st);
        }
        if modal {
            if let Some(handler) = self.take_typed_modal_handler(object) {
                let answer = self.answer_typed_modal(&st, record, handler)?;
                return self.field_argument_answer(field, answer);
            }
        }
        // A second non-modal Run of a Page variable runs its OnInit again (pages/lt-page-variable-lifecycle/
        // PageVariableLifecycle.al:131 PageVar_RunTwiceOnOneVariable `IOIO`); a second RunModal does not
        // (controls/W2PageVariableInit.al:102-104 PageVariable_RunModalTwice). A run of the other kind than the one before, and a
        // second non-modal run of a page with variables of its own (whether they keep their values is not pinned), are refused.
        let last = st.borrow_mut().last_run_modal.replace(modal);
        if let (false, Some(last)) = (trapped, last) {
            if last != modal {
                return unsupported(
                    "a Page variable run modal after a non-modal run, or the other way round (whether OnInit runs again is not pinned)",
                );
            }
            if !modal {
                let own_globals = self.engine.program.objects[object as usize].globals.iter().any(|g| {
                    !["Rec", "xRec", "CurrPage"]
                        .iter()
                        .any(|n| g.name.eq_ignore_ascii_case(n))
                });
                if own_globals {
                    return unsupported(
                        "a second non-modal Run of a Page variable whose page has variables of its own (whether they keep their values is not pinned)",
                    );
                }
                st.borrow_mut().inited = false;
            }
        }
        let kind = if modal { UiKind::ModalPage } else { UiKind::Page };
        st.borrow_mut().modal = modal && !trapped;
        let handler = self.open_page(
            &st,
            PageMode::Edit,
            record.clone(),
            if trapped { None } else { Some(kind) },
        )?;
        let Some(handler) = handler else {
            return Ok(Value::Null);
        };
        let answer = self.answer_page(&st, handler, modal)?;
        let answer = self.field_argument_answer(field, answer)?;
        self.hand_back_selection(&st, &handed, &answer);
        // After a LookupOK the page variable holds the row the page closed on, and the next run of the variable opens on it;
        // after OK it does not (pages/lt-page-run-overloads-testpart/P106RunInstance.al:P106_Inst_RunModal_LookupMode,
        // P106_Inst_RunModal_List).
        if modal && matches!(&answer, Value::Option(o, m) if m.get(*o as usize).is_some_and(|n| n == "LookupOK")) {
            let instance = st.borrow().instance.clone();
            let slot = self.engine.program.objects[object as usize].rec_slot;
            if let (Some(inst), Some(slot)) = (instance, slot) {
                let global = inst.borrow().globals[slot].clone();
                if let Value::Record(src) = global {
                    let mut copy = src.borrow().clone();
                    copy.filters.clear();
                    st.borrow_mut().set_record = Some(Rc::new(RefCell::new(copy)));
                }
            }
        }
        Ok(answer)
    }

    /// What `Page.RunModal(Id, Rec, Field)` does with the field when a handler answered. Only an answer other than LookupOK is
    /// accepted, and then the field keeps its value (documented: the field takes the page's value on LookupOK; the corpus pins no
    /// handler with a field argument, P70_06RunModalField.al has none bound). Microsoft's tests need it: Segment Line.RunCreateInteraction
    /// runs `PAGE.RunModal(PAGE::"Create Interaction", Rec, "Interaction Template Code")` and Tests-Marketing answer it with a
    /// Page-typed handler that leaves Response alone (33 tests). A LookupOK answer stays refused.
    fn field_argument_answer(&self, field: bool, answer: Value) -> Result<Value, Signal> {
        if !field {
            return Ok(answer);
        }
        let lookup_ok = matches!(&answer, Value::Option(o, m) if m.get(*o as usize).is_some_and(|n| n == "LookupOK"));
        if lookup_ok {
            return unsupported(FIELD_ARGUMENT_WITH_HANDLER);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::RunModalFieldKeptUnlessLookupOk);
        Ok(answer)
    }

    /// A `[ModalPageHandler]` typed as the Page itself (handlers/PSModalPageHandlerTyped.al): the handler gets the page
    /// object the code ran, never opened. `GetRecord` gives the record `RunModal` was given (an empty one when none),
    /// `LookupMode` is Yes, `SetRecord` and `GetRecord` work on it, `Response` arrives as None, and the value the
    /// handler leaves in it is what `Page.RunModal` answers. Whether the page's own triggers run is not pinned, so a
    /// page that declares one is refused.
    pub(super) fn answer_typed_modal(
        &mut self,
        st: &St,
        record: Option<Rec>,
        handler: Handler,
    ) -> Result<Value, Signal> {
        // OnInit runs when the page object is made, before the handler starts; no other trigger of the page runs for the call
        // (errors/lt-ms-refusals-136/P136PageTyped.al:P136_ModalHandler_TypedPage_OnInitList_LookupOK, _OnInitCard, and the
        // list and card pages there that declare OnOpenPage, OnAfterGetRecord, OnQueryClosePage and OnClosePage).
        self.construct_page(st)?;
        if self.has_source(st) {
            if let Some(r) = &record {
                let rec = self.page_rec(st);
                // The record of a SetTableView alone is a view: it carries no position, so it does not replace the Rec the
                // page's own code set (Contact Profile Answers.SetRunFromForm does `Rec := ProfileQuestionnaireLine` after the test's
                // SetTableView; its handler then reads Rec."Line No."). Pinned for a row inside and outside the view:
                // pages/lt-modal-handler-typed-page/PageTypedHandlerRec.al:SettableView_ThenAssignRecInsideView, :SettableView_ThenAssignRecOutsideView.
                let table = r.borrow().table as usize;
                let blank = self.initial_fields(table);
                let view_only = st.borrow().set_view && Value::all_identical(&r.borrow().fields, &blank);
                if !view_only {
                    rec.borrow_mut().fields = r.borrow().fields.clone();
                }
            }
        }
        {
            // The handler reads the lookup mode the run gave the page: Yes for the static Page.RunModal, which sets it, and the
            // variable's own for a Page variable, No unless LookupMode(true) was called (errors/lt-ms-refusals-128/
            // P128ModalOnInit.al:191 `lookup=Yes`, :210 P128_TypedModalHandler_PageWithOnInit_PageVariable `lookup=No`).
            let mut s = st.borrow_mut();
            s.typed_handler = true;
            s.cursor = Cursor::Row;
        }
        let (hobject, hroutine, hinst) = handler;
        let none = al_builtins::value::ACTION_MEMBERS
            .iter()
            .position(|m| *m == "None")
            .unwrap_or(0) as i32;
        let members: std::sync::Arc<[String]> = al_builtins::value::ACTION_MEMBERS
            .iter()
            .map(|m| m.to_string())
            .collect();
        self.ui_state.page_handler_depth += 1;
        let handled = self.call(
            hobject,
            hroutine,
            hinst,
            vec![Value::TestPage(st.clone()), Value::Option(none, members)],
        );
        self.ui_state.page_handler_depth -= 1;
        {
            let mut s = st.borrow_mut();
            s.typed_handler = false;
            s.cursor = Cursor::None;
        }
        let (_, finals) = handled?;
        match finals.get(1) {
            Some(v @ Value::Option(..)) => {
                // LookupOK hands the page's record back to the record RunModal was given, as the TestPage handlers do (hand_back_selection).
                // Pinned for a Page-typed handler that calls SetRecord (Saved Segment Criteria List: ReuseCriteria reads the record it passed):
                // pages/lt-modal-handler-typed-page/PageTypedHandlerRec.al:RunModalWithRecord_HandlerSetRecord_LookupOK, with Cancel and with no
                // SetRecord as the contrast (:RunModalWithRecord_HandlerSetRecord_Cancel, :RunModalWithRecord_NoSetRecord_LookupOK).
                let lookup_ok = matches!(v, Value::Option(n, m) if m.get(*n as usize).is_some_and(|a| a == "LookupOK"));
                if let (true, Some(r), true) = (lookup_ok, &record, self.has_source(st)) {
                    let page_rec = self.page_rec(st);
                    if page_rec.borrow().table == r.borrow().table
                        && page_rec.borrow().fields.len() == r.borrow().fields.len()
                    {
                        let fields = page_rec.borrow().fields.clone();
                        if !Value::all_identical(&fields, &r.borrow().fields) {
                            r.borrow_mut().fields = fields;
                        }
                    }
                }
                Ok(v.clone())
            }
            other => unsupported(format!(
                "a Page-typed ModalPageHandler left {} in Response",
                other.describe()
            )),
        }
    }

    /// Hands the open page `st` to its handler, closes it if the handler left it open, and for a
    /// modal page reports how it closed.
    pub(super) fn answer_page(&mut self, st: &St, handler: Handler, modal: bool) -> Result<Value, Signal> {
        let (hobject, hroutine, hinst) = handler;
        let st = st.clone();
        self.ui_state.page_handler_depth += 1;
        let handled = self.call(hobject, hroutine, hinst, vec![Value::TestPage(st.clone())]);
        self.ui_state.page_handler_depth -= 1;
        if let Err(e) = handled {
            self.mark_closed(&st);
            return Err(e);
        }
        if st.borrow().open {
            let a = self.unattended_action(&st);
            self.close_with(&st, a, true, false)?;
            if st.borrow().open {
                // The platform closes the page the handler left open, and OnQueryClosePage
                // raises again: a second message, a second refusal. The caller gets control back
                // with the page gone and no action completed; the page's writes stay (corpus
                // handlers/TestPageQueryCloseMessageHandler_Tests.al:
                // ErrorInQueryClosePage_MessageConsumed_CallerRegainsControl, ..._SettlesThePagesUncommittedWrite).
                // A refusal by a plain `false` is not pinned for a modal page.
                if !(modal && st.borrow().close_error) {
                    return unsupported("a page whose OnQueryClosePage refused the platform's close");
                }
                self.mark_closed(&st);
                st.borrow_mut().result = Some("None".into());
            }
        }
        // A handler that left its page torn down closed it without OK (corpus
        // testpageteardown/TestPageTeardown.al:1930; the non-lookup reading is docs-only).
        if modal && !st.borrow().open && st.borrow().result.is_none() {
            let a = self.close_action(&st, "Cancel");
            st.borrow_mut().result = Some(a);
        }
        let result = st.borrow().result.clone();
        st.borrow_mut().modal = false;
        if !modal {
            return Ok(Value::Null);
        }
        let name = result.unwrap_or_else(|| "None".into());
        let members: std::sync::Arc<[String]> = al_builtins::value::ACTION_MEMBERS
            .iter()
            .map(|m| m.to_string())
            .collect();
        let ordinal = al_builtins::value::ACTION_MEMBERS
            .iter()
            .position(|m| *m == name)
            .unwrap_or(0) as i32;
        Ok(Value::Option(ordinal, members))
    }

    // ---- writing ------------------------------------------------------------------------
}

fn s_window_key(_: &Session, st: &St, i: usize) -> Option<Vec<Value>> {
    st.borrow().window.get(i).map(|w| w.key.clone())
}
