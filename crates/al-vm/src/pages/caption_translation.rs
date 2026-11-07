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
    /// The translated caption of control `idx`, for a control of the app's own page (not of a page extension, and the page
    /// itself not extended):
    /// - a control with its own `Caption` (no `CaptionClass` on it) reads the unit `Page <page> - Control <control> - Property
    ///   Caption` of the app's file, the source `Caption` without one;
    /// - a field control with no `Caption` of its own takes the caption of its source field, translated as `FieldCaption`
    ///   does (`field_caption_translation`).
    ///
    /// localization/app-xliff/lt-r7144-ext-caption/R7187PageCaption.al:56-72 (own `Caption` 'Row id' with a unit: `Rij-id`;
    /// the field control of "LT R7144 Row"."No.": `Rijnummer`, the field's unit) under 1043 with an nl-NL file. Any other
    /// control stays `Legacy` (a control with no caption, a page extension's control, a control over a CaptionClass).
    pub(super) fn control_caption_translation(&mut self, st: &St, idx: usize) -> Result<ControlCaption, Signal> {
        if al_builtins::format::FormatCtx::language(self) == 1033 {
            return Ok(ControlCaption::Legacy);
        }
        let engine = self.engine();
        let page = st.borrow().object as usize;
        let po = &engine.program.objects[page];
        let ctl = &page_of(&engine, st).controls[idx];
        if po.kind != ir::ObjKind::Page || ctl.owner.is_some() || ctl.prop("captionclass").is_some() {
            return Ok(ControlCaption::Legacy);
        }
        let own_caption = matches!(ctl.prop("caption"), Some(ir::PropVal::Lit(_)));
        if ctl.prop("caption").is_some() && !own_caption {
            return Ok(ControlCaption::Legacy);
        }
        if own_caption {
            if self.page_extended(page) {
                return Ok(ControlCaption::Legacy);
            }
            let xliff = format!(
                "{} - {} - {}",
                ir::xliff_part("Page", &po.name),
                ir::xliff_part("Control", &ctl.name),
                ir::xliff_part("Property", "Caption")
            );
            let what = || format!("the caption of control {} of page {}", ctl.name, po.name);
            return match self.app_translation(po.app as usize, &xliff, &what) {
                Ok(Some(t)) => Ok(ControlCaption::Text(t)),
                Ok(None) => Ok(ControlCaption::Source),
                Err(why) => unsupported(why),
            };
        }
        // No Caption of its own: the source field's.
        let ir::Binding::Field(pos) = ctl.binding else {
            return Ok(ControlCaption::Legacy);
        };
        if !self.has_source(st) || self.page_extended(page) {
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
}
