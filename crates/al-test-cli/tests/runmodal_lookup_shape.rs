//! Page.RunModal(Id, Rec) and Page.RunModal(0, Rec): whether the page opens in lookup mode depends on whether the
//! caller uses the return value. Each test names the corpus test it mirrors
//! (pages/lt-be-runmodal-onlookup/BeRunModalOnLookup.al, codeunit 81162).

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62600 "LS Row"
{
    LookupPageId = "LS List";
    fields { field(1; "Code"; Code[10]) { } }
    keys { key(PK; "Code") { } }
}
page 62601 "LS List"
{
    PageType = List; SourceTable = "LS Row";
    layout { area(Content) { repeater(R) { field(CodeCtl; Rec."Code") { } } } }
    trigger OnOpenPage() var L: Codeunit "LS Log"; begin L.Add('lookupMode=' + Format(CurrPage.LookupMode)); end;
}
codeunit 62602 "LS Log"
{
    SingleInstance = true;
    var Trail: Text;
    procedure Add(T: Text) begin if Trail <> '' then Trail += ','; Trail += T; end;
    procedure Take() R: Text begin R := Trail; Trail := ''; end;
}
"#;

const HELPERS: &str = r#"
    var Log: Codeunit "LS Log";
    procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
    local procedure Seed()
    var R: Record "LS Row";
    begin
        R.DeleteAll();
        R."Code" := 'A'; R.Insert();
        R."Code" := 'B'; R.Insert();
        Log.Take();
    end;
    [ModalPageHandler] procedure WalkOnly(var P: TestPage "LS List")
    var Seen: Text; Steps: Integer;
    begin
        Log.Add('editable=' + Format(P.Editable()));
        if P.First() then
            repeat Seen += '[' + P.CodeCtl.Value() + ']'; Steps += 1; until (not P.Next()) or (Steps > 5);
        Log.Add('rows=' + Seen);
    end;
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62690 T {{ Subtype = Test; {HELPERS} {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

// BeRunModalOnLookup.al:616-629 Direct_NoLookupPageId_KeyFilter_WalkOnly: the return value used opens the list in lookup
// mode, with no new-row line.
#[test]
fn run_modal_by_id_with_the_return_value_used_is_a_lookup_without_the_new_row_line() {
    passes(
        "[Test] [HandlerFunctions('WalkOnly')]
        procedure Used() var R: Record \"LS Row\"; begin
            Seed(); Log.Add('action=' + Format(Page.RunModal(Page::\"LS List\", R)));
            Check('lookupMode=Yes,editable=No,rows=[A][B],action=LookupCancel', Log.Take()); end;",
    );
}

// BeRunModalOnLookup.al:916-929 Shape_ReturnNotUsed_WalkOnly: called as a statement, the list is not a lookup: editable,
// with the new-row line.
#[test]
fn run_modal_by_id_called_as_a_statement_is_not_a_lookup() {
    passes(
        "[Test] [HandlerFunctions('WalkOnly')]
        procedure Statement() var R: Record \"LS Row\"; begin
            Seed(); Page.RunModal(Page::\"LS List\", R);
            Check('lookupMode=No,editable=Yes,rows=[A][B][]', Log.Take()); end;",
    );
}

// BeRunModalOnLookup.al:949-966 Shape_ReturnInIf_WalkOnly: a comparison in an if uses the return value.
#[test]
fn run_modal_by_id_compared_in_an_if_is_a_lookup() {
    passes(
        "[Test] [HandlerFunctions('WalkOnly')]
        procedure InIf() var R: Record \"LS Row\"; begin
            Seed();
            if Page.RunModal(Page::\"LS List\", R) = Action::LookupOK then Log.Add('if=LookupOK') else Log.Add('if=other');
            Check('lookupMode=Yes,editable=No,rows=[A][B],if=other', Log.Take()); end;",
    );
}

// BeRunModalOnLookup.al:1100-1126 Shape_RunModalZero_ReturnUsed_WalkOnly and Shape_RunModalZero_ReturnNotUsed_WalkOnly.
#[test]
fn run_modal_zero_follows_the_same_rule() {
    passes(
        "[Test] [HandlerFunctions('WalkOnly')]
        procedure ZeroUsed() var R: Record \"LS Row\"; begin
            Seed(); Log.Add('action=' + Format(Page.RunModal(0, R)));
            Check('lookupMode=Yes,editable=No,rows=[A][B],action=LookupCancel', Log.Take()); end;
        [Test] [HandlerFunctions('WalkOnly')]
        procedure ZeroStatement() var R: Record \"LS Row\"; begin
            Seed(); Page.RunModal(0, R);
            Check('lookupMode=No,editable=Yes,rows=[A][B][]', Log.Take()); end;",
    );
}

const CARD: &str = r#"
page 62603 "LS Card"
{
    PageType = Card;
    layout { area(Content) { field(TxtCtl; TxtVar) { } } }
    var TxtVar: Text[20];
}
"#;

// errors/lt-ms-refusals-132/P132TwoModalHandlers.al:466 PW_ProbeB_Statement and :508 PW_ProbeB_InIf: Page.RunModal(Id) as a
// statement opens a card without its Cancel action; with the return value used Cancel is there.
#[test]
fn run_modal_without_a_record_as_a_statement_has_no_cancel_action() {
    let src = format!(
        "{OBJECTS}{CARD}\ncodeunit 62691 T {{ Subtype = Test; {HELPERS}
        [TryFunction] local procedure TryCancel(var P: TestPage \"LS Card\") begin P.Cancel().Invoke(); end;
        [ModalPageHandler] procedure Probe(var P: TestPage \"LS Card\") begin
            if TryCancel(P) then begin Log.Add('cancel'); exit; end;
            Log.Add('cancel ' + GetLastErrorCode()); ClearLastError(); P.OK().Invoke(); Log.Add('ok'); end;
        [Test] [HandlerFunctions('Probe')]
        procedure Statement() begin Log.Take(); Page.RunModal(Page::\"LS Card\"); Check('cancel TestActionNotFound,ok', Log.Take()); end;
        [Test] [HandlerFunctions('Probe')]
        procedure InIf() begin Log.Take();
            if Page.RunModal(Page::\"LS Card\") = Action::LookupCancel then Log.Add('if=LookupCancel');
            Check('cancel,if=LookupCancel', Log.Take()); end;
        }}"
    );
    let o = al_testkit::outcomes_in(&src, "T");
    assert_eq!(o.len(), 2);
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

// After OK of Page.RunModal(Id, Rec) called as a statement, Rec keeps the values it had, also when the page closed on another
// row (pages/lt-y41-page-probes/Y41RunModalStatementRec.al:297 RMS_List_Next_Ok, gap G4-02).
#[test]
fn rec_after_a_statement_run_modal_ok_keeps_its_values() {
    let src = format!(
        "{OBJECTS}\ncodeunit 62692 T {{ Subtype = Test; {HELPERS}
        [ModalPageHandler] procedure MoveOk(var P: TestPage \"LS List\") begin P.Last(); P.OK().Invoke(); end;
        [ModalPageHandler] procedure StayOk(var P: TestPage \"LS List\") begin P.OK().Invoke(); end;
        [Test] [HandlerFunctions('MoveOk')]
        procedure Moved() var R: Record \"LS Row\"; begin
            Seed(); R.FindFirst(); Page.RunModal(Page::\"LS List\", R); Check('A', R.\"Code\"); end;
        [Test] [HandlerFunctions('MoveOk')]
        procedure MovedThenRefilled() var R: Record \"LS Row\"; begin
            Seed(); R.FindFirst(); Page.RunModal(Page::\"LS List\", R); R.Reset(); R.SetRange(\"Code\", 'B'); R.FindFirst();
            Check('B', R.\"Code\"); end;
        [Test] [HandlerFunctions('StayOk')]
        procedure Stayed() var R: Record \"LS Row\"; begin
            Seed(); R.FindFirst(); Page.RunModal(Page::\"LS List\", R); Check('A', R.\"Code\"); end;
        }}"
    );
    let o = al_testkit::outcomes_in(&src, "T");
    assert_eq!(o.len(), 3, "{o:?}");
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}
