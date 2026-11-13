//! Checks of the table: rows that overlap nest, a row that answers before the run wins over every row it overlaps, the
//! lookup finds one row per vector, and the refusal names the facts.

use super::Format;
use super::cell::{Answer, Cell, Evidence, Renders};
use super::cells::CELLS;
use super::facts::*;
use super::lookup::{self, Before, overlap, reads, refines};

#[test]
fn report_save_rows_that_overlap_nest() {
    for (i, a) in CELLS.iter().enumerate() {
        for b in &CELLS[i + 1..] {
            if overlap(&a.key, &b.key) {
                assert!(
                    refines(&a.key, &b.key) || refines(&b.key, &a.key),
                    "rows overlap without nesting:\n{a:?}\n{b:?}"
                );
                assert!(a.key != b.key, "two rows with one key:\n{a:?}\n{b:?}");
            }
        }
    }
}

#[test]
fn report_save_a_row_answered_before_the_run_wins_over_every_row_it_overlaps() {
    let before = |c: &Cell| c.answer.before_run() && !Fact::ALL.iter().any(|f| f.after_run() && reads(&c.key, *f));
    for a in CELLS.iter().filter(|c| before(c)) {
        for b in CELLS.iter().filter(|b| b.key != a.key && overlap(&a.key, &b.key)) {
            assert!(
                refines(&a.key, &b.key),
                "a row that answers before the run is overlapped by a more specific row:\n{a:?}\n{b:?}"
            );
        }
    }
    // A row that answers after the run reads... every row that answers false, refuses or takes a policy without a run reads
    // no run fact, except the refusal of OnPostReport after a run with no row.
    for c in CELLS.iter().filter(|c| c.answer.before_run() && !before(c)) {
        assert!(matches!(c.answer, Answer::Refuse(_)), "{c:?}");
    }
}

#[test]
fn report_save_every_row_cites_its_evidence() {
    for c in CELLS.iter() {
        match c.evidence {
            Evidence::Pinned(p) | Evidence::Awaiting(_, p) | Evidence::Kept(p) => assert!(!p.is_empty(), "{c:?}"),
            Evidence::Policy(why) => assert!(!why.is_empty(), "{c:?}"),
        }
        assert!(
            !c.rgba
                || c.answer == Answer::Placeholder(Renders::Default)
                || c.answer == Answer::Placeholder(Renders::Selected)
        );
    }
}

fn base() -> Facts {
    Facts {
        format: Format::Html,
        target: Target::Stream,
        answer: AnswerForm::Used,
        kind: Kind::Word,
        layout: Layout::NotRdlc,
        decl: Decl::Rendering,
        file: LayoutFile::Present,
        processing_only: false,
        data_items: DataItems::One,
        extensions: Extensions::None,
        triggers: Triggers::None,
        custom: false,
        company_build: false,
        listed: false,
        selection: Selection::None,
        stored: Stored::None,
        named: None,
        rows: None,
        dataset_rgba: None,
    }
}

#[test]
fn report_save_the_measured_html_of_a_word_layout_runs_and_takes_the_rgba_replace() {
    let f = base();
    assert!(matches!(lookup::before_run(&CELLS, &f), Before::Run(_)));
    let after = Facts {
        rows: Some(Rows::Some),
        named: Some(false),
        dataset_rgba: Some(false),
        ..f
    };
    let row = lookup::after_run(&CELLS, &after).expect("a row");
    assert_eq!(row.answer, Answer::Placeholder(Renders::Default));
    assert!(row.rgba);
    // Three data items: the save writes, the rgba replace is not measured on it.
    let three = Facts {
        data_items: DataItems::Other,
        ..after
    };
    assert!(!lookup::after_run(&CELLS, &three).expect("a row").rgba);
}

#[test]
fn report_save_a_miss_names_the_fact_no_row_has_and_the_nearest_rows() {
    // An Html save of a Word layout under a design-time selection: the selection value is never measured for Html.
    let f = Facts {
        selection: Selection::DesignTimeHeld,
        ..base()
    };
    let Before::Miss(miss) = lookup::before_run(&CELLS, &f) else {
        panic!("a miss")
    };
    let why = miss.reason("Report.SaveAs as Html of report \"X\"");
    assert!(
        why.contains("Never measured: layout selection = a design-time selection (measured: "),
        "{why}"
    );
    // A Word layout report with no dataset row and three data items: the rows with no dataset row all read the data items.
    let f = Facts {
        format: Format::Pdf,
        data_items: DataItems::Other,
        rows: Some(Rows::Zero),
        named: Some(false),
        dataset_rgba: Some(false),
        ..base()
    };
    let miss = lookup::after_run(&CELLS, &f).expect_err("a miss");
    let why = miss.reason("Report.SaveAs as Pdf of report \"X\"");
    assert!(
        why.contains("Never measured: data items = another tree (measured: one)"),
        "{why}"
    );
    // A `DefaultLayout = Word` report with OnPreReport saved as Pdf with no dataset row: each value is measured, not
    // together, so the nearest rows are named with the facts that differ and their corpus lines.
    let f = Facts {
        data_items: DataItems::One,
        decl: Decl::Legacy,
        triggers: Triggers::WithoutOnPost,
        ..f
    };
    let miss = lookup::after_run(&CELLS, &f).expect_err("a miss");
    let why = miss.reason("Report.SaveAs as Pdf of report \"X\"");
    assert!(why.contains("Nearest measured rows differ in: "), "{why}");
    assert!(
        why.contains(
            "report triggers without OnPostReport (row: none) [reports/html-email-body/RxLayoutSelectionTests.al:979]"
        ),
        "{why}"
    );
}

#[test]
fn report_save_a_save_the_table_knows_nothing_of_is_the_allow_lists() {
    // A layout of a kind no row has (custom) as Pdf: no row is about it.
    let f = Facts {
        kind: Kind::Other,
        format: Format::Pdf,
        listed: true,
        ..base()
    };
    assert!(matches!(lookup::before_run(&CELLS, &f), Before::Miss(_)));
    assert!(!lookup::knows(&CELLS, &f));
    // A Word layout as Pdf under an unknown selection: the table has rows about the save, so the allow-list does not apply.
    let f = Facts {
        kind: Kind::Word,
        selection: Selection::Unknown,
        ..f
    };
    assert!(matches!(lookup::before_run(&CELLS, &f), Before::Miss(_)));
    assert!(lookup::knows(&CELLS, &f));
}

#[test]
fn report_save_the_request_page_form_answers_only_after_a_run() {
    use super::cell::K;
    for c in CELLS.iter() {
        if matches!(c.key.answer, K::Is(AnswerForm::None) | K::Unread) {
            assert!(matches!(c.answer, Answer::Placeholder(_)), "{c:?}");
        }
    }
}
