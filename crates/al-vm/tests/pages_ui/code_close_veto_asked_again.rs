//! A `CurrPage.Close()` that the page's OnQueryClosePage refused, on a Card not run modally: once the page's code returns
//! to the client, BC asks OnQueryClosePage a second time with the same action (corpus
//! pages/parts/Y37PartAfterClose.al:861 and :874 CLP_AGCR_CloseVeto_PartAccess_Observe, from OnAfterGetCurrRecord during
//! OpenView; :913 CLP_Action_CloseVeto_PartAccess_Observe, from an action). What a second query that allows the close does is
//! not pinned, and neither is a vetoed close from another trigger: both are refused.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69960 "CCV Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 69960 "CCV Log"
{
    SingleInstance = true;
    var L: Text; Allow: Boolean; AllowSecond: Boolean; Asked: Integer; FromAction: Boolean; InValidate: Boolean;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end;
    procedure Setup(A: Boolean; A2: Boolean; Act: Boolean) begin Allow := A; AllowSecond := A2; Asked := 0; FromAction := Act; L := ''; end;
    procedure Answer(): Boolean begin Asked += 1; if Asked = 1 then exit(Allow); exit(AllowSecond); end;
    procedure IsFromAction(): Boolean begin exit(FromAction); end;
}
page 69960 "CCV Card"
{
    PageType = Card; SourceTable = "CCV Row"; SourceTableTemporary = true; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { trigger OnValidate() begin Probe(); end; } } }
    actions { area(Processing) { action(Act) { trigger OnAction() begin G.Add('ACTION'); Probe(); end; } } }
    var G: Codeunit "CCV Log"; Probed: Boolean;
    trigger OnOpenPage() begin Rec.Init(); Rec.Id := 1; Rec.Insert(); end;
    trigger OnAfterGetCurrRecord() begin if Probed or G.IsFromAction() then exit; Probed := true; G.Add('AGCR'); Probe(); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin G.Add('Q:' + Format(CloseAction)); exit(G.Answer()); end;
    trigger OnClosePage() begin G.Add('CLOSE'); end;
    local procedure Probe() begin CurrPage.Close(); G.Add('AFTER'); end;
}
codeunit 69961 C
{
    procedure ActionVeto(): Text
    var P: TestPage "CCV Card"; G: Codeunit "CCV Log";
    begin
        G.Setup(false, false, true); P.OpenView(); P.Act.Invoke(); G.Add('RETURNED:' + P.IdCtl.Value()); P.Close(); exit(G.Take());
    end;
    procedure AgcrVeto(): Text
    var P: TestPage "CCV Card"; G: Codeunit "CCV Log";
    begin
        G.Setup(false, false, false); P.OpenView(); G.Add('OPENED'); P.Close(); exit(G.Take());
    end;
    procedure AgcrVetoReadId(): Text
    var P: TestPage "CCV Card"; G: Codeunit "CCV Log";
    begin
        G.Setup(false, false, false); P.OpenView(); G.Add('ID:' + Format(P.IdCtl.AsInteger())); P.Close(); exit(G.Take());
    end;
    procedure AgcrVetoReadValue(): Text
    var P: TestPage "CCV Card"; G: Codeunit "CCV Log";
    begin
        G.Setup(false, false, false); P.OpenView(); exit(P.IdCtl.Value());
    end;
    procedure ActionVetoThenAllow(): Text
    var P: TestPage "CCV Card"; G: Codeunit "CCV Log";
    begin
        G.Setup(false, true, true); P.OpenView(); P.Act.Invoke(); exit(G.Take());
    end;
    procedure ValidateVeto(): Text
    var P: TestPage "CCV Card"; G: Codeunit "CCV Log";
    begin
        G.Setup(false, false, true); P.OpenEdit(); P.IdCtl.SetValue(1); exit(G.Take());
    end;
}
"#;

fn refused(routine: &str) -> String {
    match run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_close_vetoed_in_an_action_is_asked_again_when_the_action_returns() {
    // Y37PartAfterClose.al:913 `PARENT.ACTION;BEFORE.CLOSE;PARENT.QUERY:OK;AFTER.CLOSE;...;PARENT.QUERY:OK;`, then the page is
    // still open (:917 follow.parent.ok T) and TestPage.Close asks once more (:926 follow.trace, follow.queries 3).
    assert_eq!(
        text(run_in(SRC, "C", "ActionVeto")),
        "ACTION;Q:OK;AFTER;Q:OK;RETURNED:1;Q:OK;"
    );
}

#[test]
fn a_close_vetoed_in_on_after_get_curr_record_at_open_view_is_asked_again() {
    // Y37PartAfterClose.al:861 `PARENT.AGCR;BEFORE.CLOSE;PARENT.QUERY:OK;AFTER.CLOSE;...;PARENT.QUERY:OK;` at OpenView.
    assert_eq!(text(run_in(SRC, "C", "AgcrVeto")), "AGCR;Q:OK;AFTER;Q:OK;OPENED;Q:OK;");
}

#[test]
fn a_second_query_that_allows_the_close_is_refused() {
    assert!(refused("ActionVetoThenAllow").contains("second OnQueryClosePage"));
}

#[test]
fn a_close_vetoed_in_on_validate_is_refused() {
    assert!(refused("ValidateVeto").contains("CurrPage.Close"));
}

#[test]
fn after_a_veto_at_open_view_the_hosts_integer_control_reads_zero() {
    // Y37PartAfterClose.al:870 `follow.parent.id` 0 after the vetoed close in OnAfterGetCurrRecord (1 after an action's, :922).
    assert_eq!(
        text(run_in(SRC, "C", "AgcrVetoReadId")),
        "AGCR;Q:OK;AFTER;Q:OK;ID:0;Q:OK;"
    );
}

#[test]
fn after_a_veto_at_open_view_other_reads_of_the_host_are_refused() {
    assert!(refused("AgcrVetoReadValue").contains("OpenView"));
}
