//! Refusals whose reason text named the wrong cause (al-runner issues 7400, 7414, 7362). The reason now says what actually
//! stopped the run; the shape of 7414 is pinned since and runs.

use al_test_cli::report::Outcome;

fn reason(src: &str, codeunit: &str) -> String {
    let o = al_testkit::outcomes_in(src, codeunit);
    match o.as_slice() {
        [(_, Outcome::NotRun(r))] => r.clone(),
        other => panic!("one refused test expected: {other:#?}"),
    }
}

/// Issue 7400: the list saves the row the ModalPageHandler edited as the platform closes it, and OnModifyRecord raises. The
/// page has no OnQueryClosePage, so the reason must not name it.
#[test]
fn issue_7400_error_of_the_save_at_modal_close_names_the_save() {
    let src = r#"
table 50400 "RR Entry" { fields { field(1; "No."; Integer) { } field(2; Canceled; Boolean) { } field(3; Code; Code[10]) { } } keys { key(PK; "No.") { } } }
codeunit 50401 "RR Entry-Edit"
{
    TableNo = "RR Entry";
    trigger OnRun() begin Rec.TestField(Canceled, false); end;
}
page 50402 "RR Entries"
{
    PageType = List; SourceTable = "RR Entry"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("No."; Rec."No.") { } field(Code; Rec.Code) { } } } }
    trigger OnModifyRecord(): Boolean
    begin
        Codeunit.Run(Codeunit::"RR Entry-Edit", Rec);
        exit(false);
    end;
}
codeunit 50403 "RR Save Tests"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('EditHandler')]
    procedure SaveAtCloseRaises()
    var E: Record "RR Entry";
    begin
        E."No." := 1; E.Canceled := true; E.Insert();
        asserterror Page.RunModal(Page::"RR Entries", E);
    end;
    [ModalPageHandler]
    procedure EditHandler(var P: TestPage "RR Entries")
    begin
        P.Code.SetValue('X');
    end;
}
"#;
    let r = reason(src, "RR Save Tests");
    assert!(!r.contains("OnQueryClosePage"), "{r}");
    assert!(r.contains("saved") && r.contains("not pinned"), "{r}");
}

/// Issue 7414: a bare read of `UseRequestPage` in a report trigger is a read, not an assignment. It now runs: after
/// `UseRequestPage(false)` on the variable it reads false in OnPreReport (reports/bare-report-member/
/// UseRequestPageReadInTriggers.al:100, :235 UseRp_Read_VariableSetFalse_Run), so the report's own Error is the outcome.
#[test]
fn issue_7414_bare_use_request_page_read_is_not_called_unassignable() {
    let src = r#"
table 50410 "RR Header" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
report 50411 "RR Batch"
{
    ProcessingOnly = true;
    dataset { dataitem(H; "RR Header") { } }
    trigger OnPreReport()
    begin
        if not UseRequestPage then
            Error('no request page');
    end;
}
codeunit 50412 "RR Batch Tests"
{
    Subtype = Test;
    [Test]
    procedure RunIt()
    var B: Report "RR Batch";
    begin
        B.UseRequestPage(false);
        B.Run();
    end;
}
"#;
    let o = al_testkit::outcomes_in(src, "RR Batch Tests");
    assert_eq!(
        o,
        vec![("RunIt".to_string(), Outcome::Fail("no request page".to_string()))]
    );
}

/// Issue 7362: a part linked through `Provider` to another part, both with a source table. When the provider part is not among
/// the open parts (here its Visible expression hid it), the reason must say so, not that a page has no source table.
#[test]
fn issue_7362_provider_linked_part_does_not_blame_a_missing_source_table() {
    let src = r#"
table 50420 "RR Host" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { } } }
table 50421 "RR Assign" { fields { field(1; "No."; Code[10]) { } field(2; "Doc No."; Code[10]) { } } keys { key(PK; "No.", "Doc No.") { } } }
table 50422 "RR Data" { fields { field(1; "Doc No."; Code[10]) { } field(2; Val; Text[20]) { } } keys { key(PK; "Doc No.") { } } }
page 50423 "RR Assign Part" { PageType = ListPart; SourceTable = "RR Assign"; layout { area(Content) { repeater(R) { field("Doc No."; Rec."Doc No.") { } } } } }
page 50424 "RR Data Part" { PageType = ListPart; SourceTable = "RR Data"; layout { area(Content) { repeater(R) { field(Val; Rec.Val) { } } } } }
page 50425 "RR Host Card"
{
    PageType = ListPlus; SourceTable = "RR Host"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } } area(FactBoxes) { part(Base; "RR Data Part") { } } }
}
pageextension 50426 "RR Host Ext" extends "RR Host Card"
{
    layout
    {
        addfirst(FactBoxes)
        {
            part(Assign; "RR Assign Part") { SubPageLink = "No." = field("No."); Visible = ShowAssign; }
            part(Data; "RR Data Part") { Provider = Assign; SubPageLink = "Doc No." = field("Doc No."); }
        }
    }
    var ShowAssign: Boolean;
}
codeunit 50427 "RR Provider Tests"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('HostHandler')]
    procedure OpenIt()
    var H: Record "RR Host"; A: Record "RR Assign"; D: Record "RR Data";
    begin
        H."No." := 'H1'; H.Insert();
        A."No." := 'H1'; A."Doc No." := 'D1'; A.Insert();
        D."Doc No." := 'D1'; D.Val := 'V'; D.Insert();
        Page.RunModal(Page::"RR Host Card", H);
        Error('stop');
    end;
    [ModalPageHandler]
    procedure HostHandler(var P: TestPage "RR Host Card")
    begin
        P.OK().Invoke();
    end;
}
"#;
    // Parts hidden by a Visible expression follow their Provider now (ProviderLinkedPartsMatrix.al), so the test runs to its
    // own Error('stop'); whatever it ends as, no refusal may blame a missing source table.
    for (test, outcome) in al_testkit::outcomes_in(src, "RR Provider Tests") {
        if let Outcome::NotRun(r) = outcome {
            assert!(!r.contains("without a source table"), "{test}: {r}");
        }
    }
}
