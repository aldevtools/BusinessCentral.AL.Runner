//! The literal own-area CaptionClass texts of a page's source table fields, resolved when the page opens.
//!
//! Pinned by record/captions/CaptionClassOwnArea.al:587-633 (`OwnArea_Issue7410_Card_LiteralCaptionReadTwice`, al-runner issue
//! 7410; BC 27, 28 and 29): a Card opened by the test with OpenEdit (:598) and later with OpenView (:610) on its one row (:594-596)
//! raises OnResolveCaptionClass at the open for every field of the source table with such a text, in field order, also for the
//! field without a control (:622 `(CCAOCL|2,1|s)(CCAOCM|2,1|s)`), and only once per session and text (:628 the second open
//! raises nothing). `Caption()` of the control raises nothing, before and after a SetValue (:623-626, :629), and FieldCaption and
//! CaptionClassTranslate of the same text later read the session's translation (:630-632).
//!
//! Every other page keeps main's path: the caption is resolved at the first `Caption()` (`read_literal_field_caption`).

use super::*;
use al_ir::{ControlKind, PageType};

impl Session {
    /// At the first row fill since the open: when the page is the pinned shape, translate the literal CaptionClass of every field
    /// of the source table in field order through the session's translations (which raise OnResolveCaptionClass for a text the
    /// session has not translated yet), with every dialog and error with a text refused. Later fills do nothing.
    pub(super) fn resolve_literal_field_classes_at_open(&mut self, st: &St) -> Result<(), Signal> {
        if st.borrow().literal_classes_at_open != LiteralClassesAtOpen::Unchecked {
            return Ok(());
        }
        let Some(classes) = self.literal_field_classes_open_pin(st) else {
            st.borrow_mut().literal_classes_at_open = LiteralClassesAtOpen::Main;
            return Ok(());
        };
        st.borrow_mut().literal_classes_at_open = LiteralClassesAtOpen::Resolved {
            revision: self.events.caption_context_revision,
            subscriptions: self.bound_subscriptions(),
        };
        // The pin's subscriber shows no dialog and raises no error; what BC does with one at the open is not measured.
        self.with_caption_dialogs(CaptionDialogs::Refused, |this| {
            classes
                .iter()
                .try_for_each(|class| this.caption_class_translate(class).map(drop))
        })
    }

    /// `Caption()` of a control over a field whose literal CaptionClass the open resolved (`LiteralClassesAtOpen::Resolved`):
    /// the session's translation of `class` in the current language, raising nothing (CaptionClassOwnArea.al:623-626, :629).
    /// `None` on any other page. A read after the session dropped that translation (SelectLatestVersion), changed language, or
    /// bound or unbound a subscription since the open is not measured: refused.
    pub(super) fn literal_field_caption_after_open(&self, st: &St, class: &str) -> Option<Result<String, Signal>> {
        let LiteralClassesAtOpen::Resolved {
            revision,
            subscriptions,
        } = st.borrow().literal_classes_at_open.clone()
        else {
            return None;
        };
        let language = self.global_language;
        let kept = (revision == self.events.caption_context_revision && subscriptions == self.bound_subscriptions())
            .then(|| {
                self.caption_translations
                    .iter()
                    .find(|(t, l, _)| t == class && *l == language)
                    .map(|(_, _, caption)| caption.clone())
            })
            .flatten();
        Some(match kept {
            Some(caption) => Ok(caption),
            None => unsupported(
                "Caption() of a field CaptionClass the page resolved at the open, after the session dropped that translation or changed its language or bindings (not pinned)",
            ),
        })
    }

    /// The literal CaptionClass texts of the source table's fields, in field order, when page `st` is the pinned shape of
    /// CaptionClassOwnArea.al:143-155 and :210-223, every fact required:
    /// - a top-level Card the test opened with OpenEdit (:598) or OpenView (:610), on a row (:594-596), at the open's own fill (no
    ///   test call on the page since the open);
    /// - page properties PageType = Card and UsageCategory = None besides SourceTable, no page trigger, no action, no page
    ///   extension and no subscriber (:210-213);
    /// - every control a field over Rec directly in the content area, with ApplicationArea = All only and no trigger (:219-220);
    /// - the source table runs no code (no trigger, procedure, table extension or subscriber), and each of its fields has no
    ///   CaptionClass or a literal one with an area that is not a number, not empty and not 3, before a comma (:150-151); at least
    ///   one has.
    fn literal_field_classes_open_pin(&self, st: &St) -> Option<Vec<String>> {
        let (opened, on_row, object, top) = {
            let s = st.borrow();
            (
                s.test_open_edit() || s.test_open_view(),
                s.cursor == Cursor::Row,
                s.object as usize,
                s.host.is_none(),
            )
        };
        let at_open = {
            let s = st.borrow();
            s.tree_calls.is_empty() && !s.tree_calls_unknown
        };
        if !opened || !on_row || !top || !at_open || !self.has_source(st) || !self.unextended_unsubscribed(st) {
            return None;
        }
        let engine = self.engine();
        if engine.object_facts[object].page_type != Some(PageType::Card) {
            return None;
        }
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
        if props
            != [
                ("pagetype".into(), "card".into()),
                ("usagecategory".into(), "none".into()),
            ]
        {
            return None;
        }
        let plain_field = |c: &ir::Control| {
            c.kind == ControlKind::Field
                && c.parent.is_none()
                && c.owner.is_none()
                && matches!(c.binding, ir::Binding::Field(_))
                && c.triggers.is_empty()
                && c.ext_triggers.is_empty()
                && matches!(c.props.as_slice(), [(n, ir::PropVal::Lit(v))]
                    if n == "applicationarea" && v.trim().eq_ignore_ascii_case("All"))
        };
        if page.controls.is_empty() || !page.controls.iter().all(plain_field) {
            return None;
        }
        let table = self.page_rec(st).borrow().table as usize;
        let t = self.engine.tables[table].as_ref()?;
        let code_free = engine.program.objects[table].routines.is_empty()
            && engine.program.objects[table]
                .table
                .as_ref()
                .is_some_and(|x| x.extensions.is_empty())
            && !engine.events.any_subscriber_of(table);
        if !code_free {
            return None;
        }
        let mut classes = Vec::new();
        for pos in 0..t.fields.len() {
            let Some(raw) = self.field_property(t, pos, "CaptionClass") else {
                continue;
            };
            let class = crate::recmeta::string_literal(&raw)?;
            if !super::format::raises_resolve(&class) {
                return None;
            }
            classes.push(class);
        }
        (!classes.is_empty()).then_some(classes)
    }
}
