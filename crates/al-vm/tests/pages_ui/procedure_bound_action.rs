//! An action or action group whose Visible or Enabled is bound to a procedure call (corpus bump 47, AL Runner issue 3762):
//! pages/lt-qq-procedure-bound-action/QqBoundProps.al. BC compiles it with warning AL0573; the property reads No whatever
//! the procedure returns, and the procedure never runs.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69960 "PB Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69961 "PB Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin if Entries <> '' then Entries += ','; Entries += E; end;
    procedure Read(): Text begin exit(Entries); end;
    procedure Clear() begin Entries := ''; end;
}
page 69962 "PB Card"
{
    PageType = Card; SourceTable = "PB Row"; ApplicationArea = All;
    layout { area(Content) { field(Id; Rec.Id) { } } }
    actions
    {
        area(Processing)
        {
            action(VisProc) { Visible = IsAllowed(); trigger OnAction() begin Log.Add('VisProc'); end; }
            action(EnProc) { Enabled = IsAllowed(); trigger OnAction() begin Log.Add('EnProc'); end; }
            group(Grp)
            {
                Visible = IsAllowed();
                action(InGrp) { trigger OnAction() begin Log.Add('InGrp'); end; }
            }
        }
    }
    trigger OnOpenPage() begin Log.Add('OnOpenPage'); end;
    var Log: Codeunit "PB Log";
    procedure IsAllowed(): Boolean begin Log.Add('IsAllowed'); exit(true); end;
}
codeunit 69963 C
{
    var Log: Codeunit "PB Log";
    procedure Reads(): Text
    var R: Record "PB Row"; P: TestPage "PB Card"; S: Text;
    begin
        R.Id := 1; R.Insert();
        P.OpenView();
        S := Format(P.VisProc.Visible()) + ' ' + Format(P.VisProc.Enabled()) + ' ' + Format(P.EnProc.Visible()) + ' '
            + Format(P.EnProc.Enabled()) + ' ' + Format(P.InGrp.Visible()) + ' ' + Format(P.InGrp.Enabled()) + ' ' + Log.Read();
        P.Close();
        exit(S);
    end;
    procedure InvokeVisProc(): Text
    var R: Record "PB Row"; P: TestPage "PB Card";
    begin
        R.Id := 1; R.Insert();
        P.OpenView(); Log.Clear();
        P.VisProc.Invoke();
        exit(Log.Read());
    end;
    procedure InvokeEnProc(): Text
    var R: Record "PB Row"; P: TestPage "PB Card";
    begin
        R.Id := 1; R.Insert();
        P.OpenView(); Log.Clear();
        P.EnProc.Invoke();
        exit('[' + Log.Read() + ']');
    end;
    procedure InvokeInGrp(): Text
    var R: Record "PB Row"; P: TestPage "PB Card";
    begin
        R.Id := 1; R.Insert();
        P.OpenView(); Log.Clear();
        P.InGrp.Invoke();
        exit(Log.Read());
    end;
}
"#;

const ARGS: &str = r#"
page 69964 "PB Arg Card"
{
    PageType = Card; ApplicationArea = All;
    actions { area(Processing) { action(VisArg) { Visible = Check(1); trigger OnAction() begin end; } } }
    procedure Check(N: Integer): Boolean begin exit(N = 1); end;
}
codeunit 69965 C
{
    procedure ReadVisArg(): Text
    var P: TestPage "PB Arg Card";
    begin
        P.OpenView();
        exit(Format(P.VisArg.Visible()));
    end;
}
"#;

/// QqBoundProps.al:77-211 (QQ_ProcBound_VisibleEnabled): Visible and Enabled bound to a procedure call read No, also on an
/// action in a group bound that way, and the procedure never runs (the log holds only OnOpenPage).
#[test]
fn a_procedure_bound_action_property_reads_no_and_the_procedure_never_runs() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Reads")),
        "No Yes Yes No No Yes OnOpenPage"
    );
}

/// QqBoundProps.al (QQ_Invoke_VisibleTrueProc, QQ_Invoke_InGroupVisibleProc): Invoke runs OnAction when Visible reads No.
#[test]
fn invoke_runs_on_action_when_a_procedure_bound_visible_reads_no() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "InvokeVisProc")), "VisProc");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "InvokeInGrp")), "InGrp");
}

/// QqBoundProps.al (QQ_Invoke_EnabledTrueProc): Invoke skips OnAction silently when Enabled reads No.
#[test]
fn invoke_skips_on_action_when_a_procedure_bound_enabled_reads_no() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "InvokeEnProc")), "[]");
}

/// A call with arguments is not pinned: still refused.
#[test]
fn a_procedure_call_with_arguments_stays_refused() {
    let r = al_testkit::run_in(ARGS, "C", "ReadVisArg");
    assert!(
        matches!(&r, Err(Signal::Unsupported(w)) if w.contains("a call")),
        "{r:?}"
    );
}
