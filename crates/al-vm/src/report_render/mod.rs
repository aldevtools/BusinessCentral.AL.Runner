//! The Html of a Word layout made by BC's own merger and converter (ADR-040 amendment 2026-11-15, accepted by the owner
//! 2026-11-15): `OfficeWordDocumentMerger.MergeWordDocument(layout, dataset, null, .., <report name>, "", ..)`, then
//! `WordTransformation.ConvertToHtml`, both from the BC artifact through the run's renderer (`DotNetConfig::renderer`).
//!
//! Only a save of a measured shape (`shape`) renders. Every other Html save keeps the opaque value of the amendment of
//! 2026-11-12, Part A (`report_placeholder`), so no shape main answered changes. The Html written here carries no mark: it is
//! plain text, every read answers (decision 3).

mod call;
mod layout_facts;
mod shape;

pub(crate) use call::{RenderInputs, SaveCall};

use crate::report_save::Renders;
use crate::reports::Schema;
use crate::{Session, Signal};
use al_builtins::assumptions::{Assumption, assume};
use al_host::render::{RenderNeed, WordMerge};

/// The facts of the save itself, each by equality with every probe cell (RxHtmlHashTests.al:192, :218, :395, :433 and :471,
/// through `HashCell`): its whole fact vector after the run is the cells' (`html_cell`, from `report_save`), its call is the
/// cells' (`call::MEASURED_CALL`), it renders with the report's own default layout (`Renders::Default`), on a major of
/// `shape::BUILDS`.
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
        let measured = shape::save_shape(program, schema.object(), &facts, dataset, inputs.kinds)
            .is_some_and(|s| shape::shape_is_measured(shape::SHAPES, &s));
        if !measured {
            return Ok(None);
        }
        let renderer = self.engine.dotnet.renderer.clone();
        // A converter whose version no cell measured on this major gives Html that keeps Part A: do not render it.
        if let Some(version) = renderer.converter_version()
            && !major.is_some_and(|m| shape::converter_is_measured(shape::BUILDS, m, version))
        {
            return Ok(None);
        }
        let available = renderer
            .availability(RenderNeed::Merge)
            .and(renderer.availability(RenderNeed::Html));
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
        // The title is the report's name: the report states neither Caption nor CaptionML (`shape::save_shape`).
        let docx = renderer
            .merge_word(&WordMerge {
                layout,
                dataset: dataset.to_vec(),
                title: schema.name().to_string(),
                company: String::new(),
                pictures: false,
            })
            .map_err(|e| refuse("Word merger", e))?;
        let html = renderer.word_to_html(&docx).map_err(|e| refuse("Html converter", e))?;
        // The converter writes its version into the Html: only a version a cell measured on this major is BC's answer.
        if !major.is_some_and(|m| shape::build_is_measured(shape::BUILDS, m, &html)) {
            return Ok(None);
        }
        assume(Assumption::ReportHtmlRenderedByBcConverter);
        Ok(Some(html))
    }
}

#[cfg(test)]
mod tests;
