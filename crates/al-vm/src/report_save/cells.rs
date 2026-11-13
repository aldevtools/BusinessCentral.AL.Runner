//! The rows of the save table (ADR-040 amendment 2026-11-13, part 1): today's cells, moved from the predicates of
//! `report_placeholder` (grid, stored selection, no dataset row, temporary selection) and the rgba list of
//! `al_builtins::rendered_text`. Every row is exact on the facts it reads (`cell::K`); a row family that spans several values
//! is one row per value. A row measured by an open probe pull request marks that probe's assumption
//! (`Evidence::Awaiting`); a row main answered without a measurement says so (`Evidence::Kept`).

use std::sync::LazyLock;

use super::Format;
use super::cell::K::Is;
use super::cell::*;
use super::facts::*;
use al_builtins::assumptions::Assumption;

const fn pin(file: &'static str, line: u32) -> Pin {
    Pin { file, line }
}

const AM: &str = "reports/layouts-saveas/AMReportOutputTests.al";
const CI: &str = "reports/CiReportRunLeads.al";
/// Corpus pull request 1371 (open).
const RX: &str = "reports/html-email-body/RxLayoutSelectionTests.al";
const RX_BODY: &str = "reports/html-email-body/RxEmailBodyTests.al";
const RX_TENANT: &str = "reports/html-email-body/RxTenantLayoutBodyTests.al";
/// Corpus pull request 1362 (open).
const HT: &str = "reports/layouts-html-text/HtRenderedTextTests.al";

/// The placeholders of the 2026-11-11 grid: a Word layout to Pdf, Word and Html, an Excel layout to Excel; RDLC to Word and
/// Excel (CiReportRunLeads.al:674-692).
const GRID_WRITES: &[Pin] = &[
    pin(AM, 173),
    pin(AM, 187),
    pin(AM, 205),
    pin(AM, 235),
    pin(CI, 581),
    pin(CI, 674),
];
/// The `ReportUnsupportedFormat` answers of the grid: Word layout to Excel, Excel layout to Pdf, Word and Html.
const GRID_UNSUPPORTED: &[Pin] = &[
    pin(AM, 221),
    pin(AM, 257),
    pin(CI, 219),
    pin(CI, 267),
    pin(CI, 601),
    pin(CI, 657),
    pin(CI, 662),
];
const RDLC_HTML: &[Pin] = &[pin("text/TestReportSaveAsDatasetContracts.al", 410)];
const NO_LAYOUT_FILE: &[Pin] = &[pin("reports/PRSaveAsFormats.al", 44)];
/// Rows of the selection tables that do not change the default layout: same-kind rows (:403, :428, :501) and rows of another
/// company (:795, :1022).
const SAME_KIND: &[Pin] = &[pin(RX, 403), pin(RX, 428), pin(RX, 501), pin(RX, 795), pin(RX, 1022)];
const STORED_RDLC: &[Pin] = &[pin(RX, 452), pin(RX, 477), pin(RX, 1082), pin(RX, 1116)];
const STORED_NO_LAYOUT: &[Pin] = &[pin(RX, 746), pin(RX, 771)];
const TEMP: &[Pin] = &[
    pin(HT, 494),
    pin(HT, 530),
    pin(RX_TENANT, 121),
    pin(RX_TENANT, 154),
    pin(RX_TENANT, 225),
];
const NO_ROW_WORD: &[Pin] = &[pin(RX, 852), pin(RX, 873), pin(RX, 915), pin(RX_BODY, 134)];
const NO_ROW_LEGACY: &[Pin] = &[pin(RX, 979), pin(RX, 1001)];
const NO_ROW_RDLC: &[Pin] = &[pin(RX, 937), pin(RX, 958)];

/// Events BC does not raise for a format the layout cannot give: SubstituteReport is raised by the caller, OnDocumentReady
/// is not raised (AMReportOutputTests.al:221, :257).
const UNSUPPORTED_EVENTS: &[&str] = &["substitutereport", "ondocumentready", "onafterdocumentready"];

const STATEMENT_UNSUPPORTED: &str = "as a statement, of a report whose layout cannot give the format (the layout events BC \
     raises before its error are not raised here; events/platform-triggers/PeVTests.al:713)";
const FILE_UNSUPPORTED: &str =
    "to a file, of a report whose layout cannot give the format (only the stream form is pinned)";
const ON_POST: &str = "of a Word layout report whose run wrote no dataset row and that declares OnPostReport (BC does not \
     run it then; this runtime has run it before it counts the rows)";

/// The table.
pub(super) static CELLS: LazyLock<Vec<Cell>> = LazyLock::new(cells);

fn cell(key: Key, answer: Answer, events: Events, evidence: Evidence) -> Cell {
    Cell {
        key,
        answer,
        events,
        rgba: false,
        evidence,
    }
}

const SAVE_FORMS: [AnswerForm; 2] = [AnswerForm::Used, AnswerForm::Statement];
const FORMATS: [Format; 4] = [Format::Pdf, Format::Word, Format::Excel, Format::Html];

fn cells() -> Vec<Cell> {
    let mut t = Vec::new();
    policy_rows(&mut t);
    no_valid_layout_rows(&mut t);
    stored_rows(&mut t);
    unsupported_rows(&mut t);
    placeholder_rows(&mut t);
    temp_rows(&mut t);
    rgba_rows(&mut t);
    no_row_rows(&mut t);
    t
}

/// A report with `DefaultLayout` and no layout file: Pdf, Html, Word and Excel answer false (PRSaveAsFormats.al:44). With the
/// answer dropped it is not pinned.
fn no_valid_layout_rows(t: &mut Vec<Cell>) {
    let key = Key {
        file: Is(LayoutFile::Missing),
        answer: Is(AnswerForm::Used),
        ..Key::ANY
    };
    t.push(cell(
        key,
        Answer::False(ErrorId::NoValidLayout),
        Events::NotChecked,
        Evidence::Pinned(NO_LAYOUT_FILE),
    ));
}

/// Owner decisions, not measurements: a layout of Type Custom hands the dataset to the merger (assumption
/// custom-layout-merger); the company build runs SaveAsPdf to a file and writes nothing (owner, 2026-10-22); a processing-only
/// report saved as Pdf goes on to its pinned answer (TestReportSaveAsColumnTypesContracts.al:372-388).
fn policy_rows(t: &mut Vec<Cell>) {
    for answer in SAVE_FORMS {
        let custom = Key {
            custom: Is(true),
            answer: Is(answer),
            file: if answer == AnswerForm::Used {
                Is(LayoutFile::Present)
            } else {
                K::Unread
            },
            ..Key::ANY
        };
        t.push(cell(
            custom,
            Answer::Dataset,
            Events::NotChecked,
            Evidence::Policy("custom-layout-merger"),
        ));
        let pdf = Key {
            format: Is(Format::Pdf),
            custom: Is(false),
            answer: Is(answer),
            file: custom.file,
            ..Key::ANY
        };
        // Every kind the layout cannot give Pdf for refuses first (`unsupported_rows`): an Excel layout that renders.
        for (kind, processing) in [
            (Kind::Rdlc, K::Unread),
            (Kind::Word, K::Unread),
            (Kind::Other, K::Unread),
            (Kind::Excel, Is(true)),
        ] {
            let build = Key {
                company_build: Is(true),
                target: Is(Target::File),
                kind: Is(kind),
                processing_only: processing,
                ..pdf
            };
            t.push(cell(
                build,
                Answer::Discard,
                Events::NotChecked,
                Evidence::Policy("company build, owner 2026-10-22"),
            ));
        }
        let processing = Key {
            processing_only: Is(true),
            ..pdf
        };
        let why = Evidence::Policy("TestReportSaveAsColumnTypesContracts.al:372");
        t.push(cell(
            Key {
                company_build: Is(false),
                ..processing
            },
            Answer::Dataset,
            Events::NotChecked,
            why,
        ));
        t.push(cell(
            Key {
                company_build: Is(true),
                target: Is(Target::Stream),
                ..processing
            },
            Answer::Dataset,
            Events::NotChecked,
            why,
        ));
    }
}

/// A stored Report Layout Selection row of the company that the probe measured (corpus pull request 1371): a Word default
/// layout `DefaultRenderingLayout` names, no report extension, a stream, the answer used. The RDLC row: Html answers false
/// with `ReportUnsupportedFormat`, Pdf, Word and Excel render the RDLC layout (:452, :477, :1082, :1116); with no dataset row
/// it is not measured. The Custom Layout row with an unknown code: every format answers false with `ReportNoLayout` (:746,
/// :771).
fn stored_rows(t: &mut Vec<Cell>) {
    let base = Key {
        answer: Is(AnswerForm::Used),
        target: Is(Target::Stream),
        kind: Is(Kind::Word),
        decl: Is(Decl::Rendering),
        processing_only: Is(false),
        extensions: Is(Extensions::None),
        custom: Is(false),
        file: Is(LayoutFile::Present),
        ..Key::ANY
    };
    let marks = |p| Evidence::Awaiting(Assumption::ReportStoredLayoutSelection, p);
    for format in FORMATS {
        let rdlc = Key {
            format: Is(format),
            stored: Is(Stored::RdlcBuiltin),
            selection: Is(Selection::OtherKind),
            ..base
        };
        if format == Format::Html {
            let key = Key {
                layout: Is(Layout::NotRdlc),
                ..rdlc
            };
            t.push(cell(
                key,
                Answer::False(ErrorId::UnsupportedFormat),
                Events::NotRaised(&[]),
                marks(STORED_RDLC),
            ));
        } else {
            let key = Key {
                rows: Is(Rows::Some),
                ..rdlc
            };
            t.push(cell(
                key,
                Answer::Placeholder(Renders::StoredRdlc),
                Events::NotRaised(&[]),
                marks(STORED_RDLC),
            ));
        }
        let none = Key {
            format: Is(format),
            stored: Is(Stored::CustomUnknownCode),
            selection: Is(Selection::Unknown),
            ..base
        };
        t.push(cell(
            none,
            Answer::False(ErrorId::NoLayout),
            Events::NotRaised(&[]),
            marks(STORED_NO_LAYOUT),
        ));
    }
}

/// The selections under which a format answers as the default layout: none (pinned), a built-in layout of the default kind
/// (corpus pull request 1371, kept), and the built-in layout of the other kind for Pdf and Word, which both kinds give
/// (main's answer, not measured).
fn selections(format: Format, pinned: &'static [Pin]) -> Vec<(Selection, Evidence)> {
    let mut out = vec![
        (Selection::None, Evidence::Pinned(pinned)),
        (Selection::SameKind, Evidence::Kept(SAME_KIND)),
    ];
    if matches!(format, Format::Pdf | Format::Word) {
        out.push((Selection::OtherKind, Evidence::Kept(pinned)));
    }
    out
}

/// A format the default layout cannot give answers false with `ReportUnsupportedFormat` (the 2026-11-11 grid; RDLC to Html:
/// TestReportSaveAsDatasetContracts.al:410, where no subscriber is checked).
fn unsupported_rows(t: &mut Vec<Cell>) {
    let base = Key {
        processing_only: Is(false),
        custom: Is(false),
        ..Key::ANY
    };
    let cells = [
        (Kind::Word, Format::Excel),
        (Kind::Excel, Format::Pdf),
        (Kind::Excel, Format::Word),
        (Kind::Excel, Format::Html),
    ];
    for (kind, format) in cells {
        let cell_key = Key {
            kind: Is(kind),
            format: Is(format),
            layout: if format == Format::Html {
                Is(Layout::NotRdlc)
            } else {
                K::Unread
            },
            ..base
        };
        t.push(refuse_statement(cell_key));
        let file = Key {
            answer: Is(AnswerForm::Used),
            target: Is(Target::File),
            file: Is(LayoutFile::Present),
            ..cell_key
        };
        t.push(cell(
            file,
            Answer::Refuse(FILE_UNSUPPORTED),
            Events::NotChecked,
            Evidence::Kept(GRID_UNSUPPORTED),
        ));
        let stream = Key {
            target: Is(Target::Stream),
            ..file
        };
        let mut sels = vec![(Selection::None, Evidence::Pinned(GRID_UNSUPPORTED))];
        sels.push((Selection::SameKind, Evidence::Kept(SAME_KIND)));
        if kind == Kind::Excel && matches!(format, Format::Pdf | Format::Word) {
            sels.push((Selection::OtherKind, Evidence::Kept(GRID_UNSUPPORTED)));
        }
        for (selection, evidence) in sels {
            let key = Key {
                selection: Is(selection),
                ..stream
            };
            t.push(cell(
                key,
                Answer::False(ErrorId::UnsupportedFormat),
                Events::NotRaised(UNSUPPORTED_EVENTS),
                evidence,
            ));
        }
    }
    let rdlc_html = Key {
        format: Is(Format::Html),
        layout: Is(Layout::Rdlc),
        ..base
    };
    t.push(refuse_statement(rdlc_html));
    for (selection, evidence) in [
        (Selection::None, Evidence::Pinned(RDLC_HTML)),
        (Selection::SameKind, Evidence::Kept(SAME_KIND)),
    ] {
        let key = Key {
            answer: Is(AnswerForm::Used),
            file: Is(LayoutFile::Present),
            selection: Is(selection),
            ..rdlc_html
        };
        t.push(cell(
            key,
            Answer::False(ErrorId::UnsupportedFormat),
            Events::NotChecked,
            evidence,
        ));
    }
}

fn refuse_statement(key: Key) -> Cell {
    let key = Key {
        answer: Is(AnswerForm::Statement),
        ..key
    };
    cell(
        key,
        Answer::Refuse(STATEMENT_UNSUPPORTED),
        Events::NotChecked,
        Evidence::Kept(GRID_UNSUPPORTED),
    )
}

/// The formats each default layout gives as a placeholder (the 2026-11-11 grid).
const WRITES: [(Kind, Format); 7] = [
    (Kind::Rdlc, Format::Pdf),
    (Kind::Rdlc, Format::Word),
    (Kind::Rdlc, Format::Excel),
    (Kind::Word, Format::Pdf),
    (Kind::Word, Format::Word),
    (Kind::Word, Format::Html),
    (Kind::Excel, Format::Excel),
];

/// The key of a placeholder save of `kind` as `format` in answer form `answer`, before the selection.
fn placeholder_keys(kind: Kind, format: Format, answer: AnswerForm) -> Vec<Key> {
    let base = Key {
        kind: Is(kind),
        format: Is(format),
        answer: Is(answer),
        processing_only: Is(false),
        file: Is(LayoutFile::Present),
        custom: Is(false),
        layout: if format == Format::Html {
            Is(Layout::NotRdlc)
        } else {
            K::Unread
        },
        // A Word layout report with no dataset row has its own cells (`no_row_rows`); the request page has no answer.
        rows: if kind == Kind::Word && answer != AnswerForm::None {
            Is(Rows::Some)
        } else {
            K::Unread
        },
        ..Key::ANY
    };
    match (format, answer) {
        // The request page's SaveAsPdf and SaveAsExcel write to a file.
        (Format::Pdf | Format::Excel, AnswerForm::None) => vec![Key {
            target: Is(Target::File),
            ..base
        }],
        (_, AnswerForm::None) => Vec::new(),
        // SaveAsPdf to a file in the company build writes nothing (`policy_rows`).
        (Format::Pdf, _) => vec![
            Key {
                target: Is(Target::Stream),
                ..base
            },
            Key {
                target: Is(Target::File),
                company_build: Is(false),
                ..base
            },
        ],
        _ => vec![base],
    }
}

fn placeholder_rows(t: &mut Vec<Cell>) {
    for (kind, format) in WRITES {
        for answer in [AnswerForm::Used, AnswerForm::Statement, AnswerForm::None] {
            for key in placeholder_keys(kind, format, answer) {
                for (selection, evidence) in selections(format, GRID_WRITES) {
                    let key = Key {
                        selection: Is(selection),
                        ..key
                    };
                    t.push(cell(
                        key,
                        Answer::Placeholder(Renders::Default),
                        Events::NotRaised(&[]),
                        evidence,
                    ));
                }
            }
        }
    }
}

/// The key of an Html save of a Word layout report under a temporary selection (ADR-040 amendment 2026-11-12, A7).
fn temp_key(answer: AnswerForm) -> Key {
    Key {
        format: Is(Format::Html),
        target: Is(Target::Stream),
        answer: Is(answer),
        kind: Is(Kind::Word),
        layout: Is(Layout::NotRdlc),
        extensions: Is(Extensions::None),
        processing_only: Is(false),
        file: Is(LayoutFile::Present),
        custom: Is(false),
        selection: Is(Selection::TempWord),
        rows: Is(Rows::Some),
        ..Key::ANY
    }
}

/// The Html of the user-defined Word layout a temporary selection named (HtRenderedTextTests.al:494, :530;
/// RxTenantLayoutBodyTests.al:121, :154, :225).
fn temp_rows(t: &mut Vec<Cell>) {
    for answer in SAVE_FORMS {
        t.push(cell(
            temp_key(answer),
            Answer::Placeholder(Renders::Selected),
            Events::AtFact,
            Evidence::Awaiting(Assumption::ReportLayoutTempSelection, TEMP),
        ));
    }
}

/// The renders the rgba replace of `Email Message.Create` was measured on (corpus pull request 1371): the Html of the default
/// Word layout `DefaultRenderingLayout` names of a report with one data item and no extension, saved with no selection row
/// naming the report and a dataset without `rgba(` (RxEmailBodyTests.al:86, :110, :158), and the four forms with one fact
/// changed: `DefaultLayout = Word` (RxTenantLayoutBodyTests.al:319), one item inside one (:346), one extension adding columns
/// (:373), the user-defined Word layout a temporary selection named (:121, :154, :225). The save itself is the grid's; the
/// rgba column is that probe's, and the replace marks `email-body-rgba-replace-keeps-content` where it runs
/// (`al_builtins::rendered_text::dotnet_guard`).
fn rgba_rows(t: &mut Vec<Cell>) {
    let measured = |key: Key, data_items, extensions, decl| Key {
        data_items: Is(data_items),
        extensions: Is(extensions),
        decl: Is(decl),
        named: Is(false),
        dataset_rgba: Is(false),
        ..key
    };
    for answer in SAVE_FORMS {
        let [default] = placeholder_keys(Kind::Word, Format::Html, answer)[..] else {
            unreachable!("one key for Html")
        };
        let default = Key {
            selection: Is(Selection::None),
            ..default
        };
        for (data_items, extensions, decl) in [
            (DataItems::One, Extensions::None, Decl::Rendering),
            (DataItems::One, Extensions::None, Decl::Legacy),
            (DataItems::ParentAndChild, Extensions::None, Decl::Rendering),
            (DataItems::One, Extensions::OneAddingColumns, Decl::Rendering),
        ] {
            t.push(Cell {
                rgba: true,
                ..cell(
                    measured(default, data_items, extensions, decl),
                    Answer::Placeholder(Renders::Default),
                    Events::NotRaised(&[]),
                    Evidence::Pinned(GRID_WRITES),
                )
            });
        }
        t.push(Cell {
            rgba: true,
            ..cell(
                measured(temp_key(answer), DataItems::One, Extensions::None, Decl::Rendering),
                Answer::Placeholder(Renders::Selected),
                Events::AtFact,
                Evidence::Awaiting(Assumption::ReportLayoutTempSelection, TEMP),
            )
        });
    }
}

/// A save whose run wrote no dataset row (corpus pull requests 1362 and 1371): one data item, no report extension, a stream,
/// no selection row naming the report. A Word default layout `DefaultRenderingLayout` names: false with `ReportResultEmpty`
/// as Pdf (:852), Word (:873), Html (:915; RxEmailBodyTests.al:134 a report without report triggers); BC runs OnPreReport
/// and OnPreRendering and not OnPostReport, so a report that declares OnPostReport refuses. `DefaultLayout = Word` without
/// report triggers: the same as Html (:979) and Pdf (:1001); with a report trigger the Html save with its answer used keeps
/// main's answer, false with the last error untouched (not measured). An RDLC default layout `DefaultRenderingLayout` names,
/// without report triggers: true, nothing written, as Pdf (:937) and Word (:958).
fn no_row_rows(t: &mut Vec<Cell>) {
    let marks = |p| Evidence::Awaiting(Assumption::ReportSaveNoDatasetRow, p);
    for answer in SAVE_FORMS {
        let base = Key {
            answer: Is(answer),
            target: Is(Target::Stream),
            data_items: Is(DataItems::One),
            extensions: Is(Extensions::None),
            named: Is(false),
            rows: Is(Rows::Zero),
            processing_only: Is(false),
            file: Is(LayoutFile::Present),
            custom: Is(false),
            selection: Is(Selection::None),
            ..Key::ANY
        };
        let of = |kind, decl, format: Format, triggers| Key {
            kind: Is(kind),
            decl: Is(decl),
            format: Is(format),
            triggers: Is(triggers),
            layout: if format == Format::Html {
                Is(Layout::NotRdlc)
            } else {
                K::Unread
            },
            ..base
        };
        for format in [Format::Pdf, Format::Word, Format::Html] {
            for triggers in [Triggers::None, Triggers::WithoutOnPost] {
                let key = of(Kind::Word, Decl::Rendering, format, triggers);
                t.push(cell(key, Answer::Empty, Events::NotRaised(&[]), marks(NO_ROW_WORD)));
            }
            let key = of(Kind::Word, Decl::Rendering, format, Triggers::OnPost);
            t.push(cell(
                key,
                Answer::Refuse(ON_POST),
                Events::NotRaised(&[]),
                Evidence::Kept(NO_ROW_WORD),
            ));
        }
        for format in [Format::Html, Format::Pdf] {
            let key = of(Kind::Word, Decl::Legacy, format, Triggers::None);
            t.push(cell(key, Answer::Empty, Events::NotRaised(&[]), marks(NO_ROW_LEGACY)));
        }
        if answer == AnswerForm::Used {
            for triggers in [Triggers::WithoutOnPost, Triggers::OnPost] {
                let key = of(Kind::Word, Decl::Legacy, Format::Html, triggers);
                t.push(cell(
                    key,
                    Answer::FalseKeepError,
                    Events::NotRaised(&[]),
                    marks(NO_ROW_LEGACY),
                ));
            }
        }
        for format in [Format::Pdf, Format::Word] {
            let key = of(Kind::Rdlc, Decl::Rendering, format, Triggers::None);
            t.push(cell(
                key,
                Answer::TrueNothing,
                Events::NotRaised(&[]),
                marks(NO_ROW_RDLC),
            ));
        }
    }
}
