//! `Page.RunModal(Id, Rec)` whose handler answers OK on a lookup page that shows no row (an empty view of a list that has no
//! new-row line, handlers/TestPageModalNewRowLine_Tests.al:164 NotEditable_RunModalById_HasNoNewRowLine): the answer is
//! LookupOK, but what Rec holds afterwards is not pinned (the corpus pins only GetRecord of a page variable after OK on an empty
//! table, errors/lt-ms-refusals-129/P129PageMisc.al:633 P129_GetRecord_AfterRunModalOkOnEmptyTable, blank). A later use of
//! Rec is refused. A partner test reached it: the runtime kept Rec as it was, and the test went on with a blank record.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62700 "LNR Row"
{
    LookupPageId = "LNR List";
    fields { field(1; "No."; Code[20]) { } field(2; Grp; Code[10]) { } }
    keys { key(PK; "No.") { } }
}
page 62701 "LNR List"
{
    PageType = List; SourceTable = "LNR Row"; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } } } }
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62702 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

#[test]
fn rec_after_lookup_ok_on_a_page_without_a_row_is_refused() {
    let o = outcomes(
        "[Test] [HandlerFunctions('OkHandler')] procedure EmptyView() var R: Record \"LNR Row\"; begin
             R.DeleteAll();
             R.\"No.\" := 'A'; R.Grp := 'X'; R.Insert();
             R.SetRange(Grp, 'NONE');
             if Page.RunModal(0, R) <> Action::LookupOK then Error('answer');
             if R.\"No.\" <> 'A' then Error('rec');
         end;
         [Test] [HandlerFunctions('OkHandler')] procedure OnARow() var R: Record \"LNR Row\"; begin
             R.DeleteAll();
             R.\"No.\" := 'A'; R.Grp := 'X'; R.Insert();
             R.\"No.\" := 'B'; R.Grp := 'X'; R.Insert();
             R.SetRange(Grp, 'X');
             if Page.RunModal(0, R) <> Action::LookupOK then Error('answer');
             if R.\"No.\" <> 'A' then Error('rec %1', R.\"No.\");
         end;
         [ModalPageHandler] procedure OkHandler(var P: TestPage \"LNR List\") begin P.First(); P.OK().Invoke(); end;",
    );
    assert!(
        matches!(&o[0].1, Outcome::NotRun(why) if why.contains("shows no row")),
        "{:?}",
        o[0]
    );
    assert_eq!(o[1].1, Outcome::Pass, "{:?}", o[1]);
}

// No corpus test pins Rec's filters, key or view after that LookupOK either: their reads refuse too.
#[test]
fn filter_reads_of_rec_after_lookup_ok_on_a_page_without_a_row_are_refused() {
    let o = outcomes(
        "[Test] [HandlerFunctions('OkHandler')] procedure Filters() var R: Record \"LNR Row\"; begin
             R.DeleteAll();
             R.\"No.\" := 'A'; R.Grp := 'X'; R.Insert();
             R.SetRange(Grp, 'NONE');
             if Page.RunModal(0, R) <> Action::LookupOK then Error('answer');
             if R.GetFilters() = '' then Error('filters');
         end;
         [Test] [HandlerFunctions('OkHandler')] procedure View() var R: Record \"LNR Row\"; begin
             R.DeleteAll();
             R.\"No.\" := 'A'; R.Grp := 'X'; R.Insert();
             R.SetRange(Grp, 'NONE');
             if Page.RunModal(0, R) <> Action::LookupOK then Error('answer');
             if R.GetView() = '' then Error('view');
         end;
         [ModalPageHandler] procedure OkHandler(var P: TestPage \"LNR List\") begin P.First(); P.OK().Invoke(); end;",
    );
    for (n, r) in &o {
        assert!(
            matches!(r, Outcome::NotRun(why) if why.contains("shows no row")),
            "{n}: {r:?}"
        );
    }
}
