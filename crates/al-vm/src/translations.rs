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

/// ADR-053 step 2: the translation files of an app folder are applied.
impl Session {
    /// The text of XLIFF unit `xliff` of app `app` in the language in force: `Ok(None)` keeps the source text, `Ok(Some)` is
    /// the unit's target, `Err` the reason the read is refused. In 1033, or for an app that ships no translation file, the
    /// source text stays (as in step 1). Otherwise the app's file for exactly the language's culture gives the target, and a
    /// unit without one gives the source; a language with no file of its own language keeps the source too. Both rest on
    /// the reporter's real-BC run of al-runner issue 5772 (assumption `app-translation-applied`). Refused: a package, whose
    /// files are not read; two files for one culture; a file that could not be read; and a file of the same language but
    /// another culture (nl-NL under nl-BE: which one BC uses is not pinned, ADR-053 P3).
    pub(crate) fn app_translation(
        &self,
        app: usize,
        xliff: &str,
        what: &dyn Fn() -> String,
    ) -> Result<Option<String>, String> {
        let language = al_builtins::format::FormatCtx::language(self);
        if language == 1033 {
            return Ok(None);
        }
        let Some(a) = self.engine.program.apps.get(app) else {
            return Ok(None);
        };
        if !a.translations {
            return Ok(None);
        }
        let refuse = |why: &str| {
            Err(format!(
                "translations are not supported: {} under GlobalLanguage {language} ({why})",
                what()
            ))
        };
        if a.translation_files.is_empty() {
            return refuse("the app ships a translation file");
        }
        let Some(culture) = crate::platform::culture_name(language) else {
            return refuse("the language's culture is not known");
        };
        let mut files = a
            .translation_files
            .iter()
            .filter(|f| f.error.is_some() || f.target_language.eq_ignore_ascii_case(culture));
        match (files.next(), files.next()) {
            (Some(_), Some(_)) => refuse(&format!(
                "two translation files for {culture}, or one that could not be read"
            )),
            (Some(f), None) => {
                if let Some(e) = &f.error {
                    return refuse(&format!("the translation file {} could not be read: {e}", f.name));
                }
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::AppTranslationApplied);
                Ok(f.target(xliff).map(str::to_string))
            }
            (None, None) => {
                let family = |c: &str| c.split('-').next().unwrap_or(c).to_ascii_lowercase();
                if a.translation_files
                    .iter()
                    .any(|f| family(&f.target_language) == family(culture))
                {
                    return refuse(&format!(
                        "the app ships a translation file of the same language for another culture than {culture}"
                    ));
                }
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::AppTranslationApplied);
                Ok(None)
            }
            (None, Some(_)) => unreachable!(),
        }
    }

    /// Whether an extension of object `object` exists: the texts it adds or changes (`modify`) are the extension's units, or
    /// texts of an app with no file, which are not looked up; such reads keep the step 1 refusal.
    fn extended(&self, object: usize) -> bool {
        let p = &self.engine.program;
        let Some(base) = p.objects.get(object) else {
            return false;
        };
        base.id.is_some()
            && p.objects
                .iter()
                .any(|o| o.kind.extended() == Some(base.kind) && o.extends_id == base.id)
    }

    /// The caption of value `ordinal` of enum `id` in the language in force, when a translation file can give it: `Ok(None)`
    /// for the source caption. A value without a declared caption, an enum an extension in a translating app extends, or
    /// an enum with no XLIFF name keeps the step 1 refusal.
    pub(crate) fn enum_caption_translated(&self, id: u32, ordinal: i32) -> Result<Option<String>, String> {
        let Some(why) = self.enum_caption_refusal(id, Some(ordinal)) else {
            return Ok(None);
        };
        let o = &self.engine.program.objects[id as usize];
        let Some(name) = o
            .enum_values
            .iter()
            .find(|(v, _)| *v == ordinal)
            .map(|(_, n)| n.clone())
        else {
            return Err(why);
        };
        if o.kind != al_ir::ObjKind::Enum || self.extended(id as usize) {
            return Err(why);
        }
        let xliff = format!(
            "{} - {} - {}",
            al_ir::xliff_part("Enum", &o.name),
            al_ir::xliff_part("EnumValue", &name),
            al_ir::xliff_part("Property", "Caption")
        );
        self.app_translation(o.app as usize, &xliff, &|| {
            format!("the caption of value {name} of enum {}", o.name)
        })
    }

    /// Every caption of enum `id` in the language in force, for Evaluate: `Ok(None)` when they are the source captions.
    pub(crate) fn enum_captions_translated(&self, id: u32) -> Result<Option<Vec<(i32, String)>>, String> {
        let Some(why) = self.enum_caption_refusal(id, None) else {
            return Ok(None);
        };
        let o = &self.engine.program.objects[id as usize];
        let no_files = self
            .engine
            .program
            .apps
            .get(o.app as usize)
            .is_none_or(|a| a.translation_files.is_empty());
        if o.kind != al_ir::ObjKind::Enum || self.extended(id as usize) || no_files {
            return Err(why);
        }
        let mut changed = false;
        let mut out = Vec::with_capacity(o.enum_captions.len());
        for (v, c) in &o.enum_captions {
            match self.enum_caption_translated(id, *v)? {
                Some(t) => {
                    changed = true;
                    out.push((*v, t));
                }
                None => out.push((*v, c.clone())),
            }
        }
        Ok(changed.then_some(out))
    }

    /// The caption of field `pos` of table `t` in the language in force, when a translation file can give it: `Ok(None)` for
    /// the source caption. Only a field the table itself declares with `Caption` (no `CaptionML`, no `CaptionClass`) is
    /// looked up; the others keep the step 1 refusal.
    pub(crate) fn field_caption_translation(
        &self,
        t: &al_data::TableDef,
        pos: usize,
    ) -> Result<Option<String>, String> {
        let Some(why) = self.field_caption_translated(t, pos) else {
            return Ok(None);
        };
        let object = t.object as usize;
        let Some(o) = self.engine.program.objects.get(object) else {
            return Err(why);
        };
        if o.kind != al_ir::ObjKind::Table
            || self.extended(object)
            || self.field_property(t, pos, "CaptionML").is_some()
            || self.field_property(t, pos, "CaptionClass").is_some()
        {
            return Err(why);
        }
        let xliff = format!(
            "{} - {} - {}",
            al_ir::xliff_part("Table", &o.name),
            al_ir::xliff_part("Field", &t.fields[pos].name),
            al_ir::xliff_part("Property", "Caption")
        );
        self.app_translation(o.app as usize, &xliff, &|| {
            format!("the caption of field {} of table {}", t.fields[pos].name, t.name)
        })
    }

    /// A Label of app `app` whose XLIFF unit is `xliff`, in the language in force (`System.TranslatedText#1` with an id).
    pub(crate) fn translated_app_text(&self, app: &str, what: &str, xliff: &str) -> Result<Option<String>, Signal> {
        let Some(index) = self
            .engine
            .program
            .apps
            .iter()
            .position(|a| a.id.eq_ignore_ascii_case(app))
        else {
            return Ok(None);
        };
        self.app_translation(index, xliff, &|| what.to_string())
            .map_err(Signal::Unsupported)
    }
}

fn refusal(what: &str, language: i32) -> String {
    format!("translations are not supported: {what} under GlobalLanguage {language} (the app ships a translation file)")
}
