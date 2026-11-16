//! `Caption()` of a TestPage control under a language other than 1033, when the page's app ships a translation file
//! (ADR-053 step 2, al-runner issue 7187).

use super::*;

/// What a control's caption read does under the language in force.
pub(super) enum ControlCaption {
    /// Not decided here: the caller keeps its refusal check and its other rules.
    Legacy,
    /// The caption a translation file gives.
    Text(String),
    /// The source caption stands: no unit gives another one.
    Source,
}

impl Session {
    /// The translated caption of control `idx`, read from the app that wrote it:
    /// - a control with its own `Caption` (no `CaptionClass` on it or on its source field) reads that app's unit, the source
    ///   `Caption` without one: `Page <page> - Control <control> - Property Caption` of the page's app for a control of a page
    ///   no extension extends, `PageExtension <ext> - Control <control> - Property Caption` for a control an extension adds,
    ///   `PageExtension <ext> - Change <control> - Property Caption` for a caption one extension's `modify` sets;
    /// - a field control with no `Caption` of its own takes the caption of its source field, translated as `FieldCaption`
    ///   does (`field_caption_translation`).
    ///
    /// A page an extension extends is decided here only when its app does not translate into the language and no
    /// extension's file leaves the culture in doubt (`only_foreign_translating_extensions`).
    ///
    /// localization/app-xliff/lt-r7144-ext-caption/R7187PageCaption.al:56-72 (own `Caption` 'Row id' with a unit: `Rij-id`;
    /// the field control of "LT R7144 Row"."No.": `Rijnummer`, the field's unit) under 1043 with an nl-NL file.
    /// localization/app-xliff/BaseAppPageExtCaption.al:205-219 (Base Application's Payment Methods under 1043, extended by an
    /// app with an nl-NL file, al-runner issue 7327): its field controls keep the Base Application captions, a `modify`
    /// caption with a unit reads it (`LPX type gewijzigd`) and one without keeps its source (`LPX No Modified`), an added
    /// control's caption likewise (`LPX toegevoegd`, `LPX Added No Unit`), an added field control without one takes the
    /// field's (`Direct Debit`); :240-251, a page an extension extends with an action only keeps its captions.
    /// Any other control stays `Legacy`: a control with no caption, a `CaptionClass` or a `CaptionML`, the own caption of a
    /// page an extension extends, a caption two extensions set or one extension sets on another one's control.
    pub(super) fn control_caption_translation(&mut self, st: &St, idx: usize) -> Result<ControlCaption, Signal> {
        if al_builtins::format::FormatCtx::language(self) == 1033 {
            return Ok(ControlCaption::Legacy);
        }
        let engine = self.engine();
        let page = st.borrow().object as usize;
        let po = &engine.program.objects[page];
        let ctl = &page_of(&engine, st).controls[idx];
        if po.kind != ir::ObjKind::Page || ctl.prop("captionclass").is_some() {
            return Ok(ControlCaption::Legacy);
        }
        let extended = self.page_extended(page);
        if extended && (ctl.prop("captionml").is_some() || !self.only_foreign_translating_extensions(page)) {
            return Ok(ControlCaption::Legacy);
        }
        match ctl.prop("caption") {
            Some(ir::PropVal::Lit(_)) => return self.own_caption_translation(st, idx, extended),
            Some(_) => return Ok(ControlCaption::Legacy),
            None => {}
        }
        // No Caption of its own: the source field's.
        let ir::Binding::Field(pos) = ctl.binding else {
            return Ok(ControlCaption::Legacy);
        };
        if !self.has_source(st) {
            return Ok(ControlCaption::Legacy);
        }
        let table = self.page_rec(st).borrow().table as usize;
        let Some(table_id) = engine.program.objects[table].id else {
            return Ok(ControlCaption::Legacy);
        };
        let Some(def) = self.table_def(table_id) else {
            return Ok(ControlCaption::Legacy);
        };
        if self.field_property(&def, pos, "Caption").is_none() {
            return Ok(ControlCaption::Legacy);
        }
        match self.field_caption_translation(&def, pos) {
            Ok(Some(t)) => Ok(ControlCaption::Text(t)),
            Ok(None) => Ok(ControlCaption::Source),
            Err(why) => unsupported(why),
        }
    }

    /// [`Self::control_caption_translation`] of a control with its own literal `Caption`, on a page an extension extends
    /// (`extended`) or not.
    fn own_caption_translation(&mut self, st: &St, idx: usize, extended: bool) -> Result<ControlCaption, Signal> {
        let engine = self.engine();
        let objects = &engine.program.objects;
        let po = &objects[st.borrow().object as usize];
        let ctl = &page_of(&engine, st).controls[idx];
        let unit = |owner: &str, kind: &str, control: &str| {
            format!(
                "{owner} - {} - {}",
                ir::xliff_part(kind, control),
                ir::xliff_part("Property", "Caption")
            )
        };
        let (app, xliff) = match (ctl.owner, ctl.caption_modifies.as_slice()) {
            (None, []) if !extended => (po.app, unit(&ir::xliff_part("Page", &po.name), "Control", &ctl.name)),
            // The unit names the control as the `modify` writes it; a name written in another case is not pinned.
            (None, [(e, written)]) if *written == ctl.name => {
                let ext = &objects[*e];
                (
                    ext.app,
                    unit(&ir::xliff_part("PageExtension", &ext.name), "Change", written),
                )
            }
            (Some(e), []) => {
                let ext = &objects[e];
                (
                    ext.app,
                    unit(&ir::xliff_part("PageExtension", &ext.name), "Control", &ctl.name),
                )
            }
            _ => return Ok(ControlCaption::Legacy),
        };
        // Whether a control's Caption beats its source field's CaptionClass is not pinned; the source caption path refuses it
        // (`control_caption_plain`), with or without a unit.
        let over_class = self.control_field(st, idx).is_some_and(|f| {
            f.caption_class_refusal.is_some()
                || f.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case("captionclass"))
        });
        if over_class {
            return Ok(ControlCaption::Source);
        }
        let what = || format!("the caption of control {} of page {}", ctl.name, po.name);
        match self.app_translation(app as usize, &xliff, &what) {
            Ok(Some(t)) => Ok(ControlCaption::Text(t)),
            Ok(None) => Ok(ControlCaption::Source),
            Err(why) => unsupported(why),
        }
    }
}
