//! `Clear` of a TestPage variable that is open: pages/controls/W2PageVariableInit.al (codeunit 68675) Clear_OpenTestPage,
//! pages/lifecycle/PageVariableLifecycle.al TestPageVar_ClearClosedOpenAndReopen. The trapping case needs a test method:
//! al-test-cli tests/suite/clear_trapping_test_page.rs.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

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
page 69813 "CL Closing Card"
{
    PageType = Card; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(StepCtl; Step) { } } }
    var Step: Integer; Log: Codeunit "CL Log";
    trigger OnClosePage() begin Log.Add('OnClosePage'); end;
}
codeunit 69812 "CL Test"
{
    var Log: Codeunit "CL Log";

    [TryFunction]
    local procedure ReadStep(var Pg: TestPage "CL Card"; var V: Text) begin V := Pg.StepCtl.Value(); end;

    [TryFunction]
    local procedure ClosePage(var Pg: TestPage "CL Card") begin Pg.Close(); end;

    [TryFunction]
    local procedure RunPageNow() begin Page.Run(Page::"CL Card"); end;

    procedure ClearOpen(): Text
    var Pg: TestPage "CL Card"; V: Text; R: Text;
    begin
        Log.Take();
        Pg.OpenEdit();
        Log.Take();
        Clear(Pg);
        R := 'after_clear=' + Log.Take();
        if not ReadStep(Pg, V) then R += ' read=' + GetLastErrorCode();
        if not ClosePage(Pg) then R += ' close=' + GetLastErrorCode() + ':' + GetLastErrorText();
        Pg.OpenEdit();
        R += ' open_again=' + Log.Take();
        Pg.Close();
        exit(R);
    end;

    procedure ClearClosing()
    var Pg: TestPage "CL Closing Card";
    begin
        Pg.OpenEdit();
        Clear(Pg);
    end;
}
"#;

#[test]
fn clear_of_an_open_test_page_closes_it() {
    // W2PageVariableInit.al:251-258 (`after_clear` empty, read and close raise TestPageNotOpened, OnInit runs again);
    // PageVariableLifecycle.al:186-189 (`clearOpen` adds nothing to the log).
    assert_eq!(
        text(run_in(SRC, "CL Test", "ClearOpen")),
        "after_clear= read=TestPageNotOpened close=TestPageNotOpened:The TestPage is not open. open_again=OnInit;OnOpenPage(step=100);"
    );
}

#[test]
fn clear_of_an_open_test_page_with_a_close_trigger_is_refused() {
    // No corpus case clears an open page that declares OnQueryClosePage or OnClosePage: whether they run is not pinned.
    match run_in(SRC, "CL Test", "ClearClosing") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("Clear of a TestPage"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
