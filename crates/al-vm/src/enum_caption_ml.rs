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
}
