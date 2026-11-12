//! The caption a CaptionClass text stands for (moved out of rows.rs).

use super::*;

const BASE_APPLICATION_APP_ID: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";

impl Session {
    pub(super) fn read_literal_field_caption(
        &mut self,
        st: &St,
        index: usize,
        class: String,
    ) -> Result<String, Signal> {
        let revision = self.events.caption_context_revision;
        let subscriptions = self.bound_subscriptions();
        {
            let state = st.borrow();
            if let Some((_, prior_class, caption)) = state.control_captions.iter().find(|(i, _, _)| *i == index) {
                if *prior_class == class
                    && state
                        .literal_field_caption_contexts
                        .iter()
                        .any(|(i, saved_revision, saved_subscriptions)| {
                            *i == index && *saved_revision == revision && *saved_subscriptions == subscriptions
                        })
                {
                    // Corpus PR 1356 (issue 7884): repeated reads, including after an unrelated control edit.
                    al_builtins::assumptions::assume(
                        al_builtins::assumptions::Assumption::LiteralFieldCaptionResolutionReused,
                    );
                    return Ok(caption.clone());
                }
                return unsupported(
                    "a second Caption() of a field CaptionClass that raises OnResolveCaptionClass (whether BC raises it again is not pinned)",
                );
            }
        }
        // XTablePropsTests.al:483-487 pins the first read.
        let caption = self.caption_class_text(st, &class)?;
        let mut state = st.borrow_mut();
        state.control_captions.push((index, class, caption.clone()));
        state
            .literal_field_caption_contexts
            .push((index, revision, subscriptions));
        Ok(caption)
    }

    /// The caption a CaptionClass text stands for. Area 3 is the literal text. Other numeric areas
    /// go to the platform's translation (`caption_class_translate`). An area that is not a number, or no
    /// area at all, goes to no built-in resolver: the System Application's Caption Class codeunit
    /// raises OnResolveCaptionClass for it, and when no subscriber claims it the text stays as
    /// written (corpus testpage/TestPageCaptionClass.al:90
    /// TestPageField_Caption_UnresolvedCaptionClass_ReturnsExpressionUnchanged;
    /// format-evaluate/autoformat/TestCaptionClassResolverFailure.al: an error the subscriber raises reaches the
    /// open, and a subscriber that claims nothing changes nothing). The language handed to the
    /// event is the session language (types/PSCaptionClass.al).
    /// The caption a control's CaptionClass text stands for. `3,` with nothing after the comma shows the caption of the last other
    /// control on the same field (its Caption, else the field's caption), or the control's own caption when it is the only one on
    /// its field (pages/properties/Y80CaptionClassEmpty.al, Y80_CC_Card_OpenView, _OpenEdit, _OpenNew,
    /// Y80_CC_List_*, Y80_CC_Worksheet_*, Y80_CC_Doc_AfterNext: the same on BC 27, 28 and 29, on every page type and open mode,
    /// after a move and after a SetValue; `CtlLit`, `CtlExpr`, `CtlAlone`, `CtlAloneExpr`, `CtlVar`). The literal and the
    /// expression forms behave the same. Needs a corpus bump for the Y80 file.
    pub(super) fn caption_class_text_for(&mut self, st: &St, idx: usize, text: &str) -> Result<String, Signal> {
        self.raise_control_area3_after_resolve(text)?;
        if text != "3," {
            return self.caption_class_text(st, text);
        }
        self.refuse_caption_class_translate()?;
        let engine = self.engine();
        let page = page_of(&engine, st);
        let me = &page.controls[idx];
        let others: Vec<usize> = (0..page.controls.len())
            .filter(|&i| {
                i != idx && page.controls[i].binding == me.binding && page.controls[i].var_field == me.var_field
            })
            .collect();
        let me_bound = matches!(me.binding, al_ir::Binding::Field(_)) || me.var_field.is_some();
        let Some(&last) = others.last().filter(|_| me_bound) else {
            return self.control_caption_plain(st, idx);
        };
        if page.controls[last].prop("captionclass").is_some() {
            return unsupported(
                "CaptionClass '3,' over a control whose last other control on the field has a CaptionClass of its own (which caption BC reads is not pinned)",
            );
        }
        self.control_caption_plain(st, last)
    }

    /// `caption_class_text_deferring` for the caption class of control `idx` (`3,` is the control's own shared-field rule).
    pub(super) fn caption_class_text_deferring_for(
        &mut self,
        st: &St,
        idx: usize,
        text: &str,
    ) -> Result<(String, Option<String>), Signal> {
        if text == "3," {
            return Ok((self.caption_class_text_for(st, idx, text)?, None));
        }
        self.caption_class_text_deferring(st, text)
    }

    pub(super) fn caption_class_text(&mut self, st: &St, text: &str) -> Result<String, Signal> {
        match self.caption_class_text_deferring(st, text)? {
            (caption, None) => Ok(caption),
            (_, Some(why)) => unsupported(why),
        }
    }

    /// `caption_class_text`, but a caption whose text the runtime cannot know (`setup_caption_refusal`) comes back with its
    /// reason instead of failing: the caller refuses where the text is read (runner issue 6794).
    pub(super) fn caption_class_text_deferring(
        &mut self,
        st: &St,
        text: &str,
    ) -> Result<(String, Option<String>), Signal> {
        self.refuse_caption_class_translate()?;
        if let Some(rest) = text.strip_prefix("3,") {
            if rest.is_empty() {
                // P84Captions.al Page_NoSubscriber c06: a control whose CaptionClass is '3,' reads the caption of another
                // control on the same field ('Ctl Plain'), not its own; which one BC takes is not pinned.
                return unsupported(
                    "CaptionClass '3,' with nothing after the comma on a page control: which caption BC reads is not pinned",
                );
            }
            // With an OnAfterCaptionClassResolve subscriber an area 3 caption of a field goes through the translation that
            // raises it, once per session and language (Y71CaptionClassEvents.al:168-170 `lang_1030|log` `after ... expr=3,Lit cap`,
            // :198-200 `lang_1031_third|log` empty).
            if self.after_resolve_subscribed() {
                return self.caption_class_translate_deferring(text);
            }
            return Ok((rest.to_string(), None));
        }
        let (area, expr) = text.split_once(',').unwrap_or(("", text));
        // An empty area before the comma is not resolved either (P84Captions.al: c14 ',x', no event call).
        if text.contains(',') && (area.trim().parse::<u32>().is_ok() || area.is_empty()) {
            // A numeric area other than 3 (dimension, VAT, county, currency captions of Base Application) goes to the platform's
            // translation as FieldCaption does: Base Application's subscribers resolve it (record/captions/
            // QdCaptionClassAreas.al:422-481 Page_ControlCaptions `lit20`, `lit53`, `pageExpr`), a text no subscriber resolves stays
            // as written, and a text the session translated before raises no event (pages/triggers/P84Captions.al:
            // Page_NoSubscriber c01..c13, Page_Log_*, Page_Resolve_A/B after the table reads of the same texts).
            return self.caption_class_translate_deferring(text);
        }
        let _ = st;
        Ok((self.raise_resolve_caption_class(area, expr, text)?, None))
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
        match self.caption_class_translate_deferring(text)? {
            (caption, None) => Ok(caption),
            (_, Some(why)) => unsupported(why),
        }
    }

    /// `caption_class_translate` with the refusal of `setup_caption_refusal` returned beside the caption.
    pub(crate) fn caption_class_translate_deferring(&mut self, text: &str) -> Result<(String, Option<String>), Signal> {
        self.refuse_caption_class_translate()?;
        self.caption_class_translate_unrefused(text)
    }

    /// `caption_class_translate_deferring` without the refusal for a `UI Helper Triggers` subscriber: for the callers that
    /// pin what that event does (`ui_caption_translate.rs`).
    pub(super) fn caption_class_translate_unrefused(&mut self, text: &str) -> Result<(String, Option<String>), Signal> {
        let after = self.after_resolve_subscribed();
        let Some((area, expr)) = text.split_once(',') else {
            if after {
                return self.raise_unclaimed_after_resolve(text);
            }
            return Ok((text.to_string(), None));
        };
        if area == "3" && !after {
            return Ok((expr.to_string(), None));
        }
        if area.is_empty() && after {
            return self.raise_unclaimed_after_resolve(text);
        }
        // An area 3 text is kept per language: a language that has not translated it yet raises the event again
        // (Y71CaptionClassEvents.al CaptionClass_LanguageChangedBetweenOpens: `lang_1036_second|log`, `lang_1031_third|log` empty).
        if area == "3" {
            let language = self.global_language;
            if let Some((_, _, caption)) = self
                .caption_translations
                .iter()
                .find(|(t, l, _)| t == text && *l == language)
            {
                return Ok((caption.clone(), None));
            }
            let caption = self.raise_after_caption_class_resolve(text, expr.to_string())?;
            self.caption_translations
                .push((text.to_string(), language, caption.clone()));
            return Ok((caption, None));
        }
        if let Some((_, language, caption)) = self.caption_translations.iter().find(|(t, _, _)| t == text) {
            if *language != self.global_language {
                return unsupported(
                    "CaptionClassTranslate of a text translated before GlobalLanguage changed (whether BC translates it again is not pinned)",
                );
            }
            let caption = caption.clone();
            let why = self.setup_caption_refusal(area, text, &caption);
            return Ok((caption, why));
        }
        if matches!(area.trim(), "1" | "101") && !self.gl_setup_written {
            self.setup_caption_read_early = true;
        }
        // Area 3 raises only OnAfterCaptionClassResolve, with the text after the comma (Y71CaptionClassEvents.al:243-246
        // `area3_x3|log`, kept for the session: `area3_x3_again|log` empty).
        let caption = self.raise_resolve_caption_class(area, expr, text)?;
        self.caption_translations
            .push((text.to_string(), self.global_language, caption.clone()));
        let why = self.setup_caption_refusal(area, text, &caption);
        Ok((caption, why))
    }

    /// A table field's CaptionClass with no comma, or with an empty area, while a subscriber listens to OnAfterCaptionClassResolve:
    /// the first read raises the event once with the expression as both the expression and the caption, and the caption is
    /// what the subscribers leave (corpus record/captions/WcExtFieldAfterResolve.al WcExtField_NoComma_FieldCaption `WCNOCOMMA`,
    /// WcExtField_EmptyArea_FieldCaption `,WC empty area`; BC 27.5, 28.5, 29.0). OnResolveCaptionClass is not raised for an
    /// empty area (pages/triggers/P84Captions.al c14 `,x`); for no comma the probe cannot tell (its OnResolve subscribers
    /// claim only their own areas), so none is raised there either.
    fn raise_unclaimed_after_resolve(&mut self, text: &str) -> Result<(String, Option<String>), Signal> {
        if text.is_empty() {
            // The empty text comes back unchanged (types/PSCaptionClass.al:5-8); that it raises no event is an assumption
            // (docs/probes-needed.md).
            return Ok((String::new(), None));
        }
        if let Some((_, language, caption)) = self.caption_translations.iter().find(|(t, _, _)| t == text) {
            if *language != self.global_language {
                return unsupported(
                    "CaptionClassTranslate of a text translated before GlobalLanguage changed (whether BC translates it again is not pinned)",
                );
            }
            return Ok((caption.clone(), None));
        }
        let caption = self.raise_after_caption_class_resolve(text, text.to_string())?;
        self.caption_translations
            .push((text.to_string(), self.global_language, caption.clone()));
        Ok((caption, None))
    }

    /// Base Application resolves areas 1 (dimensions) and 101 (currencies) from General Ledger Setup. The corpus database
    /// holds CRONUS's setup (record/captions/QdCaptionClassAreas.al:254 `Amount (GBP)`, :256 `Department Code`);
    /// without Microsoft's test data the runtime's setup row is the one the install code writes, so a caption those areas
    /// resolved is refused. A text they leave as written does not read the setup (pages/triggers/P84Captions.al:
    /// '1,1,x' and the other area 1 texts).
    /// A test that wrote General Ledger Setup before any caption of these areas was resolved in the session is answered for what
    /// it wrote: area 101 type 0 (the LCY Code) and area 1 type 1 reference 1 (Global Dimension 1 Code and its Dimension row)
    /// (QdCaptionClassAreas.al:267 `lit101` 'Amount (GBP)', :269 `lit11` 'Department Code', :311 `expr101`, after
    /// record/QkSetupRows.al WriteGLSetupCaptions). The other types read fields that test does not write.
    fn setup_caption_refusal(&self, area: &str, text: &str, caption: &str) -> Option<String> {
        let written = self.gl_setup_written && !self.setup_caption_read_early && {
            let expr = text.split_once(',').map_or("", |(_, e)| e);
            match area.trim() {
                "101" => expr.starts_with("0,"),
                "1" => expr == "1,1" || expr.starts_with("1,1,"),
                _ => false,
            }
        };
        // In a company the DemoTool built, General Ledger Setup is the row BC's own build writes (the DemoTool runs the same
        // code on BC), so the premise of this refusal does not hold there (`Baseline::company_built`).
        if self.engine.test_data.is_none()
            && !self.company_built
            && matches!(area.trim(), "1" | "101")
            && caption != text
            && !written
        {
            return Some(format!(
                "CaptionClass `{text}`: Base Application resolves area {area} from General Ledger Setup, whose corpus database row the runtime's database does not hold"
            ));
        }
        None
    }

    /// Whether a compiled OnResolveCaptionClass subscriber belongs to an app other than the Base Application. Base Application's
    /// resolvers (codeunits 342 Currency and 343 Dimension Caption Class Mgmt) only assign Caption and Resolved, so an unknown
    /// caption text is all a test can observe of them; another app's subscriber could write state, and then the open is refused
    /// whenever its text is unknown (runner issue 6794; earlier measurements in 4920 and 5376).
    pub(super) fn foreign_caption_class_subscriber(&self) -> bool {
        let program = &self.engine.program;
        let base = |object: usize| {
            program
                .apps
                .get(program.objects[object].app as usize)
                .is_some_and(|a| a.id.eq_ignore_ascii_case(BASE_APPLICATION_APP_ID))
        };
        self.engine
            .events
            .subscribers_of(crate::events::CAPTION_CLASS)
            .iter()
            .any(|&(object, _)| !base(object))
    }

    /// Every caption class the client translates raises `CaptionClassTranslate` of the platform codeunit `UI Helper Triggers`,
    /// area 3 included (events/platform-triggers/P114GlobalTriggerEvents.al:301, :304, :323-327). The runtime does
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

    /// OnResolveCaptionClass, then OnAfterCaptionClassResolve with the caption it gave (a claimed one, else the whole text):
    /// the System Application's Caption Class codeunit raises both for every translation (Y71CaptionClassEvents.al:235-236
    /// `x1_1033|log` 'resolve area=LTY71 expr=x1 ... | after ... expr=LTY71,x1 caption=res 1033').
    pub(super) fn raise_resolve_caption_class(&mut self, area: &str, expr: &str, text: &str) -> Result<String, Signal> {
        let caption = self.raise_on_resolve_caption_class(area, expr, text)?;
        self.raise_after_caption_class_resolve(text, caption)
    }

    /// Whether OnAfterCaptionClassResolve of the Caption Class codeunit would reach a subscriber now (a manual one only while
    /// bound, as Y71CaptionClassEvents.al binds its own).
    pub(crate) fn after_resolve_subscribed(&mut self) -> bool {
        self.event_reaches_any(crate::events::CAPTION_CLASS, "onaftercaptionclassresolve")
    }

    /// A CaptionClass expression whose result is `3,<text>`: with an OnAfterCaptionClassResolve subscriber it is translated like
    /// the literal `3,<text>` (the event once per text and language), else it is `<text>` (pages/extensions/
    /// TableExtCaptionClass.al:124-149). That an expression result raises the event as a literal does is an assumption
    /// (docs/probes-needed.md); only a subscriber's log can tell.
    pub(crate) fn expression_text_caption(&mut self, class: &str, text: String) -> Result<String, Signal> {
        if self.after_resolve_subscribed() {
            return self.caption_class_translate(class);
        }
        Ok(text)
    }

    /// The caption a page control over field `field` shows for the text `class` its CaptionClass expression gave: an empty
    /// text and `3,` read as the empty text (QdCaptionClassAreas.al Page_ControlCaptions `exprEmpty`, `expr3`), the rest as on a
    /// table read (`recmeta::ExprCaption`).
    pub(super) fn control_expression_caption(&mut self, field: &str, class: &str) -> Result<String, Signal> {
        use crate::recmeta::ExprCaption;
        match crate::recmeta::expression_class(class) {
            ExprCaption::FieldCaption => Ok(String::new()),
            ExprCaption::Text(c) => self.expression_text_caption(class, c),
            ExprCaption::Translate => self.caption_class_translate(class),
            ExprCaption::Plain => self.expression_plain_caption(field, class),
        }
    }

    /// The caption of field `field` whose CaptionClass expression gave `class`, a text with no comma or with an area that is
    /// not a number: the text as it is, on a table read and on a Card or List control, also over a Caption property (corpus
    /// record/captions/CaptionExpressionPlainText.al:238-262 FieldCaption and FieldRef.Caption `asint=All|plainformat=Item|
    /// plainlabel=Plain label text|...|nonnumeric=Zed,some text|withcaption=Item`, :280 inside the Error of OnInsert `Already
    /// there. Resource = .`, :296-344 the Card and List controls; BC 27.5, 28.5 and 29.0; al-runner issue 7382).
    /// The text goes to the translation as a literal of the same shape does: a text with no comma raises no event
    /// (types/PSCaptionClass.al:5-8), or OnAfterCaptionClassResolve once per session while a subscriber listens
    /// (record/captions/WcExtFieldAfterResolve.al WcExtField_NoComma_FieldCaption; the corpus run that pins the captions above
    /// has that static subscriber, WcExtFieldAfterResolve.al:248); an area that is not a number raises OnResolveCaptionClass
    /// (record/captions/R6985FieldCaptionArea.al:88-151, with the static subscriber at :61). That an expression result raises
    /// them as a literal does only a subscriber's log can tell (docs/probes-needed.md). A caption a subscriber changed is not
    /// pinned for an expression (al-runner issue 7431): refused.
    pub(crate) fn expression_plain_caption(&mut self, field: &str, class: &str) -> Result<String, Signal> {
        let caption = self.caption_class_translate(class)?;
        if caption != class {
            return unsupported(format!(
                "the caption of field {field} whose CaptionClass expression gives `{class}`, a text with no comma or with an area that is not a number, which a subscriber resolves (not pinned for an expression, al-runner issue 7431)"
            ));
        }
        Ok(caption)
    }

    /// A text other than the one the control resolved before, at a fill other than the one after SetValue
    /// (`refresh_caption_classes`, testpage/W2CaptionClass.al:174-178): whether BC resolves it there (after an action, on
    /// the next row) is not pinned.
    pub(super) fn refuse_caption_class_changed_at_fill(st: &St, idx: usize, t: &str) -> Result<(), Signal> {
        if super::format::raises_resolve(t)
            && st
                .borrow()
                .control_captions
                .iter()
                .any(|(i, old, _)| *i == idx && old != t)
        {
            return unsupported(
                "a CaptionClass expression that gives another text at a fill other than after SetValue (whether BC resolves it again there is not pinned)",
            );
        }
        Ok(())
    }

    /// A page control's own CaptionClass `3,<text>` while a subscriber listens to OnAfterCaptionClassResolve: opening the page
    /// raises the event once with the text after the comma as the caption, and the control reads that caption; later reads
    /// raise nothing (corpus record/captions/WcExtFieldAfterResolve.al WcControl_OwnArea3_TestPageCaption: `log-open`
    /// `after expr=3,WC ctl three in=WC ctl three`, `control-caption` `WC ctl three`, `log-after-control` empty). `3,` alone
    /// raises nothing, as on a field (WcExtField_ThreeAlone_FieldCaption); for a control that is an assumption, only a
    /// subscriber's log can tell (docs/probes-needed.md).
    pub(super) fn raise_control_area3_after_resolve(&mut self, text: &str) -> Result<(), Signal> {
        if text.len() > 2 && text.starts_with("3,") && self.after_resolve_subscribed() {
            self.caption_class_translate_deferring(text)?;
        }
        Ok(())
    }

    /// OnAfterCaptionClassResolve(Language, CaptionExpression, var Caption) with the session language; the caption its
    /// subscribers leave is the translation (Y71CaptionClassEvents.al:115-120, :235-246; al-runner issue 6924, a subscriber that
    /// resolves its own area there).
    pub(super) fn raise_after_caption_class_resolve(&mut self, text: &str, caption: String) -> Result<String, Signal> {
        self.note_event(crate::events::CAPTION_CLASS, "onaftercaptionclassresolve", None);
        if !self.after_resolve_subscribed() {
            return Ok(caption);
        }
        let names: Vec<String> = ["Language", "CaptionExpression", "Caption"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let mut args = vec![
            Value::Integer(self.global_language),
            Value::Text(text.to_string().into()),
            Value::Text(caption.into()),
        ];
        let call = crate::dispatch::EventCall {
            object: crate::events::CAPTION_CLASS,
            event: "onaftercaptionclassresolve",
            element: None,
            isolated: false,
            names: &names,
            sender: None,
        };
        // The publisher declares `var Caption: Text[1024]` (BCApps Caption Class/src/CaptionClass.Codeunit.al:40).
        self.events.publisher_var_lens = vec![None, None, Some(1024)];
        let raised = self.raise(&call, &mut args);
        self.events.publisher_var_lens.clear();
        raised?;
        match &args[2] {
            Value::Text(t) | Value::Code(t) => Ok(t.to_string()),
            other => unsupported(format!(
                "an OnAfterCaptionClassResolve subscriber left Caption as {}",
                other.describe()
            )),
        }
    }

    fn raise_on_resolve_caption_class(&mut self, area: &str, expr: &str, text: &str) -> Result<String, Signal> {
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
        // `var Caption: Text` (CaptionClass.Codeunit.al:29): no length.
        self.events.publisher_var_lens = vec![None, None, None, Some(i32::MAX), None];
        let raised = self.raise(&call, &mut args);
        self.events.publisher_var_lens.clear();
        raised?;
        Ok(match (&args[4], &args[3]) {
            (Value::Boolean(true), Value::Text(t) | Value::Code(t)) => t.to_string(),
            _ => text.to_string(),
        })
    }
}
