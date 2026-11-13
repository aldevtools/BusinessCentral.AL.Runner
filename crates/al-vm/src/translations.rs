//! Reads of a text a translation file could change (ADR-053 step 1, al-runner issue 5772).
//!
//! An app that ships a translation file (`Translations/*.xlf`, not the generated `*.g.xlf`; `al_ir::AppInfo::translations`)
//! has its labels, captions, option captions and enum value captions translated by BC in the session's language: after
//! `GlobalLanguage(2067)` real BC 28.1 answers `Vrijgegeven` for `Format` of an enum value with `Caption = 'Released'`,
//! accepts it in `Evaluate`, and answers `Hallo wereld` for `Label 'Hello world'` (issue 5772, three tests that pass on
//! BC). The runtime does not apply translations, so in a language other than 1033 such a read is refused instead of
//! answering the source text. Which file applies (exact culture, a fallback, an `en-US` file under 1033) is not pinned
//! (ADR-053 probes P1 to P4), so a translation file of the language in force refuses, whatever its culture; files of other
//! languages only keep the source texts (`translates_into`).
//!
//! An app without a translation file keeps the source text in every language, as the corpus pins under 1031 for its own
//! apps, which ship none (record/filters/EnumFilterByCaption.al, pages/controls/AaFieldProps.al).

use super::*;

/// Whether app `a` may translate a text in `language`: it ships a translation file of that language (any culture), or one
/// whose language is not known (a package, whose files are not read; a file that could not be read; a language without a
/// known culture). An app whose files are all of other languages keeps its source texts, as an app without a file does:
/// localization/app-xliff/TaXliff.al:230 TA_Captions_Language_Without_File (nl-NL file, 1031: field caption, table
/// caption and option caption keep the source) and the corpus cases that read captions, CaptionML, option captions and
/// control captions under 1031 in the corpus app that ships the nl-NL file (for example misc/lt-probes-needed-5/
/// PnE10Formats.al OptionField_CaptionDiffersFromMember_German, testpage/ApTestPageTests.al Ap17_NotBlank_List).
fn translates_into(a: &al_ir::AppInfo, language: i32) -> bool {
    if !a.translations {
        return false;
    }
    let heads = a.translation_heads();
    if heads.is_empty() {
        return true;
    }
    let Some(culture) = crate::platform::culture_name(language) else {
        return true;
    };
    heads
        .iter()
        .any(|f| f.error.is_some() || family(&f.target_language) == family(culture))
}

/// The cultures whose file is pinned to give the units of another culture of its language that has no file of its own: the
/// nl-NL file under nl-BE (localization/app-xliff/TaXliff.al:146, :176). Other languages wait for corpus probe
/// TlTranslationCells.al (fr-FR under fr-BE, fr-CA and fr-CH; en-GB under en-AU) and stay refused.
const PINNED_FALLBACKS: &[&str] = &["nl-NL"];

/// The language of culture `c` (`nl` for `nl-BE`), lowercase.
fn family(c: &str) -> String {
    c.split('-').next().unwrap_or(c).to_ascii_lowercase()
}

/// Every translation file of app `a` with its units (a package's are read here).
fn read_files(a: &al_ir::AppInfo) -> impl Iterator<Item = &al_ir::TranslationFile> {
    (0..a.translation_heads().len()).map(|i| a.translation_file(i))
}

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
        if !p.apps.iter().any(|a| translates_into(a, language)) {
            return None;
        }
        let translated = |o: &al_ir::Object| p.apps.get(o.app as usize).is_some_and(|a| translates_into(a, language));
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
            .any(|a| translates_into(a, language) && a.id.eq_ignore_ascii_case(app));
        if shipped {
            return Err(Signal::unsupported(refusal(what, language)));
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
    /// unit without one gives the source; a language with no file of its own language keeps the source too
    /// (localization/app-xliff/TaXliff.al: TA_Enum_Format_And_Evaluate, TA_Label_Text and TA_Captions_And_Options under
    /// 1043, TA_Captions_Language_Without_File under 1031; al-runner issue 5772). With no file of the culture and exactly one
    /// of the same language for another culture that has the unit, that file gives the target where it is pinned
    /// ([`PINNED_FALLBACKS`]: the nl-NL file under 2067). The files come from an app folder or, read on the first ask, from a package (`al_ir::AppInfo::translation_heads`).
    /// Refused: an app whose files cannot be listed (a wrapper package); two files for one culture; a file that could not be
    /// read; a unit only another culture's file has while the culture's own file exists, or that several files of other
    /// cultures have (which one BC uses is not pinned: corpus probe TlTranslationCells.al, ADR-053 P3). A unit's state is
    /// read by [`unit_text`].
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
        let heads = a.translation_heads();
        if heads.is_empty() {
            return refuse("the app ships a translation file");
        }
        let Some(culture) = crate::platform::culture_name(language) else {
            return refuse("the language's culture is not known");
        };
        let pick = |own: bool| -> Vec<usize> {
            (0..heads.len())
                .filter(|&i| {
                    let f = &heads[i];
                    if own {
                        f.error.is_some() || f.target_language.eq_ignore_ascii_case(culture)
                    } else {
                        f.error.is_none()
                            && !f.target_language.eq_ignore_ascii_case(culture)
                            && family(&f.target_language) == family(culture)
                    }
                })
                .collect()
        };
        let (own, others) = (pick(true), pick(false));
        // A file of the same language for another culture matters only when it has the unit: without one the source text
        // stays whichever file BC picks (format-evaluate/EvalLanguageBoolean.al EL_Enum_NLB_2067: the enum has no unit in the
        // app's nl-NL file; read under nl-BE).
        let mut others_with_unit = Vec::new();
        let mut fallback = true;
        for &i in &others {
            let f = a.translation_file(i);
            if let Some(e) = &f.error {
                return refuse(&format!("the translation file {} could not be read: {e}", f.name));
            }
            if let Some(unit) = f.unit(xliff) {
                others_with_unit.push(unit);
                fallback &= PINNED_FALLBACKS
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(&f.target_language));
            }
        }
        match (own.as_slice(), others_with_unit.as_slice()) {
            ([_, _, ..], _) => refuse(&format!(
                "two translation files for {culture}, or one that could not be read"
            )),
            ([i], others) => {
                let file = a.translation_file(*i);
                if let Some(e) = &file.error {
                    return refuse(&format!("the translation file {} could not be read: {e}", file.name));
                }
                let unit = file.unit(xliff);
                if !others.is_empty() {
                    // The culture's own file decides whether or not it has the unit; without one the source text stays
                    // (assumption `xliff-exact-culture-over-sibling`, as the runtime did before).
                    al_builtins::assumptions::assume(
                        al_builtins::assumptions::Assumption::XliffExactCultureOverSibling,
                    );
                }
                unit_text(unit)
            }
            ([], []) => Ok(None),
            ([], [unit]) if fallback => unit_text(Some(*unit)),
            ([], _) => refuse(&format!(
                "the app ships a translation file of the same language for another culture than {culture} with this unit"
            )),
        }
    }

    /// Whether an extension of object `object` exists: the texts it adds or changes (`modify`) are the extension's units, or
    /// texts of an app with no file, which are not looked up; such reads keep the step 1 refusal.
    pub(crate) fn page_extended(&self, object: usize) -> bool {
        self.extended(object)
    }

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
        if o.kind == al_ir::ObjKind::Enum && self.only_foreign_translating_extensions_enum(id as usize) {
            return Ok(None);
        }
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
            .is_none_or(|a| a.translation_heads().is_empty());
        if o.kind == al_ir::ObjKind::Enum && self.only_foreign_translating_extensions_enum(id as usize) {
            return Ok(None);
        }
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
        if let Some(ext) = self.declaring_extension(t, pos) {
            return self.extension_field_caption_translation(t, pos, ext, why);
        }
        if o.kind == al_ir::ObjKind::Table && self.only_foreign_translating_extensions(object) {
            return Ok(None);
        }
        // An extension that does not `modify` the caption leaves the table's unit in force
        // (localization/app-xliff/lt-r7144-ext-caption/R7187PageCaption.al:56-72: "LT R7144 Row"."No." is read as `Rijnummer`
        // while an extension of the same app adds a field); one that does is not pinned.
        let modified = o
            .table
            .as_ref()
            .and_then(|tb| tb.fields.get(pos))
            .is_some_and(|f| f.caption_modified);
        if o.kind != al_ir::ObjKind::Table
            || modified
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

    /// Whether object `object` (a table, enum or page of an app that does not translate into the language in force, such as
    /// Base Application) is extended by apps that do translate into it, and none of those extensions can change what a
    /// caption read of it gives: BC then leaves the object's captions alone, `modify`d ones included. Pinned for the table
    /// Item and the enum Sales Document Type, extended by an app with an nl-NL file under 1043: fields the extension does not
    /// touch, fields it modifies with a `Caption` (with a unit of the extension's own id, of the original table's id, or of
    /// the wrong kind in the file, all ignored), and enum values (localization/app-xliff/lt-r7144-ext-caption/
    /// R7184BaseAppCaption.al:91-122, :145-165 and Sales Line in R7187PageCaption.al, al-runner issue 7184). For a page it
    /// decides the controls the extensions leave alone (`control_caption_translation`, which reads the extensions' own units
    /// for the captions they set): Base Application's Payment Methods and Territories, extended by an app with an nl-NL file
    /// under 1043 (localization/app-xliff/BaseAppPageExtCaption.al:205-219, :240-251, al-runner issue 7327). Not decided here
    /// (stays refused): an extension app whose files are not read (a package, a file that could not be read) or are of the
    /// same language for another culture.
    pub(crate) fn only_foreign_translating_extensions(&self, object: usize) -> bool {
        let p = &self.engine.program;
        let language = al_builtins::format::FormatCtx::language(self);
        let Some(base) = p.objects.get(object) else {
            return false;
        };
        let Some(culture) = crate::platform::culture_name(language) else {
            return false;
        };
        if base.id.is_none()
            || p.apps
                .get(base.app as usize)
                .is_none_or(|a| translates_into(a, language))
        {
            return false;
        }
        p.objects
            .iter()
            .filter(|o| o.kind.extended() == Some(base.kind) && o.extends_id == base.id)
            .all(|o| {
                let Some(a) = p.apps.get(o.app as usize) else {
                    return false;
                };
                !translates_into(a, language)
                    || (!a.translation_heads().is_empty()
                        && a.translation_heads().iter().all(|f| {
                            f.error.is_none()
                                && (family(&f.target_language) != family(culture)
                                    || f.target_language.eq_ignore_ascii_case(culture))
                        }))
            })
    }

    /// [`Self::only_foreign_translating_extensions`] for an enum. Values an enum extension adds read their source caption
    /// when no unit of the file names an enum extension (none does in R7184BaseAppCaption.al); a file with such a unit keeps
    /// the refusal, since which unit BC reads for an extension's value is not pinned.
    fn only_foreign_translating_extensions_enum(&self, id: usize) -> bool {
        if !self.only_foreign_translating_extensions(id) {
            return false;
        }
        let p = &self.engine.program;
        let Some(base) = p.objects.get(id) else {
            return false;
        };
        !p.objects
            .iter()
            .filter(|o| o.kind.extended() == Some(base.kind) && o.extends_id == base.id)
            .filter_map(|o| p.apps.get(o.app as usize))
            .flat_map(read_files)
            .any(|f| f.units.iter().any(|(k, _)| k.starts_with("EnumExtension ")))
    }

    /// The table extension that declares field `pos` of `t`, when one does (`al_ir::Field::declared_in`).
    fn declaring_extension(&self, t: &al_data::TableDef, pos: usize) -> Option<usize> {
        let objects = &self.engine.program.objects;
        let declared = objects
            .get(t.object as usize)?
            .table
            .as_ref()?
            .fields
            .get(pos)?
            .declared_in? as usize;
        (declared != t.object as usize && objects.get(declared)?.kind == al_ir::ObjKind::TableExtension)
            .then_some(declared)
    }

    /// The caption of field `pos` of `t` that table extension `ext` declares, from the translation file of the extension's app:
    /// the unit `TableExtension <ext> - Field <field> - Property Caption`, as for a field of the app's own table
    /// (localization/app-xliff/lt-r7144-ext-caption/R7144ExtFieldCaption.al:68-145, corpus PR 1156, al-runner issue 7144: a table extension of
    /// Base Application's Sales Invoice Header, read with FieldCaption and FieldRef.Caption under 1043; the source caption
    /// under 1031, which has no file, and 1033). An extension of a table of its own app keeps the source caption (below).
    /// Refused (`why`): a field with `CaptionML` or `CaptionClass`, and a table another extension in an app that translates
    /// into the language extends (it could `modify` the caption; not pinned).
    fn extension_field_caption_translation(
        &self,
        t: &al_data::TableDef,
        pos: usize,
        ext: usize,
        why: String,
    ) -> Result<Option<String>, String> {
        let p = &self.engine.program;
        let e = &p.objects[ext];
        let language = al_builtins::format::FormatCtx::language(self);
        let other_translating_extension = p.objects.iter().enumerate().any(|(i, o)| {
            i != ext
                && o.kind == al_ir::ObjKind::TableExtension
                && o.extends_id == e.extends_id
                && p.apps.get(o.app as usize).is_some_and(|a| translates_into(a, language))
        });
        if self.field_property(t, pos, "CaptionML").is_some() || self.field_property(t, pos, "CaptionClass").is_some() {
            return Err(why);
        }
        // An extension of a table of its own app keeps the source caption although its TableExtension unit is in the file
        // (R7144ExtFieldCaption.al:83, :103: 'Extension code' under 1043). A unit under the table's own id for the field
        // (`Table <table> - Field <field>`, which the compiler does not write for it) is not pinned: refused.
        if let Some(base) = p.objects.get(t.object as usize).filter(|b| b.app == e.app) {
            if other_translating_extension {
                return Err(why);
            }
            let table_unit = format!(
                "{} - {} - {}",
                al_ir::xliff_part("Table", &base.name),
                al_ir::xliff_part("Field", &t.fields[pos].name),
                al_ir::xliff_part("Property", "Caption")
            );
            let what = || format!("the caption of field {} of table {}", t.fields[pos].name, t.name);
            return match self.app_translation(e.app as usize, &table_unit, &what)? {
                Some(_) => Err(why),
                None => Ok(None),
            };
        }
        let xliff = format!(
            "{} - {} - {}",
            al_ir::xliff_part("TableExtension", &e.name),
            al_ir::xliff_part("Field", &t.fields[pos].name),
            al_ir::xliff_part("Property", "Caption")
        );
        let unit = self.app_translation(e.app as usize, &xliff, &|| {
            format!("the caption of field {} of table {}", t.fields[pos].name, t.name)
        })?;
        // Another extension of an app that translates into the language could `modify` the caption (not pinned for a
        // field an extension declares). With no unit for the field in the file the source caption is read either way
        // (localization/app-xliff/lt-r7144-ext-caption/R7184BaseAppCaption.al:98-100, :145 and R7187PageCaption.al:219,
        // al-runner issue 7184: extension fields of Item and Sales Line, no unit); with a unit it stays refused.
        if other_translating_extension && unit.is_some() {
            return Err(why);
        }
        Ok(unit)
    }

    /// The caption of table `object` in the language in force, for a read step 1 refuses with `why`
    /// (`Session::caption_refused`): `Ok(None)` for the source caption. The unit `Table <table> - Property Caption` of the
    /// table's app gives it (localization/app-xliff/TaXliff.al:216 TableCaption and :223 RecordRef.Caption under 1043, the
    /// source under 1031 at :242). Refused: a table with `CaptionML`, a table an extension extends, and a caption step 1
    /// refuses for another reason (`al_ir::CAPTION_REFUSED`).
    pub(crate) fn table_caption_translation(&self, object: usize, why: String) -> Result<Option<String>, String> {
        let Some(o) = self.engine.program.objects.get(object) else {
            return Err(why);
        };
        let caption = |k: &str| o.properties.iter().any(|(p, _)| p.eq_ignore_ascii_case(k));
        if o.kind != al_ir::ObjKind::Table
            || !caption("Caption")
            || caption("CaptionML")
            || o.properties
                .iter()
                .any(|(k, _)| k == al_ir::CAPTION_REFUSED || k == al_ir::EXT_CAPTION_OVER_OWN)
            || self.extended(object)
        {
            return Err(why);
        }
        let xliff = format!(
            "{} - {}",
            al_ir::xliff_part("Table", &o.name),
            al_ir::xliff_part("Property", "Caption")
        );
        self.app_translation(o.app as usize, &xliff, &|| format!("the caption of table {}", o.name))
    }

    /// The OptionCaption of field `pos` of `t` in the language in force, for a read step 1 refuses with `why`: `Ok(None)` for
    /// the source captions. The unit `Table <table> - Field <field> - Property OptionCaption` of the table's app gives the list
    /// (localization/app-xliff/TaXliff.al:222 FieldRef.OptionCaption `Eerste,Tweede,Derde` under 1043; :243 the source under
    /// 1031). Refused when a unit exists: a field with OptionCaptionML, a field a table extension declares (unit `TableExtension
    /// <ext> - Field <field> - Property OptionCaption` of the extension's app), and a field of a table an extension extends.
    pub(crate) fn option_caption_translation(
        &self,
        t: &al_data::TableDef,
        pos: usize,
        why: String,
    ) -> Result<Option<String>, String> {
        let object = t.object as usize;
        let Some(o) = self.engine.program.objects.get(object) else {
            return Err(why);
        };
        if o.kind != al_ir::ObjKind::Table {
            return Err(why);
        }
        // The unit is looked up first: a field without one keeps its own text (a value without a unit keeps its source,
        // TaXliff.al:138), whether it is Locked, has OptionCaptionML, or its table is extended.
        let declaring = self.declaring_extension(t, pos);
        let (app, kind, owner) = match declaring.and_then(|e| self.engine.program.objects.get(e)) {
            Some(e) => (e.app, "TableExtension", &e.name),
            None => (o.app, "Table", &o.name),
        };
        let xliff = format!(
            "{} - {} - {}",
            al_ir::xliff_part(kind, owner),
            al_ir::xliff_part("Field", &t.fields[pos].name),
            al_ir::xliff_part("Property", "OptionCaption")
        );
        let unit = self.app_translation(app as usize, &xliff, &|| {
            format!("the OptionCaption of field {} of table {}", t.fields[pos].name, t.name)
        })?;
        // With a unit, not pinned: a field a table extension declares, a field of a table an extension extends, and a field
        // with OptionCaptionML (which keeps its own text of the language without a unit, format-evaluate/
        // EvalLanguageBoolean.al EL_Option_NLB_2067, EL_Option_NLD_1043).
        if unit.is_some()
            && (declaring.is_some()
                || self.extended(object)
                || self.field_property(t, pos, "OptionCaptionML").is_some())
        {
            return Err(why);
        }
        Ok(unit)
    }

    /// [`Self::option_caption_translation`] of field number `field` of table number `table`, for Format and Evaluate: `None`
    /// keeps the source captions (1033, an app without a file of the language).
    pub(crate) fn translated_option_caption(&self, table: i32, field: i32) -> Result<Option<String>, Signal> {
        let t = u32::try_from(table).ok().and_then(|n| self.table_def(n));
        let Some(t) = t else {
            return Err(Signal::unsupported(format!(
                "the OptionCaption of a field of table {table}: the table is not known"
            )));
        };
        let Some(pos) = t.fields.iter().position(|f| i64::from(f.id) == i64::from(field)) else {
            return Err(Signal::unsupported(format!(
                "the OptionCaption of field {field} of table {}: no such field",
                t.name
            )));
        };
        let Some(why) = self.translated_read_refusal(t.object as usize, || {
            format!("the OptionCaption of field {} of table {}", t.fields[pos].name, t.name)
        }) else {
            return Ok(None);
        };
        self.option_caption_translation(&t, pos, why)
            .map_err(Signal::unsupported)
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
            .map_err(Signal::unsupported)
    }
}

/// The text unit `unit` gives (`None`: no unit, the source text stays). A unit without a state gives its target
/// (TaXliff.al:170); one with any `state` gives it too, as ADR-053 decision 1 says, flagged until the corpus probe
/// TlTranslationCells.al TL_UnitForms_Dutch pins the states (assumption `xliff-unit-state`).
fn unit_text(unit: Option<(&str, &al_ir::UnitState)>) -> Result<Option<String>, String> {
    use al_builtins::assumptions::{Assumption, assume};
    Ok(unit.map(|(text, state)| {
        if *state != al_ir::UnitState::Absent {
            assume(Assumption::XliffUnitState);
        }
        text.to_string()
    }))
}

fn refusal(what: &str, language: i32) -> String {
    format!("translations are not supported: {what} under GlobalLanguage {language} (the app ships a translation file)")
}
