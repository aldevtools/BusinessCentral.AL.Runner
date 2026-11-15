//! The Html of a Word layout made by BC's own merger and converter (ADR-040 amendment 2026-11-15, accepted by the owner
//! 2026-11-15): `OfficeWordDocumentMerger.MergeWordDocument(layout, dataset, null, .., <report caption>, "", ..)`, then
//! `WordTransformation.ConvertToHtml`, both from the BC artifact through the run's renderer (`DotNetConfig::renderer`).
//!
//! Only the saves of the measured cells (`cells`) render. Every other Html save keeps the opaque value of the amendment of
//! 2026-11-12, Part A (`report_placeholder`), so no shape main answered changes. The Html written here carries no mark: it is
//! plain text, every read answers (decision 3).

mod cells;

use crate::report_save::{Plan, Renders};
use crate::reports::Schema;
use crate::{Session, Signal};
use al_builtins::assumptions::{Assumption, assume};
use al_host::render::{RenderNeed, WordMerge, digest};

pub(crate) use cells::HtmlCell;

/// The inputs of one merge, and the digests a cell is matched by.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct HtmlInputs {
    pub(crate) layout: Vec<u8>,
    pub(crate) title: String,
    /// SHA-256 of the layout's bytes.
    pub(crate) layout_sha256: String,
    /// SHA-256 of the dataset after its `BCReportInformation` block (`bound_part`): the labels and data items the merger
    /// binds (the block holds the clock and the user, which no measured layout binds).
    pub(crate) data_sha256: String,
}

/// The cell of `cells::CELLS` whose every fact equals the save's, the gate of this module. Each fact by equality:
/// - `renders` is the report's default layout (the hash cells of the default saves);
/// - the save writes a stream and its answer is used (`Ok := Report.SaveAs(Id, '', ReportFormat::Html, OutS)` of every
///   cell), with no layout selection naming the report;
/// - the BC major, the layout's bytes, the bound dataset and the title equal the cell's.
pub(crate) fn measured_cell<'c>(
    table: &'c [HtmlCell],
    save_is_measured: bool,
    renders: Renders,
    major: Option<u16>,
    inputs: &HtmlInputs,
) -> Option<&'c HtmlCell> {
    if renders != Renders::Default || !save_is_measured {
        return None;
    }
    let major = major?;
    table.iter().find(|c| {
        c.majors.contains(&major)
            && c.layout_sha256 == inputs.layout_sha256
            && c.data_sha256 == inputs.data_sha256
            && c.title == inputs.title
    })
}

/// The dataset part the merger binds, as the cells key it: what follows `</BCReportInformation>` (`dataset::Header::write`),
/// without the `systemId` attribute of each data item record (from BC 29 a new GUID per row, so no two runs agree; BC's hash
/// is the same on every run, so the Html does not read it).
fn bound_part(dataset: &[u8]) -> Option<Vec<u8>> {
    const END: &[u8] = b"</BCReportInformation>\r\n";
    const ID: &[u8] = b" systemId=\"";
    let at = dataset.windows(END.len()).position(|w| w == END)?;
    let mut rest = &dataset[at + END.len()..];
    let mut out = Vec::with_capacity(rest.len());
    while let Some(i) = rest.windows(ID.len()).position(|w| w == ID) {
        out.extend_from_slice(&rest[..i]);
        let value = &rest[i + ID.len()..];
        let close = value.iter().position(|b| *b == b'"')?;
        rest = &value[close + 1..];
    }
    out.extend_from_slice(rest);
    Some(out)
}

impl Session {
    /// The Html BC writes for this save, when it is one of the measured cells and the renderer can make it; `None` keeps
    /// Part A. A merger or converter failure refuses the save (no error text is pinned; decision 2).
    pub(crate) fn rendered_html(
        &mut self,
        plan: &Plan,
        schema: &Schema,
        renders: Renders,
        dataset: &[u8],
    ) -> Result<Option<Vec<u8>>, Signal> {
        let Some(inputs) = self.html_inputs(schema, dataset) else {
            return Ok(None);
        };
        let major = self.engine.target.as_ref().map(|t| t.bc.major);
        if measured_cell(
            cells::CELLS,
            plan.is_measured_html_stream_save(),
            renders,
            major,
            &inputs,
        )
        .is_none()
        {
            return Ok(None);
        }
        let renderer = self.engine.dotnet.renderer.clone();
        if renderer.availability(RenderNeed::Merge).is_err() || renderer.availability(RenderNeed::Html).is_err() {
            return Ok(None);
        }
        let refuse = |step: &str, e: al_host::render::RenderFailure| {
            Signal::unsupported(format!(
                "Report.SaveAs as Html of report \"{}\": BC's {step} failed ({e})",
                schema.name()
            ))
        };
        let docx = renderer
            .merge_word(&WordMerge {
                layout: inputs.layout,
                dataset: dataset.to_vec(),
                title: inputs.title,
                company: String::new(),
                pictures: false,
            })
            .map_err(|e| refuse("Word merger", e))?;
        let html = renderer.word_to_html(&docx).map_err(|e| refuse("Html converter", e))?;
        assume(Assumption::ReportHtmlRenderedByBcConverter);
        Ok(Some(html))
    }

    /// The layout, title and digests of a save of `schema`'s default Word layout; `None` when a fact is not one a cell can
    /// hold (no layout file held by the app, a caption that is no literal, a dataset without the information block).
    fn html_inputs(&self, schema: &Schema, dataset: &[u8]) -> Option<HtmlInputs> {
        let program = &self.engine.program;
        let layout = crate::report_layout_export::default_word_layout(program, schema.object())?;
        let o = &program.objects[schema.object()];
        let title = match o.properties.iter().find(|(k, _)| k.eq_ignore_ascii_case("Caption")) {
            Some((_, v)) => crate::recmeta::string_literal(v)?,
            None => o.name.clone(),
        };
        let data_sha256 = digest(&bound_part(dataset)?);
        Some(HtmlInputs {
            layout_sha256: digest(&layout),
            layout,
            title,
            data_sha256,
        })
    }
}

#[cfg(test)]
mod tests;
