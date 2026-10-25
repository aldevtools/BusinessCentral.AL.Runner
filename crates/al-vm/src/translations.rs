//! Reads of a text a translation file could change (ADR-053 step 1, al-runner issue 5772).
//!
//! An app that ships a translation file (`Translations/*.xlf`, not the generated `*.g.xlf`; `al_ir::AppInfo::translations`)
//! has its labels, captions, option captions and enum value captions translated by BC in the session's language: after
//! `GlobalLanguage(2067)` real BC 28.1 answers `Vrijgegeven` for `Format` of an enum value with `Caption = 'Released'`,
//! accepts it in `Evaluate`, and answers `Hallo wereld` for `Label 'Hello world'` (issue 5772, three tests that pass on
//! BC). The runtime does not apply translations, so in a language other than 1033 such a read is refused instead of
//! answering the source text. Which file applies (exact culture, a fallback, an `en-US` file under 1033) is not pinned
//! (ADR-053 probes P1 to P4), so any translation file of the app refuses, whatever its language.
//!
//! An app without a translation file keeps the source text in every language, as the corpus pins under 1031 for its own
//! apps, which ship none (record/lt-enum-filter-caption/EnumFilterByCaption.al, pages/lt-page-controls-aa/AaFieldProps.al).

use super::*;

impl Session {
    /// Why a read of a translatable text of object `object` (a table, field, enum value, page or control caption, an
    /// OptionCaption) is refused: the language in force is not 1033 and the object's app, or the app of an extension of
    /// it, ships a translation file. `what` names the text read.
    pub(crate) fn translated_read_refusal(&self, object: usize, what: impl FnOnce() -> String) -> Option<String> {
        let language = al_builtins::format::FormatCtx::language(self);
        if language == 1033 {
            return None;
        }
        let p = &self.engine.program;
        if !p.apps.iter().any(|a| a.translations) {
            return None;
        }
        let translated = |o: &al_ir::Object| p.apps.get(o.app as usize).is_some_and(|a| a.translations);
        let base = p.objects.get(object)?;
        let hit = translated(base)
            || (base.id.is_some()
                && p.objects
                    .iter()
                    .any(|o| o.kind.extended() == Some(base.kind) && o.extends_id == base.id && translated(o)));
        hit.then(|| refusal(&what(), language))
    }

    /// The refusal of a read of the caption of field `pos` of table `t` that declares a `Caption` or `CaptionML`.
    pub(crate) fn field_caption_translated(&self, t: &al_data::TableDef, pos: usize) -> Option<String> {
        if al_builtins::format::FormatCtx::language(self) == 1033
            || (self.field_property(t, pos, "Caption").is_none() && self.field_property(t, pos, "CaptionML").is_none())
        {
            return None;
        }
        self.translated_read_refusal(t.object as usize, || {
            format!("the caption of field {} of table {}", t.fields[pos].name, t.name)
        })
    }

    /// [`Self::translated_read_refusal`] for a text of the app with id `app` (a Label of its code), as a signal.
    pub(crate) fn translated_app_read(&self, app: &str, what: &str) -> Result<(), Signal> {
        let language = al_builtins::format::FormatCtx::language(self);
        if language == 1033 {
            return Ok(());
        }
        let shipped = self
            .engine
            .program
            .apps
            .iter()
            .any(|a| a.translations && a.id.eq_ignore_ascii_case(app));
        if shipped {
            return Err(Signal::Unsupported(refusal(what, language)));
        }
        Ok(())
    }

    /// The refusal of a read of the captions of enum `id` (`ordinal`: of that value only; `None`: of any value), when the
    /// enum declares a caption the read can see.
    pub(crate) fn enum_caption_refusal(&self, id: u32, ordinal: Option<i32>) -> Option<String> {
        let o = self.engine.program.objects.get(id as usize)?;
        let captioned = o
            .enum_captions
            .iter()
            .any(|(v, c)| !c.is_empty() && ordinal.is_none_or(|w| w == *v));
        if !captioned {
            return None;
        }
        self.translated_read_refusal(id as usize, || match ordinal {
            Some(w) => {
                let name = o
                    .enum_values
                    .iter()
                    .find(|(v, _)| *v == w)
                    .map_or_else(|| w.to_string(), |(_, n)| n.clone());
                format!("the caption of value {name} of enum {}", o.name)
            }
            None => format!("the captions of enum {}", o.name),
        })
    }
}

fn refusal(what: &str, language: i32) -> String {
    format!("translations are not supported: {what} under GlobalLanguage {language} (the app ships a translation file)")
}
