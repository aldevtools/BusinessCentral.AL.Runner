//! The errors a row of the save table answers with (`cell::ErrorId`), each with its corpus line.

use super::Format;
use super::cell::ErrorId;
use al_builtins::native::AlError;

fn error(message: String, code: &str) -> AlError {
    AlError {
        message,
        code: code.into(),
        call_stack: String::new(),
        ..Default::default()
    }
}

/// The error of `id` for a save of report `report_id` as `format`.
pub(super) fn error_of(id: ErrorId, report_id: u32, format: Format) -> AlError {
    // reports/PRSaveAsFormats.al:44 (code Dialog); reports/html-email-body/RxLayoutSelectionTests.al:746-796 (code
    // ReportNoLayout, corpus pull request 1371).
    let no_layout = || {
        format!(
            "Report {report_id} does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom."
        )
    };
    match id {
        ErrorId::NoValidLayout => error(no_layout(), "Dialog"),
        ErrorId::NoLayout => error(no_layout(), "ReportNoLayout"),
        // text/TestReportSaveAsDatasetContracts.al:410, reports/CiReportRunLeads.al:603, :659, :664, :268.
        ErrorId::UnsupportedFormat => error(
            format!(
                "The {} output format is not supported by the current report layout format.",
                format.name()
            ),
            "ReportUnsupportedFormat",
        ),
        // reports/html-email-body/RxLayoutSelectionTests.al:852 `used err|text` and `used err|code` (corpus pull request 1371).
        ErrorId::ResultEmpty => error(
            "The report couldn\u{2019}t be generated, because it was empty. Adjust your filters and try again.".into(),
            "ReportResultEmpty",
        ),
    }
}
