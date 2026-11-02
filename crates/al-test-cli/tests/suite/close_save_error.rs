//! `TestPage.Close()` on a Card whose save fails with an AL error (crates/al-vm/src/pages/close_save_failed.rs). Each test names
//! the corpus test of testpageclose/lt-y67-close-save-error/Y67CloseSaveError.al that pins it.

use al_testrun::Outcome;

const SRC: &str = r#"
table 69980 "CS Row"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Name; Text[30]) { } field(3; Note; Text[30]) { } }
    keys { key(PK; "Code") { } }
    trigger OnModify()
    var Log: Codeunit "CS Log";
    begin
        Log.Add('table modify ' + Rec."Code" + ' name=' + Rec.Name);
        if Rec.Name = 'FAILMOD' then Error('Table modify refused for %1.', Rec."Code");
    end;
}
page 69981 "CS Card"
{
    PageType = Card; SourceTable = "CS Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } field(Note; Rec.Note) { } } }
    trigger OnOpenPage() var Log: Codeunit "CS Log"; begin Log.Add('open'); end;
    trigger OnModifyRecord(): Boolean
    var Log: Codeunit "CS Log";
    begin
        Log.Add('page modify ' + Rec."Code" + ' name=' + Rec.Name);
        if Rec.Name = 'FAILPAGE' then Error('Page modify refused for %1.', Rec."Code");
        exit(true);
    end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean var Log: Codeunit "CS Log"; begin Log.Add('queryclose ' + Format(CloseAction)); exit(true); end;
    trigger OnClosePage() var Log: Codeunit "CS Log"; begin Log.Add('close'); end;
}
page 69982 "CS Plain"
{
    PageType = Card; SourceTable = "CS Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } }
}
codeunit 69983 "CS Log"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Add(Line: Text) begin Lines += Line + ';'; end;
    procedure Take(): Text var R: Text; begin R := Lines; Lines := ''; exit(R); end;
}
codeunit 69984 C
{
    Subtype = Test;

    local procedure Seed()
    var Row: Record "CS Row"; Log: Codeunit "CS Log";
    begin
        Row.DeleteAll();
        Row."Code" := 'R1'; Row.Name := 'start'; Row.Insert();
        Commit();
        Log.Take();
    end;

    local procedure Names(): Text
    var Row: Record "CS Row"; R: Text;
    begin
        if Row.FindSet() then
            repeat R += Row."Code" + '=' + Row.Name + ' '; until Row.Next() = 0;
        exit(R);
    end;

    local procedure Err(): Text
    var R: Text;
    begin
        R := GetLastErrorText() + '/' + GetLastErrorCode();
        ClearLastError();
        exit(R);
    end;

    [TryFunction]
    local procedure TryClose(var Card: TestPage "CS Card") begin Card.Close(); end;

    [TryFunction]
    local procedure TrySet(var Card: TestPage "CS Card"; V: Text) begin Card.Name.SetValue(V); end;

    [TryFunction]
    local procedure TryNext(var Card: TestPage "CS Card") begin Card.Next(); end;

    local procedure Failing(var Card: TestPage "CS Card"; NewName: Text): Text
    var Row: Record "CS Row"; Log: Codeunit "CS Log"; R: Text;
    begin
        Seed();
        Row.Get('R1');
        Card.OpenEdit();
        Card.GoToRecord(Row);
        Card.Name.SetValue(NewName);
        Log.Add('before close');
        asserterror Card.Close();
        R := Err() + '|' + Log.Take() + '|' + Names() + '|' + Card.Name.Value();
        exit(R);
    end;

    // Y67CloseSaveError.al:239 R1_TableModifyError_SecondClose
    [Test]
    procedure T_TableSecondClose()
    var Card: TestPage "CS Card"; Log: Codeunit "CS Log"; R: Text;
    begin
        R := Failing(Card, 'FAILMOD');
        if TryClose(Card) then R += '|ok' else R += '|error ' + Err();
        Error(R + '|' + Log.Take() + '|' + Names());
    end;

    // Y67CloseSaveError.al:256 R1_TableModifyError_CorrectedThenClose
    [Test]
    procedure T_Corrected()
    var Card: TestPage "CS Card"; Log: Codeunit "CS Log"; R: Text;
    begin
        R := Failing(Card, 'FAILMOD');
        if TrySet(Card, 'fixed') then R += '|ok' else R += '|error ' + Err();
        if TryClose(Card) then R += '|ok' else R += '|error ' + Err();
        Error(R + '|' + Log.Take() + '|' + Names());
    end;

    // Y67CloseSaveError.al:285 R1_TableModifyError_LeftOpen and :298 _LeftOpen_Read
    [Test]
    procedure T_LeftOpen()
    var Card: TestPage "CS Card"; Log: Codeunit "CS Log";
    begin
        Failing(Card, 'FAILMOD');
        Log.Add('test body end');
    end;

    [Test]
    procedure T_LeftOpenRead()
    var Log: Codeunit "CS Log";
    begin
        Error(Log.Take() + '|' + Names());
    end;

    // Y67CloseSaveError.al:336 R1_PageModifyError_SecondClose
    [Test]
    procedure T_PageSecondClose()
    var Card: TestPage "CS Card"; Log: Codeunit "CS Log"; R: Text;
    begin
        R := Failing(Card, 'FAILPAGE');
        if TryClose(Card) then R += '|ok' else R += '|error ' + Err();
        Error(R + '|' + Log.Take() + '|' + Names());
    end;

    // Y67CloseSaveError.al:409 R1_NoCloseTriggers_TableModifyError
    [Test]
    procedure T_Plain()
    var Card: TestPage "CS Plain"; Row: Record "CS Row"; Log: Codeunit "CS Log"; R: Text;
    begin
        Seed();
        Row.Get('R1');
        Card.OpenEdit();
        Card.GoToRecord(Row);
        Card.Name.SetValue('FAILMOD');
        asserterror Card.Close();
        R := Err() + '|' + Log.Take() + '|' + Card.Name.Value();
        Card.Close();
        Error(R + '|' + Log.Take() + '|' + Names());
    end;

    // Not pinned: moving to another row of a page whose save failed at Close() is refused.
    [Test]
    procedure T_NextRefused()
    var Card: TestPage "CS Card";
    begin
        Failing(Card, 'FAILMOD');
        if TryNext(Card) then;
    end;
}
"#;

fn outcome(name: &str) -> Outcome {
    let outcomes = al_testkit::outcomes_in(SRC, "C");
    outcomes
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
}

fn fail(name: &str) -> Vec<String> {
    match outcome(name) {
        Outcome::Fail(msg) => msg.split('|').map(str::to_string).collect(),
        other => panic!("{other:?}"),
    }
}

const FAILED: [&str; 4] = [
    "Table modify refused for R1./TestWrapped",
    "open;before close;page modify R1 name=FAILMOD;table modify R1 name=FAILMOD;",
    "R1=start ",
    "FAILMOD",
];

#[test]
fn a_failed_save_raises_test_wrapped_and_a_second_close_runs_nothing() {
    let mut want = FAILED.to_vec();
    want.extend(["ok", "", "R1=start "]);
    assert_eq!(fail("T_TableSecondClose"), want);
}

#[test]
fn set_value_after_a_failed_save_and_close_store_nothing() {
    let mut want = FAILED.to_vec();
    want.extend(["ok", "ok", "", "R1=start "]);
    assert_eq!(fail("T_Corrected"), want);
}

#[test]
fn a_page_left_open_after_a_failed_save_does_not_close_at_test_end() {
    assert!(
        matches!(outcome("T_LeftOpen"), Outcome::Pass),
        "{:?}",
        outcome("T_LeftOpen")
    );
    assert_eq!(fail("T_LeftOpenRead"), ["test body end;", "R1=start "]);
}

#[test]
fn a_failed_on_modify_record_raises_test_wrapped() {
    assert_eq!(
        fail("T_PageSecondClose"),
        [
            "Page modify refused for R1./TestWrapped",
            "open;before close;page modify R1 name=FAILPAGE;",
            "R1=start ",
            "FAILPAGE",
            "ok",
            "",
            "R1=start "
        ]
    );
}

#[test]
fn a_card_without_close_triggers_behaves_the_same() {
    assert_eq!(
        fail("T_Plain"),
        [
            "Table modify refused for R1./TestWrapped",
            "table modify R1 name=FAILMOD;",
            "FAILMOD",
            "",
            "R1=start "
        ]
    );
}

#[test]
fn moving_after_a_failed_save_is_refused() {
    match outcome("T_NextRefused") {
        Outcome::NotRun(why) => assert!(why.contains("whose save failed"), "{why}"),
        other => panic!("{other:?}"),
    }
}
