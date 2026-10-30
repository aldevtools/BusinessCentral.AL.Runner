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
page 62703 "LNR Plain List"
{
    PageType = List; SourceTable = "LNR Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } field(GrpCtl; Rec.Grp) { } } } }
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

// errors/lt-ms-refusals-129/P129PageMisc.al:726 TV_GetRecord_AfterLookupOk_FilteredToNoRow and :772
// TV_GetRecord_AfterSetRecordAndLookupOk_FilteredToNoRow: a list page variable with LookupMode(true) whose view shows no row has
// no OK action (TestActionNotFound); Cancel closes it with LookupCancel.
#[test]
fn ok_of_a_lookup_mode_list_variable_that_shows_no_row_is_not_found() {
    let o = outcomes(
        "[Test] [HandlerFunctions('TryOkHandler')] procedure NoRow() var R: Record \"LNR Row\"; Pg: Page \"LNR List\"; begin
             R.DeleteAll();
             R.\"No.\" := 'A'; R.Grp := 'X'; R.Insert();
             R.SetRange(Grp, 'NONE');
             Pg.SetTableView(R);
             Pg.LookupMode(true);
             if Pg.RunModal() <> Action::LookupCancel then Error('answer');
         end;
         [ModalPageHandler] procedure TryOkHandler(var P: TestPage \"LNR List\") begin
             asserterror P.OK().Invoke();
             if GetLastErrorText() <> 'The built-in action = OK is not found on the page.' then Error('text %1', GetLastErrorText());
             if GetLastErrorCode() <> 'TestActionNotFound' then Error('code %1', GetLastErrorCode());
             P.Cancel().Invoke();
         end;",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0]);
}

// errors/lt-ms-refusals-129/P129PageMisc.al:675-818 (writer TV, corpus master 44b64f47): a Page variable of a list with no Editable
// property, filtered with SetTableView to no row of a table that has rows. In lookup mode the handler's OK is not found
// (TestActionNotFound) and RunModal answers LookupCancel; GetRecord afterwards hands back the blank record whatever the record held,
// also after SetRecord of a row, after a plain OK and after Cancel.
#[test]
fn get_record_of_a_page_closed_on_no_row_of_a_table_with_rows_is_blank() {
    let o = outcomes(
        "local procedure Seed() var R: Record \"LNR Row\"; begin
             R.DeleteAll(); R.\"No.\" := 'A'; R.Grp := 'X'; R.Insert(); R.\"No.\" := 'B'; R.Grp := 'Y'; R.Insert();
         end;
         local procedure After(var Pg: Page \"LNR Plain List\"): Text var R: Record \"LNR Row\"; begin
             R.\"No.\" := 'Q'; R.Grp := 'Q'; Pg.GetRecord(R); exit('no=' + R.\"No.\" + '|grp=' + R.Grp);
         end;
         local procedure NoRow(var Pg: Page \"LNR Plain List\") var R: Record \"LNR Row\"; begin
             R.SetRange(Grp, 'NONE'); Pg.SetTableView(R);
         end;
         [Test] [HandlerFunctions('PlainOkH')] procedure PlainOk() var Pg: Page \"LNR Plain List\"; begin
             Seed(); NoRow(Pg);
             if Pg.RunModal() <> Action::OK then Error('answer');
             if After(Pg) <> 'no=|grp=' then Error('got %1', After(Pg));
         end;
         [Test] [HandlerFunctions('LookupOkH')] procedure LookupOkAfterSetRecord() var Pg: Page \"LNR Plain List\"; R: Record \"LNR Row\"; begin
             Seed(); R.Get('B'); Pg.SetRecord(R); NoRow(Pg); Pg.LookupMode(true);
             if Pg.RunModal() <> Action::LookupCancel then Error('answer');
             if Seen <> 'TestActionNotFound' then Error('seen %1', Seen);
             if After(Pg) <> 'no=|grp=' then Error('got %1', After(Pg));
         end;
         [ModalPageHandler] procedure PlainOkH(var P: TestPage \"LNR Plain List\") begin P.OK().Invoke(); end;
         [ModalPageHandler] procedure LookupOkH(var P: TestPage \"LNR Plain List\") begin
             if P.First() then Error('row');
             asserterror P.OK().Invoke(); Seen := GetLastErrorCode(); P.Cancel().Invoke();
         end;
         var Seen: Text;",
    );
    for (n, r) in &o {
        assert_eq!(*r, Outcome::Pass, "{n}");
    }
}

// pages/lt-modal-handler-typed-page/PageTypedHandlerRec.al:214-266 (PageVariable_LookupMode_HandlerSetRecord_LookupOK, _Cancel):
// a Page-typed handler that calls SetRecord leaves that record on the page; GetRecord afterwards hands it back, for LookupOK and
// Cancel. A view the test set with SetTableView does not bound the page's Rec (PageTypedHandlerRec.al:116
// SettableView_ThenAssignRecOutsideView), so the same holds after SetTableView.
#[test]
fn get_record_after_a_typed_handler_set_the_record() {
    let o = outcomes(
        "local procedure Seed() var R: Record \"LNR Row\"; begin
             R.DeleteAll(); R.\"No.\" := 'A'; R.Grp := 'X'; R.Insert(); R.\"No.\" := 'B'; R.Grp := 'X'; R.Insert();
         end;
         [Test] [HandlerFunctions('SetB')] procedure LookupOk() var Pg: Page \"LNR Plain List\"; R: Record \"LNR Row\"; begin
             Seed(); R.Get('A'); Reply := Action::LookupOK; Pg.LookupMode(true); Pg.SetRecord(R);
             if Pg.RunModal() <> Action::LookupOK then Error('answer');
             Clear(R); Pg.GetRecord(R); if R.\"No.\" <> 'B' then Error('got %1', R.\"No.\");
         end;
         [Test] [HandlerFunctions('SetB')] procedure CancelAfterView() var Pg: Page \"LNR Plain List\"; R: Record \"LNR Row\"; begin
             Seed(); Reply := Action::Cancel; R.SetRange(Grp, 'X'); Pg.SetTableView(R); Pg.LookupMode(true);
             if Pg.RunModal() <> Action::Cancel then Error('answer');
             Clear(R); Pg.GetRecord(R); if R.\"No.\" <> 'B' then Error('got %1', R.\"No.\");
         end;
         [ModalPageHandler] procedure SetB(var P: Page \"LNR Plain List\"; var Response: Action) var R: Record \"LNR Row\"; begin
             R.Get('B'); P.SetRecord(R); Response := Reply;
         end;
         var Reply: Action;",
    );
    for (n, r) in &o {
        assert_eq!(*r, Outcome::Pass, "{n}");
    }
}
