//! What a rendering save of a report does, by format and default layout (ADR-040 amendment of 2026-11-11, the grid):
//!
//! | Format | RDLC layout | Word layout | Excel layout |
//! |---|---|---|---|
//! | Pdf | placeholder | placeholder | false, `ReportUnsupportedFormat` |
//! | Word | placeholder | placeholder | false, `ReportUnsupportedFormat` |
//! | Excel | placeholder | false, `ReportUnsupportedFormat` | placeholder |
//! | Html | false, `ReportUnsupportedFormat` | placeholder | false, `ReportUnsupportedFormat` |
//!
//! The placeholders: reports/layouts-saveas/AMReportOutputTests.al:173 (Word layout to Pdf), :187 (to Word), :205 (to Html),
//! :235 (Excel layout to Excel); reports/CiReportRunLeads.al:581-617 TargetFormat_PageNo_WordDefaultLayout (Pdf, Word, Html of
//! a Word layout: true, length > 0), :180-226 (Excel to Excel). The false answers: AMReportOutputTests.al:221 and
//! CiReportRunLeads.al:601-605 (Word layout to Excel), AMReportOutputTests.al:257, CiReportRunLeads.al:219-223 and :657-661
//! (Excel layout to Pdf), CiReportRunLeads.al:267-270 (Excel to Word), :662-666 (Excel to Html), text/
//! TestReportSaveAsDatasetContracts.al:SaveAs_Html_WithAnRdlcLayout_IsRefused (RDLC to Html). RDLC to Word and Excel: true, length
//! > 0 (CiReportRunLeads.al:674-692 SaveAs_RdlcOnly_WordExcel; reports/layouts-saveas/AMRdlcOutput.al:336-373 a zip, Windows values).

use super::{Format, SaveOutput};
use crate::reports::{SaveTarget, Schema};
use crate::{Session, Signal};
use al_builtins::native::{AlError, unsupported};
use al_ir as ir;

/// What a rendering save goes on with.
pub(crate) enum Rendered {
    /// Run the report and write this.
    Write(SaveOutput),
    /// Answer false with this error in `GetLastErrorText`; nothing runs and nothing is written.
    False(AlError),
}

/// Whether a save as `format` of a report whose default layout is of `kind` writes the placeholder.
pub(super) fn writes(kind: &str, format: Format) -> bool {
    matches!(
        (kind, format),
        ("rdlc", Format::Pdf | Format::Word | Format::Excel)
            | ("word", Format::Pdf | Format::Word | Format::Html)
            | ("excel", Format::Excel)
    )
}

/// Whether BC answers a save as `format` of a report whose default layout is of `kind` with `ReportUnsupportedFormat` (the
/// cells of the 2026-11-11 grid; RDLC to Html is the older `rdlc_html` arm).
fn answers_unsupported(kind: &str, format: Format) -> bool {
    matches!(
        (kind, format),
        ("word", Format::Excel) | ("excel", Format::Pdf | Format::Word | Format::Html)
    )
}

/// The error of a format the default layout cannot give (TestReportSaveAsDatasetContracts.al:SaveAs_Html_WithAnRdlcLayout_IsRefused,
/// CiReportRunLeads.al:603, :659, :664, :268; the code: al-language-onprem/reports/layouts-saveas/AMRdlcSaveAs.al:461-483).
fn unsupported_format(format: &str) -> AlError {
    AlError {
        message: format!("The {format} output format is not supported by the current report layout format."),
        code: "ReportUnsupportedFormat".into(),
        call_stack: String::new(),
        ..Default::default()
    }
}

/// Whether the format is one the default layout cannot give, so a static SaveAs raises SubstituteReport and nothing after it
/// (reports/layouts-saveas/AMRdlcOutput.al:TextRdlc_SaveAsHtml_IsRefused `SubstituteReport(79256,SaveAs,req=Yes);`;
/// AMReportOutputTests.al:221 and :257 `SubstituteReport(7925N,SaveAs,req=Yes);` for a Word layout to Excel and an Excel layout to Pdf).
pub(crate) fn format_refused_by_layout(schema: &Schema, format: &str) -> bool {
    let report = schema.report();
    if report.processing_only {
        return false;
    }
    (format == "Html" && report.layout == ir::ReportLayout::Rdlc)
        || Format::from_name(format).is_some_and(|f| answers_unsupported(&report.default_layout_kind, f))
}

impl Session {
    /// What a save as `format` (not Xml) does once the report and its layout are known: run and write the dataset's
    /// replacement, or answer false at once. A save no arm pins is refused.
    pub(crate) fn rendered_save(
        &mut self,
        schema: &Schema,
        format: &str,
        target: &SaveTarget,
        result_used: bool,
        custom: bool,
    ) -> Result<Rendered, Signal> {
        let report = schema.report();
        let go = |o| Ok(Rendered::Write(o));
        // reports/PRSaveAsFormats.al SaveAs_Formats_OfReportWithoutLayoutFile: Pdf, Html, Word and Excel of a report with
        // `DefaultLayout` and no layout file answer false, write nothing and set this text. With the answer dropped BC's
        // behaviour is not pinned.
        if report.layout_file_missing && result_used {
            return Ok(Rendered::False(AlError {
                message: format!(
                    "Report {} does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom.",
                    self.object_id_of(schema.object as u32)
                ),
                code: "Dialog".into(),
                call_stack: String::new(),
                ..Default::default()
            }));
        }
        // A layout of Type = Custom is rendered by whoever subscribes to the platform's CustomDocumentMergerEx; the event is
        // raised once the dataset is written (assumption custom-layout-merger).
        if custom {
            return go(SaveOutput::Dataset);
        }
        if let Some(stored) = self.stored_selection(schema, target, result_used)? {
            return self.stored_selection_save(schema, format, stored);
        }
        if format_refused_by_layout(schema, format) {
            return self.answer_unsupported_format(schema, format, target, result_used);
        }
        // Owner decision 2026-10-22 ("Run triggers, no file"): while Microsoft's DemoTool builds the company, SaveAsPdf to a
        // file runs the report, triggers and data, and writes no file (MakeCorrections.Codeunit.al:62, PostRevaluation.
        // Codeunit.al:82, RunPostInventoryCosttoGL.Codeunit.al:17 write a PDF under TemporaryPath that nothing reads). The
        // file is marked as for a listed report, so a look at it is refused. Outside the company build it stays refused.
        if format == "Pdf" && self.company_build && matches!(target, SaveTarget::File(_)) {
            return go(SaveOutput::Discard);
        }
        // A processing-only report goes on to its pinned answer (TestReportSaveAsColumnTypesContracts.al:372-388,
        // reports/runtime/LtTkShowPrintStatus.al:621); nothing is rendered.
        if format == "Pdf" && report.processing_only {
            return go(SaveOutput::Dataset);
        }
        if let Some(f) = Format::from_name(format) {
            if let Some(output) = self.placeholder_allowed(schema, f, target)? {
                return go(output);
            }
            if self.output_is_ignored(schema) {
                return go(SaveOutput::Discard);
            }
        }
        unsupported(format!(
            "Report.SaveAs of \"{}\" as {format}: only the Xml dataset and placeholders of the formats a default layout gives \
             are produced (any other output needs a layout renderer)",
            schema.name()
        ))
    }

    /// What a save does under a measured stored selection (`stored`): the grid's RDLC column, or the false answer of a
    /// selection that names no layout (reports/html-email-body/RxLayoutSelectionTests.al:452, :477, :746, :771, corpus pull
    /// request 1371). The layout events BC raises around it are not raised, so a subscriber the test can see refuses.
    fn stored_selection_save(
        &mut self,
        schema: &Schema,
        format: &str,
        stored: super::stored::Stored,
    ) -> Result<Rendered, Signal> {
        use super::stored::Stored;
        let Some(f) = Format::from_name(format) else {
            return unsupported(format!(
                "Report.SaveAs as {format} of report \"{}\" while a stored Report Layout Selection row selects a layout \
                 for it (measured for Pdf, Word, Excel and Html)",
                schema.name()
            ));
        };
        self.refuse_layout_event_subscribers(schema, &format!("Report.SaveAs as {format}"), &[], None)?;
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ReportStoredLayoutSelection);
        Ok(match (stored, f) {
            // RxLayoutSelectionTests.al:746-796: the text and the code ReportNoLayout, for each of the four formats.
            (Stored::NoLayout, _) => Rendered::False(AlError {
                message: format!(
                    "Report {} does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom.",
                    self.object_id_of(schema.object as u32)
                ),
                code: "ReportNoLayout".into(),
                call_stack: String::new(),
                ..Default::default()
            }),
            (Stored::Rdlc, Format::Html) => Rendered::False(unsupported_format(format)),
            (Stored::Rdlc, Format::Pdf | Format::Word | Format::Excel) => Rendered::Write(SaveOutput::StoredRdlc(f)),
        })
    }

    /// The false answer of a format the default layout cannot give (`format_refused_by_layout`).
    fn answer_unsupported_format(
        &self,
        schema: &Schema,
        format: &str,
        target: &SaveTarget,
        result_used: bool,
    ) -> Result<Rendered, Signal> {
        let rdlc_html = format == "Html" && schema.report().layout == ir::ReportLayout::Rdlc;
        // As a statement BC raises ReportUnsupportedFormat after the layout events SelectReportLayoutCode, HasCustomLayout,
        // SelectedBuiltinLayoutType and ApplicationReportMergeStrategy (events/platform-triggers/PeVTests.al:713
        // PW_RaisedEvents_HtmlFormat), which this runtime does not raise.
        if !result_used {
            return unsupported(format!(
                "Report.SaveAs as {format} of a report whose layout cannot give it, as a statement (the layout events BC raises \
                 before its error are not raised here)"
            ));
        }
        if rdlc_html {
            // The false answer is the report's own RDLC layout's; a layout selection may pick a Word layout, which renders Html.
            self.refuse_layout_selection(schema, &format!("Report.SaveAs as {format}"))?;
        } else {
            // Only the stream form is pinned for the cells of the 2026-11-11 grid.
            if matches!(target, SaveTarget::File(_)) {
                return unsupported(format!(
                    "SaveAs{format} to a file of report \"{}\", whose layout cannot give {format} (only the stream form is pinned)",
                    schema.name()
                ));
            }
            // SubstituteReport is raised by the caller and OnDocumentReady is not raised (AMReportOutputTests.al:221, :257); the
            // other layout events are not pinned for these layouts.
            self.refuse_unraised_layout_events(
                schema,
                &format!("Report.SaveAs as {format}"),
                &["substitutereport", "ondocumentready", "onafterdocumentready"],
            )?;
        }
        Ok(Rendered::False(unsupported_format(format)))
    }
}
