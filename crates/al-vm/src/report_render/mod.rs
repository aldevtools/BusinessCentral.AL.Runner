//! The Html of a Word layout made by BC's own merger and converter (ADR-040 amendment 2026-11-15, accepted by the owner
//! 2026-11-15): `OfficeWordDocumentMerger.MergeWordDocument(layout, dataset, null, .., <report name>, "", ..)`, then
//! `WordTransformation.ConvertToHtml`, both from the BC artifact through the run's renderer (`DotNetConfig::renderer`).
//!
//! Only a save of a measured shape (`shape`) renders. Every other Html save keeps the opaque value of the amendment of
//! 2026-11-12, Part A (`report_placeholder`), so no shape main answered changes. The Html written here carries no mark: it is
//! plain text, every read answers (decision 3).

mod layout_facts;
mod shape;

use crate::report_save::{Plan, Renders};
use crate::reports::Schema;
use crate::{Session, Signal};
use al_builtins::assumptions::{Assumption, assume};
use al_host::render::{RenderNeed, WordMerge};

/// The facts of the save itself, each by equality with every probe cell (RxHtmlHashTests.al:192, :218, :395, :433 and :471,
/// through `HashCell`): Html of a report whose default layout is Word, into a stream, its answer used, with no layout selection naming
/// the report (`Plan::is_measured_html_stream_save`), rendered with the report's own default layout (`Renders::Default`), on a
/// major of `shape::BUILDS`.
pub(crate) fn save_is_measured(plan_is_measured: bool, renders: Renders, major: Option<u16>) -> bool {
    plan_is_measured && renders == Renders::Default && major.is_some_and(|m| shape::BUILDS.iter().any(|b| b.major == m))
}

impl Session {
    /// The Html BC writes for this save, when its shape is measured and the renderer can make it; `None` keeps Part A. A
    /// merger or converter failure refuses the save (no error text is pinned; decision 2).
    pub(crate) fn rendered_html(
        &mut self,
        plan: &Plan,
        schema: &Schema,
        renders: Renders,
        dataset: &[u8],
        kinds: &[(String, Option<crate::dataset::ValueKind>)],
    ) -> Result<Option<Vec<u8>>, Signal> {
        let major = self.engine.target.as_ref().map(|t| t.bc.major);
        if !save_is_measured(plan.is_measured_html_stream_save(), renders, major) {
            return Ok(None);
        }
        let program = &self.engine.program;
        let Some(layout) = crate::report_layout_export::default_word_layout(program, schema.object()) else {
            return Ok(None);
        };
        let Some(facts) = layout_facts::read(&layout).filter(shape::content_is_measured) else {
            return Ok(None);
        };
        let measured = shape::save_shape(program, schema.object(), &facts, dataset, kinds)
            .is_some_and(|s| shape::shape_is_measured(shape::SHAPES, &s));
        if !measured {
            return Ok(None);
        }
        let renderer = self.engine.dotnet.renderer.clone();
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
        // The title is the report's name: no measured report has a caption (`shape::save_shape` requires none).
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
