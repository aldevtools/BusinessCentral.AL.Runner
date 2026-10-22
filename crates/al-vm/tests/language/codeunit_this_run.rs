//! `this.Run()` inside a codeunit runs the codeunit's own instance: its globals count the runs, an error in OnRun is
//! returned when the result is used, and the record form hands the record to OnRun
//! (xmlport/lt-aj-xmlport/AjRunAndXmlPort.al:6-140, ThisRun_* tests).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50120 "This Run T" { fields { field(1; Id; Integer) { } field(2; Txt; Text[30]) { } } keys { key(PK; Id) { } } }
codeunit 50121 "This Run Plain"
{
    var RunCount: Integer;
    trigger OnRun() begin RunCount += 1; end;
    procedure CallThisRun(): Boolean begin exit(this.Run()); end;
    procedure CallThisRunAsStatement() begin this.Run(); end;
    procedure GetCount(): Integer begin exit(RunCount); end;
}
codeunit 50122 "This Run Error"
{
    trigger OnRun() begin Error('boom'); end;
    procedure CallThisRun(): Boolean begin exit(this.Run()); end;
}
codeunit 50123 "This Run Rec"
{
    TableNo = "This Run T";
    trigger OnRun() begin Rec.Txt := 'ran'; Rec.Id += 100; end;
    procedure CallThisRun(var R: Record "This Run T"): Boolean begin exit(this.Run(R)); end;
}
codeunit 50124 C
{
    procedure Main(): Text
    var
        Plain: Codeunit "This Run Plain";
        Failing: Codeunit "This Run Error";
        WithRec: Codeunit "This Run Rec";
        R: Record "This Run T";
        S: Text;
    begin
        S := Format(Plain.CallThisRun()) + '|' + Format(Plain.GetCount());
        Plain.CallThisRunAsStatement();
        S += ' ' + Format(Plain.GetCount());
        S += ' ' + Format(Plain.Run()) + '|' + Format(Plain.GetCount());
        S += ' ' + Format(Failing.CallThisRun()) + '|' + GetLastErrorText();
        R.Id := 1;
        S += ' ' + Format(WithRec.CallThisRun(R)) + '|' + Format(R.Id) + R.Txt;
        exit(S);
    end;
}
"#;

#[test]
fn this_run_runs_the_codeunits_own_instance() {
    assert_eq!(text(run_in(SRC, "C", "Main")), "Yes|1 2 Yes|3 No|boom Yes|101ran");
}
