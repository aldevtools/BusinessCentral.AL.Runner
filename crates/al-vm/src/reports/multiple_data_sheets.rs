//! `ExcelLayoutMultipleDataSheets = true` on a report (al-runner issue 7836). The corpus probe measured that the property
//! changes nothing in these runs (assumption excel-layout-multiple-data-sheets, reports/layouts-saveas/
//! ExcelMultipleDataSheetsReport.al): they go on as the same report without it. Every other run of such a report refuses.

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

/// Refuses a run of a report with the property outside the measured shapes. The measured shapes, by the default layout's
/// kind (`al_ir::Report::default_layout_kind`, empty for no layout):
/// - `SaveAs` as Xml with no layout, a Word or an Excel layout: the same dataset as without the property
///   (XMS_SaveAsXml_SameDatasetAsWithoutProperty);
/// - `Run` and `RunModal` without the request page, no layout or an Excel layout: ReportNoLayout and
///   ReportUnsupportedOperation (XMS_Run_NoRequestPage);
/// - `SaveAs` as Excel, no layout or an Excel layout: nothing written, and a file that is not empty (XMS_SaveAsExcel).
pub(crate) fn gate(report: &al_ir::Report, name: &str, op: Op) -> Result<(), Signal> {
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
        "a run of report \"{name}\" with ExcelLayoutMultipleDataSheets = true other than SaveAs as Xml, SaveAs as Excel or Run \
         without the request page of a report with no layout or an Excel layout (not pinned)"
    )))
}
