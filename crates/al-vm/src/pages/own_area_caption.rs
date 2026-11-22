//! Own-area CaptionClass texts (an area that is not a number, resolved by an OnResolveCaptionClass subscriber) of a page's source
//! table fields, on the pages record/captions/CaptionClassOwnArea.al pins (al-runner issues 7410 and 7431; BC 27, 28 and 29).
//!
//! - The literal texts (:587-633 `OwnArea_Issue7410_Card_LiteralCaptionReadTwice`): the open of the Card raises
//!   OnResolveCaptionClass for every field of the source table with such a text, in field order, also for the field without a
//!   control (:622 `(CCAOCL|2,1|s)(CCAOCM|2,1|s)`), once per session and text (:628, the second open raises nothing).
//!   `Caption()` of the control raises nothing, before and after a SetValue (:623-626, :629). A first OpenView raises at the open
//!   too (record/captions/WcExtFieldAfterResolve.al:403, :411 `log-open`). Every other page keeps main's path: the caption is
//!   resolved at the first `Caption()` (`read_literal_field_caption`).
//! - An expression (:315-357 `ReadCard`, `ReadList`, :543-583 the Card of issue 7431): the control shows the subscriber's
//!   caption on the Card opened with OpenEdit and the List opened with OpenView. Elsewhere it stays refused
//!   (`expression_plain_caption`).
//!
//! Each path requires every fact of its pin by equality (`own_area_pinned_page`, `own_area_session_pinned`,
//! `own_area_calls_pinned`).

use super::pinned_shape::TestCall;
use super::*;
use al_builtins::page_op::PageMethod;
use al_ir::{ControlKind, PageType};

/// A test call of the pinned sequences: a control's `Caption()` or a value typed into a control.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PinnedCall {
    Caption,
    Write,
}

/// The test calls on the literal Card after its first open (CaptionClassOwnArea.al:600-608: Caption, Caption, SetValue, Caption;
/// then Close). The second open reads once (:612).
const LITERAL_CARD_CALLS: [PinnedCall; 4] = [
    PinnedCall::Caption,
    PinnedCall::Caption,
    PinnedCall::Write,
    PinnedCall::Caption,
];
/// The test calls on the Card of an expression after its open: four reads, a SetValue, a read (:557-569, issue 7431); `ReadCard`
/// makes the first two (:327-331).
const EXPRESSION_CARD_CALLS: [PinnedCall; 6] = [
    PinnedCall::Caption,
    PinnedCall::Caption,
    PinnedCall::Caption,
    PinnedCall::Caption,
    PinnedCall::Write,
    PinnedCall::Caption,
];
/// The test calls on the List of an expression after its open: two reads (:349-353 `ReadList`).
const EXPRESSION_LIST_CALLS: [PinnedCall; 2] = [PinnedCall::Caption, PinnedCall::Caption];

impl Session {
    /// At the first row fill since the open: when the page is the pinned Card (`literal_field_classes_open_pin`), translate the
    /// literal CaptionClass of every field of the source table in field order through the session's translations (which raise
    /// OnResolveCaptionClass for a text the session has not translated yet), with every dialog and error with a text refused.
    /// Later fills do nothing.
    pub(super) fn resolve_literal_field_classes_at_open(&mut self, st: &St) -> Result<(), Signal> {
        if st.borrow().literal_classes_at_open != LiteralClassesAtOpen::Unchecked {
            return Ok(());
        }
        let Some(classes) = self.literal_field_classes_open_pin(st)? else {
            st.borrow_mut().literal_classes_at_open = LiteralClassesAtOpen::Main;
            return Ok(());
        };
        st.borrow_mut().literal_classes_at_open = LiteralClassesAtOpen::Resolved {
            revision: self.events.caption_context_revision,
        };
        // The pin's subscriber shows no dialog and raises no error; what BC does with one at the open is not measured.
        self.with_caption_dialogs(CaptionDialogs::Refused, |this| {
            classes
                .iter()
                .try_for_each(|class| this.caption_class_translate(class).map(drop))
        })
    }

    /// `Caption()` of a control over a field whose literal CaptionClass the open resolved (`LiteralClassesAtOpen::Resolved`):
    /// the session's translation of `class`, raising nothing (CaptionClassOwnArea.al:623-626, :629). `None` on any other page.
    /// Refused unless every test call since the open is one of the pinned sequence (`LITERAL_CARD_CALLS`) and succeeded, and
    /// the session still has the open's caption context (language, bindings: `caption_context_revision`), no bound
    /// subscription, language 1033 and the translation (SelectLatestVersion drops it).
    pub(super) fn literal_field_caption_after_open(&self, st: &St, class: &str) -> Option<Result<String, Signal>> {
        let LiteralClassesAtOpen::Resolved { revision } = st.borrow().literal_classes_at_open.clone() else {
            return None;
        };
        let kept = (revision == self.events.caption_context_revision
            && self.own_area_session_pinned()
            && Self::own_area_calls_pinned(st, &LITERAL_CARD_CALLS))
        .then(|| {
            self.caption_translations
                .iter()
                .find(|(t, l, _)| t == class && *l == self.global_language())
                .map(|(_, _, caption)| caption.clone())
        })
        .flatten();
        Some(match kept {
            Some(caption) => Ok(caption),
            None => unsupported(
                "Caption() of a field CaptionClass the page resolved at the open, after a test call, a language, a binding or a dropped translation the pin does not have (not pinned)",
            ),
        })
    }

    /// The literal CaptionClass texts of the source table's fields, in field order, when page `st` is the pinned Card of
    /// CaptionClassOwnArea.al:144-156 and :210-224 at the open's own fill, every fact required:
    /// - the page is a pinned Card (`own_area_pinned_page`), opened with OpenEdit (:598) or OpenView (:610;
    ///   WcExtFieldAfterResolve.al:403), and the test made no call on it yet;
    /// - the source table has no trigger, procedure, table extension or subscriber (table 62402 at :144-156);
    /// - every field of the table declares nothing or a literal CaptionClass alone (:150-152): a normal, enabled field that is not
    ///   obsolete; each text has an area that is not a number, not empty and not 3, before a comma (:151-152); at least one;
    /// - the session: no bound subscription and language 1033 (`own_area_session_pinned`);
    /// - no text would refuse its translation (`caption_class_translate_refuses_now`): the open then keeps main's path instead of
    ///   carrying that refusal to a test that reads no caption.
    fn literal_field_classes_open_pin(&mut self, st: &St) -> Result<Option<Vec<String>>, Signal> {
        let at_open = {
            let s = st.borrow();
            (s.test_open_edit() || s.test_open_view()) && s.tree_calls.is_empty() && !s.tree_calls_unknown
        };
        if !at_open || !self.has_source(st) || !self.own_area_session_pinned() {
            return Ok(None);
        }
        let table = self.page_rec(st).borrow().table as usize;
        if !self.table_code_is(table, &[]) {
            return Ok(None);
        }
        let mut classes = Vec::new();
        {
            let engine = self.engine();
            let Some(def) = engine.program.objects[table].table.as_ref() else {
                return Ok(None);
            };
            // The platform's system fields (ids from 2000000000) are on every table of the pin too.
            for field in def.fields.iter().filter(|f| f.id < 2_000_000_000) {
                match field.properties.as_slice() {
                    [] => {}
                    [(k, raw)] if k.eq_ignore_ascii_case("CaptionClass") => match crate::recmeta::string_literal(raw) {
                        Some(class) if super::format::raises_resolve(&class) => classes.push(class),
                        _ => return Ok(None),
                    },
                    _ => return Ok(None),
                }
            }
        }
        if classes.is_empty() || classes.iter().any(|c| self.caption_class_translate_refuses_now(c)) {
            return Ok(None);
        }
        // The page shape last: it counts the table's rows.
        Ok((self.own_area_pinned_page(st)? == Some(PageType::Card)).then_some(classes))
    }

    /// Whether a control of page `st` shows the caption a subscriber gave its field's own-area CaptionClass expression: a pinned
    /// Card (`own_area_pinned_page`) the test opened with OpenEdit (CaptionClassOwnArea.al:325, :555) or a pinned List it opened
    /// with OpenView (:347), in the pinned session (`own_area_session_pinned`), after only the pinned test calls
    /// (`EXPRESSION_CARD_CALLS`, `EXPRESSION_LIST_CALLS`). The source table may declare procedures (table 62400's `DynCc` at :110)
    /// but no trigger (:95-115, :117-141).
    pub(super) fn own_area_control_caption_pinned(&mut self, st: &St) -> Result<bool, Signal> {
        let (edit, view) = {
            let s = st.borrow();
            (s.test_open_edit(), s.test_open_view())
        };
        if !self.own_area_session_pinned() {
            return Ok(false);
        }
        let calls: &[PinnedCall] = match self.own_area_pinned_page(st)? {
            Some(PageType::Card) if edit => &EXPRESSION_CARD_CALLS,
            Some(PageType::List) if view => &EXPRESSION_LIST_CALLS,
            _ => return Ok(false),
        };
        let table = self.page_rec(st).borrow().table as usize;
        Ok(self.table_runs_no_trigger(table) && Self::own_area_calls_pinned(st, calls))
    }

    /// `Caption()` of a control over field `field` whose caption the page filled with a subscriber's for an own-area expression
    /// (`own_area_control_caption_pinned` held at the fill): refused unless the calls since the open are still a pinned sequence.
    pub(super) fn refuse_own_area_expression_read(&mut self, st: &St, field: &str) -> Result<(), Signal> {
        if self.own_area_control_caption_pinned(st)? {
            return Ok(());
        }
        unsupported(format!(
            "Caption() of field {field}, whose own-area CaptionClass expression a subscriber resolved, after a test call or a session change the pin does not have (not pinned for an expression, al-runner issue 7431)"
        ))
    }

    /// Whether page `st` is one of the pages of CaptionClassOwnArea.al (pages 62400 to 62403 at :158-224), every fact required, and
    /// which type: a top-level page that declares PageType (Card or List), SourceTable and UsageCategory = None and nothing else, no
    /// trigger, no action, no page extension and no subscriber; a Card's controls are fields over Rec directly in the content area,
    /// a List's are one repeater with such fields under it, each with ApplicationArea = All alone and no trigger; the source table
    /// declares DataClassification alone (:97, :119, :146), is not temporary, and holds one row (:274-281 `Seed`,
    /// :550-552, :594-596), shown with no filter on its primary key.
    fn own_area_pinned_page(&mut self, st: &St) -> Result<Option<PageType>, Signal> {
        let (top, on_row, object) = {
            let s = st.borrow();
            (s.host.is_none(), s.cursor == Cursor::Row, s.object as usize)
        };
        if !top || !on_row || !self.has_source(st) || !self.page_declares_within(st, &[], &[]) {
            return Ok(None);
        }
        let page_type = {
            let engine = self.engine();
            let page = page_of(&engine, st);
            if !page.triggers.is_empty() || !page.action_areas.is_empty() {
                return Ok(None);
            }
            let page_type = engine.object_facts[object].page_type.clone();
            let type_name = match page_type {
                Some(PageType::Card) => "card",
                Some(PageType::List) => "list",
                _ => return Ok(None),
            };
            let mut props: Vec<(String, String)> = engine.program.objects[object]
                .properties
                .iter()
                .map(|(k, v)| (k.to_ascii_lowercase(), v.trim().to_ascii_lowercase()))
                .filter(|(k, _)| k != "sourcetable")
                .collect();
            props.sort();
            if props
                != [
                    ("pagetype".into(), type_name.into()),
                    ("usagecategory".into(), "none".into()),
                ]
            {
                return Ok(None);
            }
            let field = |c: &ir::Control, parent: Option<usize>| {
                c.kind == ControlKind::Field
                    && c.parent == parent
                    && matches!(c.binding, ir::Binding::Field(_))
                    && c.triggers.is_empty()
                    && matches!(c.props.as_slice(), [(n, ir::PropVal::Lit(v))]
                        if n == "applicationarea" && v.trim().eq_ignore_ascii_case("All"))
            };
            let controls = match page.controls.split_first() {
                None => false,
                Some((repeater, rest)) if page_type == Some(PageType::List) => {
                    repeater.kind == ControlKind::Repeater
                        && repeater.parent.is_none()
                        && repeater.props.is_empty()
                        && repeater.triggers.is_empty()
                        && !rest.is_empty()
                        && rest.iter().all(|c| field(c, Some(0)))
                }
                Some(_) => page.controls.iter().all(|c| field(c, None)),
            };
            if !controls {
                return Ok(None);
            }
            page_type
        };
        let rec = self.page_rec(st);
        let table = rec.borrow().table as usize;
        let table_props = {
            let engine = self.engine();
            let props = &engine.program.objects[table].properties;
            matches!(props.as_slice(), [(k, _)] if k.eq_ignore_ascii_case("DataClassification"))
        };
        if !table_props || rec.borrow().temporary || !self.rec_view_is(st, 0) {
            return Ok(None);
        }
        Ok((self.stored_rows_passing(&rec, Vec::new())? == 1)
            .then_some(page_type)
            .flatten())
    }

    /// The session facts of every CaptionClassOwnArea.al pin: its one subscriber is static (:65), so no subscription is bound, and
    /// the session language is the default one (`s` at :622), 1033.
    fn own_area_session_pinned(&self) -> bool {
        self.global_language() == 1033 && self.bound_subscriptions().is_empty()
    }

    /// Whether the test calls on page `st` since it opened, the running one last, are a prefix of `pinned` and every one before
    /// the running one succeeded (`test_calls_all_succeeded`). True before the first call (the open's own fill).
    fn own_area_calls_pinned(st: &St, pinned: &[PinnedCall]) -> bool {
        {
            let s = st.borrow();
            if s.host.is_none() && !s.tree_calls_unknown && s.tree_calls.is_empty() {
                return true;
            }
        }
        let Some(calls) = Self::test_calls_all_succeeded(st) else {
            return false;
        };
        calls.len() <= pinned.len()
            && calls.iter().zip(pinned).all(|(call, want)| {
                matches!(
                    (call, want),
                    (TestCall::Other(PageMethod::Caption), PinnedCall::Caption)
                        | (TestCall::Write(_), PinnedCall::Write)
                )
            })
    }
}
