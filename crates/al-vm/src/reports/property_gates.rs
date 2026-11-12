//! Report properties a run checks before it starts.
//!
//! `ExcelLayoutMultipleDataSheets = true` (al-runner issue 7836, [`gate`]): the corpus probe measured that the property changes
//! nothing in some runs (assumption excel-layout-multiple-data-sheets, reports/layouts-saveas/ExcelMultipleDataSheetsReport.al);
//! they go on as the same report without it. Every other run of such a report refuses.
//!
//! The declaration refusals of a SaveAs: [`save_as_declaration_refusals`].

use al_builtins::assumptions::{Assumption, hit};
use al_builtins::native::Signal;

/// The run of a report that is asked about.
pub(crate) enum Op<'a> {
    /// `Report.SaveAs` as this format (`Xml`, `Excel`, ...).
    SaveAs(&'a str),
    /// `Report.Run` or `Report.RunModal`; `page` says whether the request page opens.
    Run { page: bool },
    /// Execute, Print and the other entries.
    Other,
}

impl Op<'_> {
    /// `Run { page }` for `Report.Run` and `RunModal` (`by_run`), else `Other`.
    pub(crate) fn run(by_run: bool, page: bool) -> Self {
        if by_run { Op::Run { page } } else { Op::Other }
    }
}

/// Refuses a run of a report with the property outside the measured shapes. The measured shapes, by the default layout's
/// kind (`al_ir::Report::default_layout_kind`, empty for no layout):
/// - `SaveAs` as Xml with no layout, a Word or an Excel layout: the same dataset as without the property
///   (XMS_SaveAsXml_SameDatasetAsWithoutProperty);
/// - `Run` and `RunModal` without the request page, no layout or an Excel layout: ReportNoLayout and
///   ReportUnsupportedOperation (XMS_Run_NoRequestPage);
/// - `SaveAs` as Excel: a file that is not empty with an Excel layout, nothing written with no layout (XMS_SaveAsExcel).
pub(crate) fn gate(schema: &super::Schema, op: Op) -> Result<(), Signal> {
    let report = schema.report();
    if !report.excel_multiple_data_sheets {
        return Ok(());
    }
    let kind = report.default_layout_kind.as_str();
    let measured = match op {
        Op::SaveAs("Xml") => matches!(kind, "" | "word" | "excel"),
        Op::SaveAs("Excel") | Op::Run { page: false } => matches!(kind, "" | "excel"),
        _ => false,
    };
    if measured && hit(Assumption::ExcelLayoutMultipleDataSheets) {
        return Ok(());
    }
    Err(Signal::unsupported(format!(
        "a run of report \"{}\" with ExcelLayoutMultipleDataSheets = true other than SaveAs as Xml, SaveAs as Excel or Run \
         without the request page of a report with no layout or an Excel layout (not pinned)",
        schema.name()
    )))
}

/// What a `Report.SaveAs` as `format` refuses in the report's declaration before it runs: a dataset this runtime cannot write,
/// a TransactionType (what SaveAs does with it is not pinned), and a run of [`gate`].
pub(crate) fn save_as_declaration_refusals(schema: &super::Schema, format: &str) -> Result<(), Signal> {
    if let Some(why) = schema.report().dataset_unsupported.first() {
        return Err(Signal::unsupported(format!(
            "Report.SaveAs of \"{}\": {why}",
            schema.name()
        )));
    }
    gate(schema, Op::SaveAs(format))?;
    if schema.report().transaction_type.is_some() {
        return Err(Signal::unsupported(format!(
            "Report.SaveAs of \"{}\", which declares a TransactionType (what SaveAs does with it is not pinned)",
            schema.name()
        )));
    }
    Ok(())
}
