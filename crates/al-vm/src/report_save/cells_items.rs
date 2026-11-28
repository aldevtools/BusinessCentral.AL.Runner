//! The rows of the save table for a report with two or three top-level data items whose run wrote no dataset row (corpus pull
//! request 1508, merged, not yet pinned, the answer used; corpus pull request 1513, open, the statement; assumption
//! `report-save-top-level-items-no-row`). The probes' reports (reports/saveas-empty-items/SaeFixtures.al) declare their default layout with `DefaultRenderingLayout`, no report trigger and no report extension, and
//! are saved by `Report.SaveAs(Id, '', Format, OutStream)` with no layout selection row. Both measured the same on BC 27.5, 28.5
//! and 29.0. The probes' labels and request-page reports (SaeStatementTests.al:185-244, Pdf, two items) are not told apart.

use super::Format;
use super::cell::K::Is;
use super::cell::*;
use super::cells::{cell, pin};
use super::facts::*;
use al_builtins::assumptions::Assumption;

const TWO: &str = "reports/saveas-empty-items/SaeTwoItemsTests.al";
const THREE: &str = "reports/saveas-empty-items/SaeThreeItemsTests.al";
const BREAK: &str = "reports/saveas-empty-items/SaeBreakTests.al";
const STMT: &str = "reports/saveas-empty-items/SaeStatementTests.al";
const RDLC: &str = "reports/saveas-empty-items/SaeRdlcTwoTests.al";

/// A Word layout, two items with no row: `Word_EE` (both tables empty), `Word_BreakBoth` (both items stop in OnPreDataItem).
const WORD_TWO: &[Pin] = &[pin(TWO, 18), pin(BREAK, 18)];
/// A Word layout, three items with no row: `Word_EEE`.
const WORD_THREE: &[Pin] = &[pin(THREE, 18)];
/// The statement form, a Word layout, two items with no row: `Word_EE_Statement`, `Word_BreakBoth_Statement`.
const WORD_TWO_STMT: &[Pin] = &[pin(STMT, 19), pin(STMT, 109)];
/// The statement form, a Word layout, three items with no row: `Word_EEE_Statement`.
const WORD_THREE_STMT: &[Pin] = &[pin(STMT, 49)];
/// An RDLC layout, two items with no row: `Rdlc_EE`.
const RDLC_TWO: &[Pin] = &[pin(RDLC, 18)];

/// The facts every row here requires, each as the probe measured it.
fn base(answer: AnswerForm, format: Format, kind: Kind, data_items: DataItems) -> Key {
    Key {
        format: Is(format),
        target: Is(Target::Stream),
        call: Is(Call::StaticPlain),
        answer: Is(answer),
        kind: Is(kind),
        layout: Is(if kind == Kind::Rdlc {
            Layout::Rdlc
        } else {
            Layout::NotRdlc
        }),
        decl: Is(Decl::Rendering),
        declares: Is(Declares::Other),
        file: Is(LayoutFile::Present),
        processing_only: Is(false),
        data_items: Is(data_items),
        extensions: Is(Extensions::None),
        triggers: Is(Triggers::None),
        sole_layout: K::Unread,
        own_layout: K::Unread,
        init_triggers: K::Unread,
        custom: Is(false),
        company_build: Is(false),
        listed: Is(false),
        selection: Is(Selection::None),
        stored: Is(Stored::None),
        named: Is(false),
        rows: Is(Rows::Zero),
        dataset_rgba: Is(false),
    }
}

/// A Word layout as Pdf, Word or Html, the answer used: false with `ReportResultEmpty`, nothing written (SaeTwoItemsTests.al:18,
/// SaeBreakTests.al:18, SaeThreeItemsTests.al:18). As a statement the same three formats of two and of three items: the
/// error is raised at the statement (SaeStatementTests.al:19, :49, :109), the shape of al-runner issue 8668's repro. An RDLC layout as Pdf or Word, the answer used: true, nothing written
/// (SaeRdlcTwoTests.al:18); its Html answers false before the run (`cells::unsupported_rows`).
pub(super) fn top_level_items_rows(t: &mut Vec<Cell>) {
    let marks = |p| Evidence::Awaiting(Assumption::ReportSaveTopLevelItemsNoRow, p);
    for (data_items, used, statement) in [
        (DataItems::TwoTopLevel, WORD_TWO, WORD_TWO_STMT),
        (DataItems::ThreeTopLevel, WORD_THREE, WORD_THREE_STMT),
    ] {
        for format in [Format::Pdf, Format::Word, Format::Html] {
            for (form, pins) in [(AnswerForm::Used, used), (AnswerForm::Statement, statement)] {
                let key = base(form, format, Kind::Word, data_items);
                t.push(cell(key, Answer::Empty, Events::NotRaised(&[]), marks(pins)));
            }
        }
    }
    for format in [Format::Pdf, Format::Word] {
        let key = base(AnswerForm::Used, format, Kind::Rdlc, DataItems::TwoTopLevel);
        t.push(cell(key, Answer::TrueNothing, Events::NotRaised(&[]), marks(RDLC_TWO)));
    }
}
