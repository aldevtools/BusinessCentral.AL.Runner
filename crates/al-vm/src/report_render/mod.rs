//! The Html of a Word layout made by BC's own merger and converter (ADR-040 amendment 2026-11-15, accepted by the owner
//! 2026-11-15): `OfficeWordDocumentMerger.MergeWordDocument(layout, dataset, null, .., <report caption>, "", ..)`, then
//! `WordTransformation.ConvertToHtml`, both from the BC artifact through the run's renderer (`DotNetConfig::renderer`).
//!
//! Only a save of a measured shape (`shape`) renders. Every other Html save keeps the opaque value of the amendment of
//! 2026-11-12, Part A (`report_placeholder`), so no shape main answered changes. The Html written here carries no mark: it is
//! plain text, every read answers (decision 3).
//!
//! the Html of a Word layout made by BC's own merger and converter through `DotNetConfig::renderer`.
//! `mod.rs` the gate and the render (`Session::rendered_html`); `call.rs` the facts of the
//! `Report.SaveAs` call (`SaveCall`, taken in `reports.rs` `report_save_as`); `shape.rs` the measured
//! reports and datasets, each fact by equality (report properties and sections, the title, columns,
//! rows, major and converter version), and the layout vocabulary (parts, relationships, element names
//! per part, namespaces; the structure is free); `layout_facts.rs` reads the layout facts from the
//! file. The save's whole fact vector after the run is compared in `report_save/html_cell.rs`. The
//! content mark of the Word, Excel and Html placeholders, and what may still answer on them
//! (presence, size, byte copies), is `al-builtins/src/placeholder_output.rs`.

mod call;
mod layout_facts;
mod shape;

pub(crate) use call::{RenderInputs, SaveCall};

use crate::report_save::Renders;
use crate::reports::Schema;
use crate::{Session, Signal};
use al_host::render::{RenderNeed, WordMerge};

/// What the real Html of a Word layout needs of the run's renderer: BC's merger, then its Html converter.
pub const WORD_HTML_NEEDS: [RenderNeed; 2] = [RenderNeed::Merge, RenderNeed::Html];

/// The facts of the save itself, each by equality with every probe cell (RxHtmlHashTests.al:192, :218, :395, :433 and :471,
/// RxHtmlWidenTests.al:394, :440, :482, :568 and :614, through `HashCell`): its whole fact vector after the run is the
/// cells' (`html_cell`, from `report_save`), its call is the cells' (`call::MEASURED_CALL`), it renders with the report's
/// own default layout (`Renders::Default`), on a major of `shape::BUILDS`.
pub(crate) fn save_is_measured(html_cell: bool, call: SaveCall, renders: Renders, major: Option<u16>) -> bool {
    html_cell
        && call == call::MEASURED_CALL
        && renders == Renders::Default
        && major.is_some_and(|m| shape::BUILDS.iter().any(|b| b.major == m))
}

impl Session {
    /// The Html BC writes for this save, when its shape is measured and the renderer can make it; `None` keeps Part A. A
    /// merger or converter failure refuses the save (no error text is pinned; decision 2).
    pub(crate) fn rendered_html(
        &mut self,
        html_cell: bool,
        schema: &Schema,
        renders: Renders,
        dataset: &[u8],
        inputs: &RenderInputs,
    ) -> Result<Option<Vec<u8>>, Signal> {
        let major = self.engine.target.as_ref().map(|t| t.bc.major);
        if !save_is_measured(html_cell, inputs.call, renders, major) {
            return Ok(None);
        }
        let program = &self.engine.program;
        let Some(layout) = crate::report_layout_export::default_word_layout(program, schema.object()) else {
            return Ok(None);
        };
        let Some(facts) = layout_facts::read(&layout).filter(shape::content_is_measured) else {
            return Ok(None);
        };
        let Some((save, title)) = shape::save_shape(program, schema.object(), &facts, dataset, inputs.kinds) else {
            return Ok(None);
        };
        // A Caption a translation file could change in the call's language 1033 is not the measured title
        // (RxHtmlWidenTests.al:648, measured in an app that ships no file of that language).
        let translated = title.caption && self.app_translates_into(schema.object(), call::MEASURED_CALL.language);
        if !shape::shape_is_measured(shape::SHAPES, &save) || translated {
            return Ok(None);
        }
        let renderer = self.engine.dotnet.renderer.clone();
        // A converter whose version no cell measured on this major gives Html that keeps Part A: do not render it.
        if let Some(version) = renderer.converter_version()
            && !major.is_some_and(|m| shape::converter_is_measured(shape::BUILDS, m, version))
        {
            return Ok(None);
        }
        let available = WORD_HTML_NEEDS.iter().try_for_each(|n| renderer.availability(*n));
        match available {
            Ok(()) => {}
            Err(why) if self.engine.dotnet.require_renderer => {
                return Err(Signal::unsupported(format!(
                    "--require-report-renderer: Report.SaveAs as Html of report \"{}\" needs BC's own merger and converter, \
                     and the report renderer is unavailable ({why})",
                    schema.name()
                )));
            }
            Err(_) => return Ok(None),
        }
        let refuse = |step: &str, e: al_host::render::RenderFailure| {
            Signal::unsupported(format!(
                "Report.SaveAs as Html of report \"{}\": BC's {step} failed ({e})",
                schema.name()
            ))
        };
        // The title is the report's Caption, or its name when it states none (`shape::save_shape`).
        let docx = renderer
            .merge_word(&WordMerge {
                layout,
                dataset: dataset.to_vec(),
                title: title.text,
                company: String::new(),
                pictures: false,
            })
            .map_err(|e| refuse("Word merger", e))?;
        let html = renderer.word_to_html(&docx).map_err(|e| refuse("Html converter", e))?;
        // The converter writes its version into the Html: only a version a cell measured on this major is BC's answer.
        if !major.is_some_and(|m| shape::build_is_measured(shape::BUILDS, m, &html)) {
            return Ok(None);
        }
        // BC writes these bytes: RxHtmlHashTests.al:192, :218, :395, :433 and :471 assert their SHA-256 and length, and
        // RxHtmlWidenTests.al:394, :440, :482, :568 and :614 assert the Html's facts.
        Ok(Some(html))
    }
}

#[cfg(test)]
mod tests;
