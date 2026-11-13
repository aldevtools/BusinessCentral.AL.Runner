//! The rows of the save table for a report declared as report 80040 "RXD Decl Word" is (corpus pull request 1381, branch
//! al-runtime-rs-staging-reports-decl, open; reports/html-email-body/RxdDeclFixtures.al): `WordLayout` and `RDLCLayout`
//! properties with `DefaultLayout = Word`, no rendering section, one data item, no report trigger, no report extension, saved to
//! a stream with dataset rows. Measured on BC 27.5, 28.5 and 29.0 with the same values. Every row marks
//! `report-save-decl-word-rdlc` until a corpus bump pins the file.

use super::Format;
use super::cell::K::Is;
use super::cell::*;
use super::cells::{SAVE_FORMS, UNSUPPORTED_EVENTS, cell, pin};
use super::facts::*;
use al_builtins::assumptions::Assumption;

const RXD: &str = "reports/html-email-body/RxdDeclLayoutTests.al";
const RXD_BODY: &str = "reports/html-email-body/RxdDeclEmailBodyTests.al";

/// A Report Layout Selection row of the company that selects "RDLC (built-in)", alone or with a row of an empty Company Name,
/// with or without `SetTempLayoutSelected('')`: the RDLC layout renders Pdf, Word and Excel; Html answers false with
/// `ReportUnsupportedFormat`, and its statement raises it.
const STORED_RDLC: &[Pin] = &[
    pin(RXD, 130),
    pin(RXD, 177),
    pin(RXD, 219),
    pin(RXD, 266),
    pin(RXD, 706),
    pin(RXD, 1231),
    pin(RXD, 1441),
];
/// A temporary selection of a user-defined Word layout (`SetTempLayoutSelectedName` of a GUID-named Tenant Report Layout row,
/// the null App ID) with no row of the company: the selected layout renders Html, Word and Pdf; Excel answers false with
/// `ReportUnsupportedFormat`, and its statement raises it.
const TEMP: &[Pin] = &[
    pin(RXD, 313),
    pin(RXD, 340),
    pin(RXD, 367),
    pin(RXD, 391),
    pin(RXD, 1021),
];
/// The same over a row of the company that selects "RDLC (built-in)" or "Word (built-in)".
const TEMP_OVER_STORED: &[Pin] = &[pin(RXD, 811), pin(RXD, 1336), pin(RXD, 1651)];
/// The rgba replace of `Email Message.Create` on the Html of the selected layout (GetBody holds `rgb(1, 2, 3)` for the
/// layout's `rgba(1,2,3,1)`), with no row of the company and over one.
const RGBA_TEMP: &[Pin] = &[pin(RXD_BODY, 69), pin(RXD_BODY, 251)];
const RGBA_TEMP_OVER_STORED: &[Pin] = &[pin(RXD_BODY, 199), pin(RXD_BODY, 329), pin(RXD_BODY, 407)];

/// BC raises no SelectedBuiltinLayoutType once SelectReportLayoutCode answered a selected layout (HtRenderedTextTests.al:494).
const TEMP_EVENTS: &[&str] = &["selectedbuiltinlayouttype"];
const TEMP_UNSUPPORTED_EVENTS: &[&str] = &[
    "substitutereport",
    "ondocumentready",
    "onafterdocumentready",
    "selectedbuiltinlayouttype",
];

/// The facts every row here requires, as the probe measured them.
fn base(answer: AnswerForm, format: Format, selection: Selection, stored: Stored) -> Key {
    Key {
        format: Is(format),
        target: Is(Target::Stream),
        answer: Is(answer),
        kind: Is(Kind::Word),
        layout: Is(Layout::NotRdlc),
        decl: Is(Decl::Legacy),
        declares: Is(Declares::WordAndRdlcProperties),
        file: Is(LayoutFile::Present),
        processing_only: Is(false),
        data_items: Is(DataItems::One),
        extensions: Is(Extensions::None),
        triggers: Is(Triggers::None),
        custom: Is(false),
        selection: Is(selection),
        stored: Is(stored),
        ..Key::ANY
    }
}

fn marks(p: &'static [Pin]) -> Evidence {
    Evidence::Awaiting(Assumption::ReportSaveDeclWordRdlc, p)
}

pub(super) fn decl_rows(t: &mut Vec<Cell>) {
    for answer in SAVE_FORMS {
        stored_rdlc_rows(t, answer);
        for (selection, pins) in [
            (Selection::TempWord, TEMP),
            (Selection::TempWordOverStored, TEMP_OVER_STORED),
        ] {
            temp_rows(t, answer, selection, pins);
        }
    }
    for (selection, named, pins) in [
        (Selection::TempWord, false, RGBA_TEMP),
        (Selection::TempWordOverStored, true, RGBA_TEMP_OVER_STORED),
    ] {
        let key = Key {
            rows: Is(Rows::Some),
            named: Is(named),
            dataset_rgba: Is(false),
            ..base(AnswerForm::Used, Format::Html, selection, Stored::None)
        };
        t.push(Cell {
            rgba: true,
            ..cell(
                key,
                Answer::Placeholder(Renders::Selected),
                Events::NotRaised(TEMP_EVENTS),
                marks(pins),
            )
        });
    }
}

fn stored_rdlc_rows(t: &mut Vec<Cell>, answer: AnswerForm) {
    for format in [Format::Pdf, Format::Word, Format::Excel] {
        let key = Key {
            rows: Is(Rows::Some),
            ..base(answer, format, Selection::OtherKind, Stored::RdlcBuiltin)
        };
        t.push(cell(
            key,
            Answer::Placeholder(Renders::StoredRdlc),
            Events::NotRaised(&[]),
            marks(STORED_RDLC),
        ));
    }
    let key = base(answer, Format::Html, Selection::OtherKind, Stored::RdlcBuiltin);
    t.push(cell(
        key,
        Answer::False(ErrorId::UnsupportedFormat),
        Events::NotRaised(UNSUPPORTED_EVENTS),
        marks(STORED_RDLC),
    ));
}

fn temp_rows(t: &mut Vec<Cell>, answer: AnswerForm, selection: Selection, pins: &'static [Pin]) {
    for format in [Format::Pdf, Format::Word, Format::Html] {
        let key = Key {
            rows: Is(Rows::Some),
            ..base(answer, format, selection, Stored::None)
        };
        t.push(cell(
            key,
            Answer::Placeholder(Renders::Selected),
            Events::NotRaised(TEMP_EVENTS),
            marks(pins),
        ));
    }
    let key = base(answer, Format::Excel, selection, Stored::None);
    t.push(cell(
        key,
        Answer::False(ErrorId::UnsupportedFormat),
        Events::NotRaised(TEMP_UNSUPPORTED_EVENTS),
        marks(pins),
    ));
}
