//! A rendering save writes a placeholder file, and a run renders nothing (ADR-040 amendments "A placeholder PDF" of
//! 2026-11-08 and "Placeholder Word, Excel and Html outputs, and a run without output" of 2026-11-11).
//!
//! `Report.SaveAs(.., Pdf | Word | Excel | Html, ..)`, `SaveAsPdf`/`SaveAsWord`/`SaveAsExcel`/`SaveAsHtml` (by number and on
//! a variable) and `TestRequestPage.SaveAsPdf`/`SaveAsExcel` run the report exactly as the Xml path does (request
//! parameters, triggers, data items, errors) and write a small valid file of the format (`files`) where BC writes its
//! rendering. Which format a default layout gives is the grid of `grid`. The bytes are not BC's: only that the output exists,
//! is of the right kind and is not empty is meant to match. The output carries the mark of `al_builtins::placeholder_output`,
//! so a read of its content is refused (a read inside the PDF's pinned first bytes `%PDF-` answers:
//! `placeholder_output::pdf_read`). Each path marks its assumption (`report-pdf-placeholder`, `report-word-placeholder`,
//! `report-excel-placeholder`, `report-html-placeholder`), so a test that passes through it is a flagged pass.
//! `run` is the Run/RunModal/Execute of a report that renders: it runs the Xml path and writes nothing
//! (`report-run-without-output`).
//!
//! Microsoft tests that need the Pdf form (none reads the bytes): `Library - Post Inventory to G/L`.PostInvtCostToGL calls
//! `Post Inventory Cost to G/L`.SaveAsPdf (SCM-Costing, SCM-Manufacturing, SCM-Assembly, SCM); the request page
//! handlers of Report Layout, ERM Account Schedule II and ERM Purchase Reports III call `SaveAsPdf`;
//! ReportCatalogTests CustomerStatementReport checks the file is not empty (`CheckFileNotEmpty`). The Html form: e-mail body
//! reports of an outside reporter's suite (al-runner issue 6924) that copy the Html into an attachment or check that it saved.

mod files;
mod run;

pub use files::{Format, placeholder_pdf};

use crate::report_save::{AfterRun, Plan, Renders};
use crate::reports::{SaveTarget, Schema};
use crate::{Session, Signal};

/// What a rendering save writes.
#[derive(Clone, Debug)]
pub(crate) enum SaveOutput {
    /// The Xml dataset the run wrote.
    Dataset,
    /// Nothing; the target is marked as pending (ADR-040 allow-list, and the company build).
    Discard,
    /// What the row of the save table answers after the run (`report_save`).
    Rendered(Box<Plan>),
}

impl Session {
    /// Puts the output of a rendering save that ran to its end where the caller asked for it, and gives the save's answer.
    /// `rows` is the number of data item records the run wrote to its dataset.
    pub(crate) fn write_save_output(
        &mut self,
        target: SaveTarget,
        output: SaveOutput,
        schema: &Schema,
        dataset: &[u8],
        rows: usize,
    ) -> Result<bool, Signal> {
        use al_builtins::assumptions::{Assumption, assume};
        match (output, target) {
            (SaveOutput::Dataset, SaveTarget::Stream(blob)) => al_builtins::stream::write_bytes(&blob, dataset),
            (SaveOutput::Dataset, SaveTarget::File(name)) => self.write_save_file(&name, dataset)?,
            (SaveOutput::Rendered(plan), target) => {
                let (renders, selected, rgba) = match self.save_after_run(&plan, schema, dataset, rows)? {
                    AfterRun::Answer(saved) => return Ok(saved),
                    AfterRun::Write {
                        renders,
                        selected,
                        rgba,
                    } => (renders, selected, rgba),
                };
                let format = plan.format();
                let layout_kind = match renders {
                    Renders::StoredRdlc => "rdlc",
                    Renders::Default | Renders::Selected => schema.report().default_layout_kind.as_str(),
                };
                // Every placeholder carries the mark, so a read of its content is refused (amendment 2026-11-11; the PDF too,
                // owner decision 2026-11-12: only a read inside the bytes the corpus pins at its start answers,
                // `al_builtins::placeholder_output::pdf_read`).
                let mark = match format {
                    Format::Html => html_render_mark(schema, rows, selected.as_deref(), rgba),
                    _ => al_builtins::placeholder_output::mark(format.name(), schema.name()),
                };
                self.placeholder_files.saved(&mark, layout_kind);
                match target {
                    SaveTarget::Stream(blob) => {
                        al_builtins::stream::write_bytes(&blob, &format.bytes());
                        blob.borrow_mut().render_pending = Some(mark);
                    }
                    SaveTarget::File(name) => {
                        al_builtins::native::Ctx::fs_write_placeholder(self, &name, &format.bytes(), mark)
                            .map_err(Signal::unsupported)?;
                    }
                }
                assume(match format {
                    Format::Pdf => Assumption::ReportPdfPlaceholder,
                    Format::Word => Assumption::ReportWordPlaceholder,
                    Format::Excel => Assumption::ReportExcelPlaceholder,
                    Format::Html => Assumption::ReportHtmlPlaceholder,
                });
            }
            (SaveOutput::Discard, SaveTarget::Stream(blob)) => {
                // The dataset is dropped; the buffer is marked and any look at it is refused.
                blob.borrow_mut().render_pending = Some(schema.name().into());
                assume(Assumption::ReportRenderDiscarded);
            }
            (SaveOutput::Discard, SaveTarget::File(name)) => {
                self.placeholder_files.written(&name);
                self.render_pending_files
                    .insert(crate::ctx::pending_key(&name), schema.name().into());
                assume(Assumption::ReportRenderDiscarded);
            }
        }
        Ok(true)
    }

    /// Writes a file a save produced; a write over a path that was pending ends the mark, as a write by AL does.
    pub(crate) fn write_save_file(&mut self, name: &str, bytes: &[u8]) -> Result<(), Signal> {
        al_builtins::native::Ctx::fs_write(self, name, bytes).map_err(Signal::unsupported)
    }
}

/// The mark of the Html placeholder a save of `schema` writes, with the number of its render: a whole text read of it gives
/// the rendered content's mark (`al_builtins::rendered_text`, ADR-040 amendment 2026-11-12, A1 and A2). The render number is
/// taken only for a run with rows. `selected` is the layout a temporary layout selection named (A7), else the report's default
/// layout renders; `rgba` the rgba column of the save's row.
fn html_render_mark(
    schema: &Schema,
    rows: usize,
    selected: Option<&str>,
    rgba: al_builtins::rendered_text::RgbaReplace,
) -> std::sync::Arc<str> {
    let report = schema.report();
    if rows == 0 {
        return al_builtins::rendered_text::html_mark(schema.name(), None);
    }
    let layout = selected.map(str::to_string).unwrap_or_else(|| {
        report
            .default_rendering_layout
            .clone()
            .unwrap_or_else(|| report.default_layout_kind.clone())
    });
    let render = al_builtins::rendered_text::register_with(schema.name(), "Html", &layout, rgba);
    al_builtins::rendered_text::html_mark(schema.name(), render)
}
