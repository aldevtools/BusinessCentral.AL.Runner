//! The rows of the save table where the platform's own check of a `Report.SaveAs` comes before any trigger (corpus pull
//! request 1579, merged, not yet pinned; assumption `saveas-platform-error-before-triggers`, issue 8486; the
//! processing-only answers with the answer used are pinned by streams/PnH6SaveAs.al). The probe's reports
//! (reports/layouts-saveas/SaveAsTriggerErrors.al) have one data item over Integer, no report extension, one layout at
//! most, no OnInitReport and no request page, and are saved by `Report.SaveAs(Id, '', Format, OutStream)` with no
//! layout selection row, as a statement inside `Codeunit.Run` and in the Boolean form; each test runs the control and
//! an error in OnPreReport, OnAfterGetRecord, OnPreDataItem and OnPostReport, and no error site is reached. Measured
//! the same on every supported major.

use super::Format;
use super::cell::K::Is;
use super::cell::*;
use super::cells::{UNSUPPORTED_EVENTS, cell, pin};
use super::facts::*;
use al_builtins::assumptions::Assumption;

const SAE: &str = "reports/layouts-saveas/SaveAsTriggerErrors.al";

/// "SAE Excel Rpt" (one Excel layout, DefaultRenderingLayout): SaveAs_Excel_Control, _OnPreReport, _OnAfterGetRecord,
/// _OnPreDataItemTestField, _OnPostReport.
const EXCEL: &[Pin] = &[
    pin(SAE, 604),
    pin(SAE, 620),
    pin(SAE, 636),
    pin(SAE, 652),
    pin(SAE, 668),
];
/// "SAE Processing Rpt" (ProcessingOnly, no layout): SaveAs_ProcessingOnly_Control and the four error sites.
const PROCESSING: &[Pin] = &[
    pin(SAE, 684),
    pin(SAE, 700),
    pin(SAE, 716),
    pin(SAE, 732),
    pin(SAE, 748),
];
const PNH6: &str = "streams/PnH6SaveAs.al";
const PROCESSING_PDF_WORD: &[Pin] = &[pin(PNH6, 196)];
const PROCESSING_EXCEL_HTML: &[Pin] = &[pin(PNH6, 217)];

/// The facts every row here requires, each as the probe measured it.
fn base(answer: AnswerForm, format: Format) -> Key {
    Key {
        format: Is(format),
        target: Is(Target::Stream),
        call: Is(Call::StaticPlain),
        answer: Is(answer),
        file: Is(LayoutFile::Present),
        data_items: Is(DataItems::One),
        extensions: Is(Extensions::None),
        custom: Is(false),
        company_build: Is(false),
        listed: Is(false),
        init_triggers: Is(false),
        selection: Is(Selection::None),
        stored: Is(Stored::None),
        ..Key::ANY
    }
}

/// An Excel layout as Pdf or Word, as a statement: the statement raises the `ReportUnsupportedFormat` the Boolean form answers
/// false with (`cells::unsupported_rows`), at every site (SaveAsTriggerErrors.al:604-683, the `Pdf stmt` and `Word stmt`
/// calls). A processing-only report: the save goes on to the processing-only answer (`reports/save_processing_only.rs`, no
/// trigger runs), as Pdf does (`cells::policy_rows`). Pinned with the answer used as Word (streams/PnH6SaveAs.al:196
/// Formats_ProcessingOnly_PdfWord), Excel and Html (:217 Formats_ProcessingOnly_ExcelHtmlXml); measured as Word in the
/// statement form (SaveAsTriggerErrors.al:684-763, the `Word stmt` calls).
pub(super) fn platform_first_rows(t: &mut Vec<Cell>) {
    let marks = |p| Evidence::Awaiting(Assumption::SaveasPlatformErrorBeforeTriggers, p);
    for format in [Format::Pdf, Format::Word] {
        let key = Key {
            kind: Is(Kind::Excel),
            layout: Is(Layout::NotRdlc),
            decl: Is(Decl::Rendering),
            declares: Is(Declares::Other),
            processing_only: Is(false),
            sole_layout: Is(true),
            ..base(AnswerForm::Statement, format)
        };
        t.push(cell(
            key,
            Answer::False(ErrorId::UnsupportedFormat),
            Events::NotRaised(UNSUPPORTED_EVENTS),
            marks(EXCEL),
        ));
    }
    for (format, answer, evidence) in [
        (Format::Word, AnswerForm::Used, Evidence::Pinned(PROCESSING_PDF_WORD)),
        (Format::Excel, AnswerForm::Used, Evidence::Pinned(PROCESSING_EXCEL_HTML)),
        (Format::Html, AnswerForm::Used, Evidence::Pinned(PROCESSING_EXCEL_HTML)),
        (Format::Word, AnswerForm::Statement, marks(PROCESSING)),
    ] {
        let key = Key {
            kind: Is(Kind::Other),
            layout: Is(Layout::NotRdlc),
            decl: Is(Decl::Legacy),
            processing_only: Is(true),
            ..base(answer, format)
        };
        t.push(cell(key, Answer::Dataset, Events::NotRaised(&[]), evidence));
    }
}
