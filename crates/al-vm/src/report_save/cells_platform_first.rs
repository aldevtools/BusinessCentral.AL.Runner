//! The rows of the save table where the platform's own check of a `Report.SaveAs` comes before any trigger (corpus pull
//! request 1579, merged, not yet pinned; assumption `saveas-platform-error-before-triggers`, issue 8486). The probe's reports
//! (reports/layouts-saveas/SaveAsTriggerErrors.al) have one data item over Integer and no report extension, and are saved by
//! `Report.SaveAs(Id, '', Format, OutStream)` with no layout selection row, as a statement inside `Codeunit.Run` and in the
//! Boolean form; each test runs the control and an error in OnPreReport, OnAfterGetRecord, OnPreDataItem and OnPostReport,
//! and no error site is reached. Measured the same on BC 27.5, 28.5 and 29.0.

use super::Format;
use super::cell::K::Is;
use super::cell::*;
use super::cells::{SAVE_FORMS, UNSUPPORTED_EVENTS, cell, pin};
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
        selection: Is(Selection::None),
        stored: Is(Stored::None),
        ..Key::ANY
    }
}

/// An Excel layout as Pdf or Word, as a statement: the statement raises the `ReportUnsupportedFormat` the Boolean form answers
/// false with (`cells::unsupported_rows`), at every site (SaveAsTriggerErrors.al:604-683, the `Pdf stmt` and `Word stmt`
/// calls). A processing-only report as Word, both forms: the save goes on to the processing-only answer
/// (`reports/save_processing_only.rs`, no trigger runs), as Pdf does (`cells::policy_rows`; :684-763, the `Word` calls).
pub(super) fn platform_first_rows(t: &mut Vec<Cell>) {
    let marks = |p| Evidence::Awaiting(Assumption::SaveasPlatformErrorBeforeTriggers, p);
    for format in [Format::Pdf, Format::Word] {
        let key = Key {
            kind: Is(Kind::Excel),
            layout: Is(Layout::NotRdlc),
            decl: Is(Decl::Rendering),
            declares: Is(Declares::Other),
            processing_only: Is(false),
            ..base(AnswerForm::Statement, format)
        };
        t.push(cell(
            key,
            Answer::False(ErrorId::UnsupportedFormat),
            Events::NotRaised(UNSUPPORTED_EVENTS),
            marks(EXCEL),
        ));
    }
    for answer in SAVE_FORMS {
        let key = Key {
            kind: Is(Kind::Other),
            layout: Is(Layout::NotRdlc),
            decl: Is(Decl::Legacy),
            processing_only: Is(true),
            ..base(answer, Format::Word)
        };
        t.push(cell(key, Answer::Dataset, Events::NotRaised(&[]), marks(PROCESSING)));
    }
}
