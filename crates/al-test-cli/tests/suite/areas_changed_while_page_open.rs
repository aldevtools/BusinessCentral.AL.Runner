//! The session's application areas set again while a TestPage is open (al-runner issue 7001; Base Application's
//! RefreshExperienceTierCurrentCompany ends in ApplicationArea(Text)): the open page keeps the controls it had when it opened, until
//! it is closed and opened again (corpus branch al-runtime-rs-staging-r6962, pages/application-area/AreasChangedWhilePageOpen.al).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 50770 "AC Row" { fields { field(1; Id; Integer) { } field(2; A; Text[10]) { } field(3; B; Text[10]) { } field(4; S; Text[10]) { } } keys { key(PK; Id) { } } }
page 50771 "AC Card"
{
    PageType = Card; SourceTable = "AC Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) {
        field(AllCtl; Rec.A) { ApplicationArea = All; }
        field(BasicCtl; Rec.B) { ApplicationArea = Basic; }
        field(SuiteCtl; Rec.S) { ApplicationArea = Suite; } } }
    actions { area(Processing) {
        action(SetBasic) { ApplicationArea = All; trigger OnAction() begin ApplicationArea('#Basic'); end; }
        action(SetSuite) { ApplicationArea = All; trigger OnAction() begin ApplicationArea('#Suite'); end; } } }
}
codeunit 50772 T
{
    Subtype = Test;
    local procedure Start(Areas: Text)
    var Row: Record "AC Row";
    begin
        Row.DeleteAll(); Row.Id := 1; Row.A := 'a'; Row.B := 'b'; Row.S := 's'; Row.Insert();
        ApplicationArea(Areas);
    end;
    [TryFunction] local procedure TryRead(var P: TestPage "AC Card"; Which: Integer; var Value: Text)
    begin
        case Which of 1: Value := P.AllCtl.Value(); 2: Value := P.BasicCtl.Value(); 3: Value := P.SuiteCtl.Value(); end;
    end;
    local procedure Reads(var P: TestPage "AC Card"): Text
    var R: Text; V: Text; W: Integer;
    begin
        for W := 1 to 3 do begin V := ''; if TryRead(P, W, V) then R += V + ';' else R += 'ERR;'; end;
        exit(R);
    end;
    [Test] procedure FewerTags()
    var P: TestPage "AC Card"; R: Text;
    begin
        Start('#Basic,#Suite'); P.OpenView(); P.SetBasic.Invoke();
        R := ApplicationArea() + ' ' + Reads(P); P.Close(); P.OpenView(); R += ' ' + Reads(P); P.Close();
        ApplicationArea(''); Error(R);
    end;
    [Test] procedure OtherTags()
    var P: TestPage "AC Card"; R: Text;
    begin
        Start('#Basic'); P.OpenView(); R := Reads(P); P.SetSuite.Invoke();
        R += ' ' + Reads(P); P.Close(); P.OpenView(); R += ' ' + Reads(P); P.Close();
        ApplicationArea(''); Error(R);
    end;
    [Test] procedure FromEmptyInTest()
    var P: TestPage "AC Card"; R: Text;
    begin
        Start(''); P.OpenView(); ApplicationArea('#Basic'); R := Reads(P); P.Close();
        ApplicationArea(''); Error(R);
    end;
}
"#;

/// AreasChangedWhilePageOpen.al: _FewerTags `after` `a;b;s;`, `reopened` `a;b;ERR`; _OtherTags `a;b;ERR` before and after, `a;ERR;s;`
/// reopened; _FromEmpty and _InTest_ `a;b;s;`.
#[test]
fn an_open_page_keeps_the_controls_it_opened_with() {
    let got: Vec<_> = outcomes_in(SRC, "T").into_iter().map(|(_, o)| o).collect();
    let fail = |s: &str| Outcome::Fail(s.into());
    assert_eq!(
        got,
        vec![
            fail("#Basic a;b;s; a;b;ERR;"),
            fail("a;b;ERR; a;b;ERR; a;ERR;s;"),
            fail("a;b;s;"),
        ]
    );
}
