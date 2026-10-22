//! A Worksheet page over a temporary source, opened without a record, whose Rec stands on a row other than the first of the
//! view when it opens. Rows a procedure of the page variable filled before Run or RunModal, with OnOpenPage leaving Rec as it
//! was: it opens on that row, as a List does (pinned by pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al:2378-2705, the
//! QT_Temp* tests; the modal and lookup forms are checked by the corpus run). Forms past those pins are refused. A Worksheet
//! whose OnOpenPage moved Rec or changed its key is not pinned: assumption `worksheet-temp-source-open-row`.
//! The same Worksheet over a database table stays refused.

const SRC: &str = r#"
table 69980 "OpWs Row" {
    fields { field(1; "No."; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "No.") { Clustered = true; } key(ByName; Name) { } }
}
page 69981 "OpWs Temp Worksheet"
{
    PageType = Worksheet; SourceTable = "OpWs Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } field(NameCtl; Rec.Name) { } } } }
    trigger OnOpenPage()
    begin
        Opened := true;
    end;
    procedure Fill()
    begin
        Rec.DeleteAll();
        Rec.Init(); Rec."No." := 1; Rec.Name := 'one'; Rec.Insert();
        Rec.Init(); Rec."No." := 11; Rec.Name := 'eleven'; Rec.Insert();
    end;
    var Opened: Boolean;
}
page 69982 "OpWs Self Worksheet"
{
    PageType = Worksheet; SourceTable = "OpWs Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } } } }
    trigger OnOpenPage()
    begin
        Rec.Init(); Rec."No." := 1; Rec.Insert();
        Rec.Init(); Rec."No." := 2; Rec.Insert();
        Rec.Init(); Rec."No." := 3; Rec.Insert();
    end;
}
page 69985 "OpWs Key Worksheet"
{
    PageType = Worksheet; SourceTable = "OpWs Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } } } }
    trigger OnOpenPage()
    begin
        Rec.Init(); Rec."No." := 1; Rec.Name := 'b'; Rec.Insert();
        Rec.Init(); Rec."No." := 2; Rec.Name := 'a'; Rec.Insert();
        Rec.SetCurrentKey(Name);
        Rec.Get(1);
    end;
}
page 69983 "OpWs Db Worksheet"
{
    PageType = Worksheet; SourceTable = "OpWs Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } } } }
    trigger OnOpenPage() begin Rec.Get(2); end;
}
page 69986 "OpWs Next Worksheet"
{
    PageType = Worksheet; SourceTable = "OpWs Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } } } }
    trigger OnOpenPage() begin Opened := true; end;
    trigger OnNextRecord(Steps: Integer): Integer
    begin
        exit(Rec.Next(Steps));
    end;
    procedure Fill()
    begin
        Rec.Init(); Rec."No." := 1; Rec.Name := 'one'; Rec.Insert();
        Rec.Init(); Rec."No." := 11; Rec.Name := 'eleven'; Rec.Insert();
        Rec.Get(11);
    end;
    var Opened: Boolean;
}
page 69987 "OpWs Keyed Before Worksheet"
{
    PageType = Worksheet; SourceTable = "OpWs Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } } } }
    trigger OnOpenPage() begin Opened := true; end;
    var Opened: Boolean;
    procedure Fill()
    begin
        Rec.Init(); Rec."No." := 1; Rec.Name := 'b'; Rec.Insert();
        Rec.Init(); Rec."No." := 2; Rec.Name := 'a'; Rec.Insert();
        Rec.SetCurrentKey(Name);
        Rec.Get(1);
    end;
}
codeunit 69984 C
{
    procedure NextPageRun(): Text
    var
        P: TestPage "OpWs Next Worksheet";
        W: Page "OpWs Next Worksheet";
    begin
        W.Fill();
        P.Trap();
        W.Run();
        exit(P.NoCtl.Value);
    end;
    procedure KeyedBeforePageRun(): Text
    var
        P: TestPage "OpWs Keyed Before Worksheet";
        W: Page "OpWs Keyed Before Worksheet";
    begin
        W.Fill();
        P.Trap();
        W.Run();
        exit(P.NoCtl.Value);
    end;
    procedure FilledByProcedure(): Text
    var
        P: TestPage "OpWs Temp Worksheet";
        W: Page "OpWs Temp Worksheet";
    begin
        W.Fill();
        P.Trap();
        W.Run();
        exit(P.NoCtl.Value + '|' + P.NameCtl.Value);
    end;
    procedure FilledInOnOpenPage(): Text
    var
        P: TestPage "OpWs Self Worksheet";
    begin
        P.OpenView();
        exit(P.NoCtl.Value);
    end;
    procedure KeyChanged(): Text
    var
        P: TestPage "OpWs Key Worksheet";
    begin
        P.OpenView();
        exit(P.NoCtl.Value);
    end;
    procedure DatabaseSource(): Text
    var
        R: Record "OpWs Row";
        P: TestPage "OpWs Db Worksheet";
    begin
        R."No." := 1; R.Insert();
        R."No." := 2; R.Insert();
        P.OpenView();
        exit(P.NoCtl.Value);
    end;
}
"#;

#[test]
fn a_temporary_worksheet_filled_before_it_opens_opens_on_the_row_rec_stands_on() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "FilledByProcedure")),
        "11|eleven"
    );
}

#[test]
fn a_temporary_worksheet_filled_in_onopenpage_opens_on_the_last_insert() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "FilledInOnOpenPage")),
        "3"
    );
}

#[test]
fn a_worksheet_over_a_database_table_whose_onopenpage_moves_rec_stays_refused() {
    match al_testkit::run_in(SRC, "C", "DatabaseSource") {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains("OnOpenPage left Rec"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_temporary_worksheet_filled_before_it_opens_is_pinned_and_marks_no_assumption() {
    use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
    clear_hits();
    let _ = al_testkit::run_in(SRC, "C", "FilledByProcedure");
    assert!(
        !take_hits().contains(&Assumption::WorksheetTempSourceOpenRow),
        "QrMovedTests.al:2665 QT_TempWs_Run_B pins this open"
    );
}

#[test]
fn a_temporary_worksheet_whose_onopenpage_filled_it_still_marks_the_assumption() {
    use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
    clear_hits();
    let _ = al_testkit::run_in(SRC, "C", "FilledInOnOpenPage");
    assert!(
        take_hits().contains(&Assumption::WorksheetTempSourceOpenRow),
        "the open must mark the assumption"
    );
}

/// QT_TempWsNext_* pin OnNextRecord for RunModal and lookup mode only; Page.Run of such a page is past the pins.
#[test]
fn a_temporary_worksheet_with_onnextrecord_run_by_page_run_is_refused() {
    match al_testkit::run_in(SRC, "C", "NextPageRun") {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains("not pinned"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// QT_TempWs_RunModal_ByName_B pins a key set before the open for RunModal only; Page.Run in that key is past the pins.
#[test]
fn a_temporary_worksheet_in_a_key_set_before_page_run_is_refused() {
    match al_testkit::run_in(SRC, "C", "KeyedBeforePageRun") {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains("not pinned"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_temporary_worksheet_whose_key_changed_opens_on_its_first_row_and_marks_the_same_assumption() {
    use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
    clear_hits();
    // First row of the view in key ByName: No. 2 ('a').
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "KeyChanged")), "2");
    assert!(
        take_hits().contains(&Assumption::WorksheetTempSourceOpenRow),
        "the first-row path must mark the assumption"
    );
}
