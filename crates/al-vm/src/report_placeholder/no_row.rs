//! A rendering save whose run wrote no dataset row (ADR-040). BC then answers by the layout, measured by corpus pull requests
//! 1362 and 1371 (open; the same on the three BC majors the corpus runs), reports/html-email-body/RxLayoutSelectionTests.al:
//!
//! | Default layout | Formats | Answer used | Statement |
//! |---|---|---|---|
//! | Word, named by `DefaultRenderingLayout` | Pdf :852, Word :873, Html :915 | false, `ReportResultEmpty` | raises it |
//! | Word, `DefaultLayout = Word` | Html :979, Pdf :1001 | false, `ReportResultEmpty` | raises it |
//! | RDLC, named by `DefaultRenderingLayout` | Pdf :937, Word :958 | true, nothing written | nothing written |
//!
//! Every cell: one data item, no report extension, a stream target, no layout selection (no stored row of the report in
//! Report Layout Selection or Tenant Report Layout Selection, of any company). The Word layout report of the first
//! row declares OnPreReport, OnPreRendering and OnPostReport, and BC runs the first two and not OnPostReport (`used log`
//! `OnPreReport;OnPreRendering;`); this runtime has run OnPostReport by the time it counts the rows, so a report that declares
//! it refuses: the three cells of that row themselves (report 80020) refuse. The shape of that row that runs here, a report
//! without OnPostReport, has reports/html-email-body/RxEmailBodyTests.al:134-143 (Html, answer used, report 80013, no report
//! trigger) as its direct cell; Pdf, Word, the statement form and the run of OnPreReport and OnPreRendering follow from the
//! first row's cells, where BC ran those two triggers and skipped OnPostReport. The reports of the other two rows declare no
//! report trigger, so only such reports take those rows.
//! The paths run under the assumption `report-save-no-dataset-row` until the probes are pinned. A Word layout report outside
//! the cells refuses; an RDLC report outside them writes its placeholder as with rows, as before.

use super::{Format, SaveOutput, report_shape};
use crate::reports::{SaveTarget, Schema};
use crate::{Session, Signal};
use al_builtins::native::AlError;
use al_builtins::rendered_text::{DataItems, Extensions, ReportShape};

/// What a measured cell answers.
enum Answer {
    /// False with `ReportResultEmpty` left as the last error; the statement form raises it.
    Empty,
    /// False, the last error untouched: main's answer kept for the Html save, with its answer used, of a `DefaultLayout = Word`
    /// report that declares a report trigger. No cell measures that shape (a legacy declared report with a report trigger);
    /// every measured Word cell leaves `ReportResultEmpty`, so the untouched error is probably not BC's answer.
    FalseOnly,
    /// True, nothing written.
    TrueNothing,
}

/// RxLayoutSelectionTests.al:852 `used err|text` and `used err|code`.
fn result_empty() -> AlError {
    AlError {
        message: "The report couldn\u{2019}t be generated, because it was empty. Adjust your filters and try again."
            .into(),
        code: "ReportResultEmpty".into(),
        call_stack: String::new(),
        ..Default::default()
    }
}

impl Session {
    /// The answer of a placeholder save whose run wrote no dataset row, `None` for a save outside the module's table that
    /// writes as with rows.
    pub(super) fn no_row_save(
        &mut self,
        output: &SaveOutput,
        target: &SaveTarget,
        schema: &Schema,
        answer_used: bool,
    ) -> Result<Option<bool>, Signal> {
        let report = schema.report();
        let (format, name) = match output {
            SaveOutput::Placeholder(f) => (Some(*f), f.name()),
            SaveOutput::SelectedHtml(_) => (None, "Html with a temporary layout selection"),
            SaveOutput::StoredRdlc(f) => {
                return Err(Signal::unsupported(format!(
                    "Report.SaveAs as {} of report \"{}\", whose run wrote no dataset row, while a stored Report Layout \
                     Selection row selects its RDLC layout (not measured)",
                    f.name(),
                    schema.name()
                )));
            }
            SaveOutput::Dataset | SaveOutput::Discard => return Ok(None),
        };
        let kind = report.default_layout_kind.as_str();
        let shape = report_shape(&self.engine.program, schema);
        let declared = shape.rendering_layout_declared;
        let no_report_trigger = report.on_init.is_none()
            && report.on_pre.is_none()
            && report.on_post.is_none()
            && report.on_pre_rendering.is_none();
        // No stored layout selection row of the report, in either table and of any company: every cell was measured
        // without one, and with one the selected layout may render (RxLayoutSelectionTests.al:452, :501-:648).
        let row = self.selection_row_of_report(schema)?;
        // What every cell shares.
        let base = row.is_none()
            && matches!(target, SaveTarget::Stream(_))
            && shape
                == ReportShape {
                    data_items: DataItems::One,
                    extensions: Extensions::None,
                    rendering_layout_declared: declared,
                };
        let answer = match (kind, declared, format) {
            ("word", true, Some(Format::Pdf | Format::Word | Format::Html)) if base => {
                if report.on_post.is_some() {
                    return Err(Signal::unsupported(format!(
                        "Report.SaveAs as {name} of report \"{}\", a Word layout report whose run wrote no dataset row and that \
                         declares OnPostReport (BC does not run it then; this runtime has run it before it counts the rows)",
                        schema.name()
                    )));
                }
                Some(Answer::Empty)
            }
            ("word", false, Some(Format::Html | Format::Pdf)) if base && no_report_trigger => Some(Answer::Empty),
            ("word", false, Some(Format::Html)) if base && answer_used => Some(Answer::FalseOnly),
            ("rdlc", true, Some(Format::Pdf | Format::Word)) if base && no_report_trigger => Some(Answer::TrueNothing),
            _ => None,
        };
        let Some(answer) = answer else {
            if kind != "word" {
                return Ok(None);
            }
            if let Some(what) = row {
                return Err(Signal::unsupported(format!(
                    "Report.SaveAs as {name} of report \"{}\", a Word layout report whose run wrote no dataset row, while \
                     {what} (measured only without a stored layout selection row)",
                    schema.name()
                )));
            }
            return Err(Signal::unsupported(format!(
                "Report.SaveAs as {name} of report \"{}\", a Word layout report whose run wrote no dataset row (measured only for \
                 a save to a stream of a report with one data item and no report extension: as Pdf, Word or \
                 Html when DefaultRenderingLayout names the layout, as Html or Pdf of a DefaultLayout = Word report without \
                 report triggers)",
                schema.name()
            )));
        };
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReportSaveNoDatasetRow);
        match answer {
            Answer::Empty if answer_used => {
                self.last_error = result_empty();
                Ok(Some(false))
            }
            Answer::Empty => Err(Signal::Error(Box::new(result_empty()))),
            Answer::FalseOnly => Ok(Some(false)),
            Answer::TrueNothing => Ok(Some(true)),
        }
    }
}
