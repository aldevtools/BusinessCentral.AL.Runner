//! `Clear` of a TestPage variable that waits for `Trap()`: pages/controls/W2PageVariableInit.al:266-291 (codeunit 68675)
//! Clear_TrappingTestPage. The next Page.Run is not trapped, so with no PageHandler it raises MissingUIHandler after OnInit.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
codeunit 69810 "CL Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
page 69811 "CL Card"
{
    PageType = Card; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(StepCtl; Step) { } } }
    var Step: Integer; Log: Codeunit "CL Log";
    trigger OnInit() begin Step := 100; Log.Add('OnInit'); end;
    trigger OnOpenPage() begin Log.Add('OnOpenPage(step=' + Format(Step) + ')'); end;
}
codeunit 69814 "CL Trap Test"
{
    Subtype = Test;
    var Log: Codeunit "CL Log";

    [TryFunction]
    local procedure ReadStep(var Pg: TestPage "CL Card"; var V: Text) begin V := Pg.StepCtl.Value(); end;

    [TryFunction]
    local procedure RunPageNow() begin Page.Run(Page::"CL Card"); end;

    [Test]
    procedure ClearTrapping()
    var Pg: TestPage "CL Card"; V: Text; R: Text;
    begin
        Log.Take();
        Pg.Trap();
        Clear(Pg);
        if RunPageNow() then R := 'run=ok' else R := 'run=' + GetLastErrorCode() + ':' + GetLastErrorText();
        R += ' log=' + Log.Take();
        if not ReadStep(Pg, V) then R += ' read=' + GetLastErrorCode();
        Error(R);
    end;
}
"#;

#[test]
fn clear_of_a_trapping_test_page_drops_the_trap() {
    assert_eq!(
        outcomes_in(SRC, "CL Trap Test"),
        vec![(
            "ClearTrapping".to_string(),
            Outcome::Fail("run=MissingUIHandler:Unhandled UI: Page 69811 log=OnInit; read=TestPageNotOpened".into())
        )]
    );
}
