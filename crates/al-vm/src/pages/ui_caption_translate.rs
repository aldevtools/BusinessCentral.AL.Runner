//! `CaptionClassTranslate` of the platform codeunit `UI Helper Triggers`.
//!
//! Pinned by events/platform-triggers/P114GlobalTriggerEvents.al:309-330 (`P114_CaptionClassTranslateCall`): the
//! `CaptionClassTranslate` statement raises it with (session language, the class text, the translation before the event), where
//! the translation is the text after the comma for area 3 and the whole text for an unclaimed area or a text with no comma.
//! FieldCaption of a table field raises nothing (events/platform-triggers/PtObserve.al:135-150 `Captions`, `fieldcap`).
//!
//! Not pinned, so refused when a subscriber would see it: a subscriber that changes `Translation`; the same text translated
//! again; the empty text; a translation that other subscribers (OnResolveCaptionClass, OnAfterCaptionClassResolve) changed
//! (the order of the events is not observable by the pinned cases). A TestPage control's caption class still refuses
//! (`refuse_caption_class_translate`).

use super::*;

impl Session {
    /// FieldCaption's translation of an area 3 class: no UI Helper event.
    pub(crate) fn caption_class_translate_silent(&mut self, text: &str) -> Result<String, Signal> {
        match self.caption_class_translate_unrefused(text)? {
            (caption, None) => Ok(caption),
            (_, Some(why)) => unsupported(why),
        }
    }

    /// The `CaptionClassTranslate(Text)` statement.
    pub(crate) fn caption_class_translate_statement(&mut self, text: &str) -> Result<String, Signal> {
        // A subscriber the test cannot see (it holds no variable of the subscriber's codeunit or of a SingleInstance codeunit
        // the subscriber logs to) changes nothing the test can observe: the translation runs as it did before the event was
        // raised (decided without owner review, 2026-10-19, `Registry::unraised_subscriber`).
        let test_object = self.test_object_read();
        let test = test_object.and_then(|t| self.engine.program.objects.get(t));
        let visible = self
            .engine
            .events
            .unraised_subscriber("ui helper triggers", Some("captionclasstranslate"), test, test_object)
            .is_some();
        if !visible {
            return self.caption_class_translate(text);
        }
        let language = self.global_language;
        if text.is_empty()
            || self
                .ui_caption_translated
                .iter()
                .any(|(t, l)| t == text && *l == language)
        {
            return unsupported(
                "CaptionClassTranslate of the empty text or of a text translated before, with a UI Helper Triggers subscriber (whether the event is raised is not pinned)",
            );
        }
        let before = match text.split_once(',') {
            Some(("3", rest)) => rest.to_string(),
            _ => text.to_string(),
        };
        let caption = match self.caption_class_translate_unrefused(text)? {
            (caption, None) => caption,
            (_, Some(why)) => return unsupported(why),
        };
        if caption != before {
            return unsupported(
                "CaptionClassTranslate whose translation another subscriber changed, with a UI Helper Triggers subscriber (the order of the events is not pinned)",
            );
        }
        let names: Vec<String> = ["Language", "CaptionExpr", "Translation"]
            .iter()
            .map(|n| n.to_string())
            .collect();
        let mut args = vec![
            Value::Integer(language),
            Value::Text(text.to_string().into()),
            Value::Text(before.clone().into()),
        ];
        let call = crate::dispatch::EventCall {
            object: crate::events::UI_HELPER_TRIGGERS,
            event: "captionclasstranslate",
            element: None,
            isolated: false,
            names: &names,
            sender: None,
        };
        // The platform declares `CaptionExpr: Text[1024]` and `var Translation: Text[1024]` (PtFixtures.al:174-175).
        self.events.publisher_var_lens = vec![None, None, Some(1024)];
        // Microsoft's own subscribers of the event are not run: no case pins what they do.
        self.events.skip_dependency_subscribers = true;
        let raised = self.raise(&call, &mut args);
        self.events.skip_dependency_subscribers = false;
        self.events.publisher_var_lens.clear();
        raised?;
        match &args[2] {
            Value::Text(t) | Value::Code(t) if **t == *before => {}
            _ => {
                return unsupported(
                    "a UI Helper Triggers subscriber changed Translation of CaptionClassTranslate (its effect is not pinned)",
                );
            }
        }
        self.ui_caption_translated.push((text.to_string(), language));
        Ok(caption)
    }
}
