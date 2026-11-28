//! The rows of the save table for a report declared as report 80040 "RXD Decl Word" is
//! (reports/html-email-body/RxdDeclFixtures.al:31): `WordLayout` and `RDLCLayout` properties with `DefaultLayout = Word`, no
//! rendering section, one data item without a trigger, no report trigger, no report extension, saved to a stream, no
//! subscriber to a layout event. Every cell saved with two dataset rows (`Rows(2)`, RxdDeclFixtures.al:54), so every row reads
//! the run's rows, the false answers too: they answer after the run, and a save with no dataset row refuses then. The false
//! answers refuse before the run when a data item declares a trigger or a column whose source is not a stored field of its
//! table (`refuse_unmeasured_data_items_before_false`). The probe binds no subscriber, so every row refuses a subscriber the test can see to any layout event (SubstituteReport,
//! OnDocumentReady, SelectedBuiltinLayoutType, ...). Measured the same on the three BC majors the corpus runs.

use super::Format;
use super::cell::K::Is;
use super::cell::*;
use super::cells::{SAVE_FORMS, cell, pin};
use super::facts::*;

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
/// the null App ID), with no row of the company or over one that selects "RDLC (built-in)" or "Word (built-in)" (BC reads no
/// row of the company then): the selected layout renders Html, Word and Pdf; Excel answers false with
/// `ReportUnsupportedFormat`, and its statement raises it.
const TEMP: &[Pin] = &[
    pin(RXD, 313),
    pin(RXD, 340),
    pin(RXD, 367),
    pin(RXD, 391),
    pin(RXD, 811),
    pin(RXD, 1021),
    pin(RXD, 1336),
    pin(RXD, 1651),
];
/// The rgba replace of `Email Message.Create` on the Html of the selected layout (GetBody holds `rgb(1, 2, 3)` for the
/// layout's `rgba(1,2,3,1)`), with no row of the company and over one.
const RGBA_TEMP: &[Pin] = &[
    pin(RXD_BODY, 69),
    pin(RXD_BODY, 199),
    pin(RXD_BODY, 251),
    pin(RXD_BODY, 329),
    pin(RXD_BODY, 407),
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
        sole_layout: K::Unread,
        own_layout: K::Unread,
        init_triggers: K::Unread,
        custom: Is(false),
        selection: Is(selection),
        stored: Is(stored),
        ..Key::ANY
    }
}

pub(super) fn decl_rows(t: &mut Vec<Cell>) {
    for answer in SAVE_FORMS {
        stored_rdlc_rows(t, answer);
        temp_rows(t, answer);
    }
    let key = Key {
        rows: Is(Rows::Some),
        named: Is(false),
        dataset_rgba: Is(false),
        ..base(AnswerForm::Used, Format::Html, Selection::TempWord, Stored::None)
    };
    t.push(Cell {
        rgba: true,
        ..cell(
            key,
            Answer::Placeholder(Renders::Selected),
            Events::NotRaised(&[]),
            Evidence::Pinned(RGBA_TEMP),
        )
    });
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
            Evidence::Pinned(STORED_RDLC),
        ));
    }
    let key = Key {
        rows: Is(Rows::Some),
        ..base(answer, Format::Html, Selection::OtherKind, Stored::RdlcBuiltin)
    };
    t.push(cell(
        key,
        Answer::FalseAfterRun(ErrorId::UnsupportedFormat),
        Events::NotRaised(&[]),
        Evidence::Pinned(STORED_RDLC),
    ));
}

fn temp_rows(t: &mut Vec<Cell>, answer: AnswerForm) {
    for format in [Format::Pdf, Format::Word, Format::Html] {
        let key = Key {
            rows: Is(Rows::Some),
            ..base(answer, format, Selection::TempWord, Stored::None)
        };
        t.push(cell(
            key,
            Answer::Placeholder(Renders::Selected),
            Events::NotRaised(&[]),
            Evidence::Pinned(TEMP),
        ));
    }
    let key = Key {
        rows: Is(Rows::Some),
        ..base(answer, Format::Excel, Selection::TempWord, Stored::None)
    };
    t.push(cell(
        key,
        Answer::FalseAfterRun(ErrorId::UnsupportedFormat),
        Events::NotRaised(&[]),
        Evidence::Pinned(TEMP),
    ));
}
