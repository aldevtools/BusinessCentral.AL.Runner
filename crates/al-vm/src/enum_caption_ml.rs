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

    /// Notes the language a filter is set under (`SetRange`, `SetFilter`, `SetView`, `SetRecFilter`, `CopyFilter(s)`) on a
    /// table with a CaptionML enum field, for [`Session::enum_ml_filter_refusal`].
    pub(crate) fn note_enum_ml_filter(
        &self,
        method: al_builtins::record_op::RecordOp,
        rec: &std::rc::Rc<std::cell::RefCell<al_builtins::value::RecordState>>,
    ) {
        use al_builtins::record_op::RecordOp::*;
        if !matches!(
            method,
            SetRange | SetFilter | SetView | SetRecFilter | CopyFilter | CopyFilters
        ) {
            return;
        }
        let objects = &self.engine.program.objects;
        let Some(t) = objects.get(rec.borrow().table as usize).and_then(|o| o.table.as_ref()) else {
            return;
        };
        let ml = |ty: &RtType| matches!(ty, RtType::Enum(id) if objects.get(*id as usize).is_some_and(|o| !o.enum_captions_ml.is_empty()));
        if t.fields.iter().any(|f| ml(&f.ty)) {
            let language = al_builtins::format::FormatCtx::language(self);
            let mut langs = self.enum_ml_filter_langs.1.borrow_mut();
            if !langs.contains(&language) {
                langs.push(language);
            }
        }
    }

    /// The refusal of a filter text (GetFilter, GetFilters, GetView) showing the CaptionML caption of enum value `v` when the
    /// filter may have been set under another GlobalLanguage than the current one: the corpus pins only a filter set and read
    /// under the same language (format-evaluate/EnumCaptionReaders.al:573-609, ECR_FilterTexts_1033 and _2067). Whether BC
    /// shows the caption of the language the filter was set under or of the current one is not pinned (hard rule 4). Without
    /// a language change since the test method started, the filter was set under the current language.
    pub(crate) fn enum_ml_filter_refusal(&self, v: &Value) -> Option<String> {
        let Value::Enum(id, ordinal) = v else {
            return None;
        };
        let (changed, langs) = &self.enum_ml_filter_langs;
        let language = al_builtins::format::FormatCtx::language(self);
        let langs = langs.borrow();
        if !*changed || (!langs.is_empty() && langs.iter().all(|l| *l == language)) {
            return None;
        }
        self.enum_ml_unpinned(
            *id,
            Some(*ordinal),
            "a filter set under another GlobalLanguage than the one GetFilter runs under",
        )
    }
}
