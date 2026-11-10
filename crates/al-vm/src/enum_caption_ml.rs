//! The `CaptionML` of enum values in the session language (al-runner issue 7195).
//!
//! `Format` and `Evaluate` of an enum read the caption of the session language: `CaptionML = ENU = 'Open cap', NLB = 'Open BE'`
//! gives `Open BE` under GlobalLanguage 2067 and `Open cap` under 1033 (format-evaluate/EvalLanguageBoolean.al EL_Enum_*, the
//! same on the three supported majors). A value that declares both a `Caption` and a `CaptionML`, and a language whose entry
//! is not pinned (`al_builtins::format::ml_text_in_language`), are refused.

use super::*;

impl Session {
    /// The caption of value `ordinal` (`None`: of every value with a `CaptionML`) of enum `id` in the session language, when
    /// the value declares a `CaptionML`: `Ok(None)` for no `CaptionML`, `Err` for the reason the read is refused.
    pub(crate) fn enum_ml_captions(&self, id: u32, ordinal: Option<i32>) -> Result<Vec<(i32, String)>, String> {
        let o = &self.engine.program.objects[id as usize];
        let language = al_builtins::format::FormatCtx::language(self);
        let mut out = Vec::new();
        for (v, ml) in &o.enum_captions_ml {
            if ordinal.is_some_and(|w| w != *v) {
                continue;
            }
            if o.enum_captions.iter().any(|(w, c)| w == v && !c.is_empty()) {
                return Err(format!(
                    "the caption of value {v} of enum {}: it declares a Caption and a CaptionML (not pinned)",
                    o.name
                ));
            }
            let text = al_builtins::format::ml_text_in_language(ml, language)
                .map_err(|e| format!("the CaptionML of enum {} {e}", o.name))?;
            out.push((*v, text));
        }
        Ok(out)
    }

    /// The refusal of a reader other than Format and Evaluate (a page control, a filter text, a key value, a metadata read)
    /// of the caption of value `ordinal` (`None`: of any value) of enum `id`, when a value declares a `CaptionML`. Only Format
    /// and Evaluate are pinned to read it (above); every other reader would show the name or the plain caption, a silent
    /// wrong result (hard rule 4).
    pub(crate) fn enum_ml_unpinned(&self, id: u32, ordinal: Option<i32>, reader: &str) -> Option<String> {
        let o = self.engine.program.objects.get(id as usize)?;
        let (v, _) = o
            .enum_captions_ml
            .iter()
            .find(|(v, _)| ordinal.is_none_or(|w| w == *v))?;
        Some(format!(
            "the caption of enum value {v} of enum {} under GlobalLanguage {} (CaptionML; not pinned for {reader})",
            o.name,
            al_builtins::format::FormatCtx::language(self)
        ))
    }

    /// Refuses reading a filter entry on a CaptionML enum field as text (GetFilter, GetFilters, GetView, GetRangeMin/Max,
    /// TestFilter.GetFilter) under another GlobalLanguage than the one it was set under (`FieldFilter::lang`). The corpus pins
    /// only a filter set and read under the same language (format-evaluate/EnumCaptionReaders.al:573-662, ECR_FilterTexts_1033
    /// and _2067, `GlobalLanguage(Lang)` before `SetRange`); which language's caption BC shows after a switch is not pinned
    /// (hard rule 4).
    pub(crate) fn enum_ml_filter_check(
        &self,
        t: &al_data::TableDef,
        f: &al_builtins::filter::FieldFilter,
    ) -> Result<(), Signal> {
        let RtType::Enum(id) = t.fields[f.field].ty else {
            return Ok(());
        };
        let language = al_builtins::format::FormatCtx::language(self);
        if f.lang == language {
            return Ok(());
        }
        match self.enum_ml_unpinned(
            id,
            None,
            "a filter set under another GlobalLanguage than the one it is read under",
        ) {
            Some(why) => Err(Signal::unsupported(format!(
                "{why}; the filter was set under GlobalLanguage {}",
                f.lang
            ))),
            None => Ok(()),
        }
    }
}
