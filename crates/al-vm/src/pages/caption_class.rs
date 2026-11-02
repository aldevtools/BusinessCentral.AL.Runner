//! The caption a CaptionClass text stands for (moved out of rows.rs).

use super::*;

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
        // In a company the DemoTool built, General Ledger Setup is the row BC's own build writes (the DemoTool runs the same
        // code on BC), so the premise of this refusal does not hold there (`Baseline::company_built`).
        if self.engine.test_data.is_none()
            && !self.company_built
            && matches!(area.trim(), "1" | "101")
            && caption != text
            && !written
        {
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
}
