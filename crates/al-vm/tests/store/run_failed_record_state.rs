//! The caller's record after a failed `Codeunit.Run(Id, Rec)` whose result is used (al-runner issue 6924).
//! Corpus: codeunits/run-onrun/RunFailedRecordState.al:287-484.

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 51400 "RF Row"
{
    fields { field(1; Code; Code[10]) { } field(2; Amount; Integer) { } }
    keys { key(PK; Code) { } key(ByAmount; Amount) { } }
}

codeunit 51401 "RF Runner"
{
    TableNo = "RF Row";
    trigger OnRun()
    begin
        Rec.SetRange(Code, 'Z');
        Rec.SetCurrentKey(Amount);
        Rec.Amount := 99;
        Rec.Mark(true);
        Error('boom');
    end;
}

codeunit 51402 C
{
    procedure Failed(): Text
    var R: Record "RF Row"; Ok: Boolean;
    begin
        R.Code := 'A'; R.Amount := 10; R.Insert();
        Commit(); R.Get('A');
        Ok := Codeunit.Run(Codeunit::"RF Runner", R);
        exit(StrSubstNo('%1 %2 [%3] %4 %5', Ok, R.Amount, R.GetFilters(), R.CurrentKey(), R.MarkedOnly()));
    end;

    procedure Statement(): Text
    var R: Record "RF Row";
    begin
        R.Code := 'A'; R.Amount := 10; R.Insert();
        R.Get('A');
        asserterror Codeunit.Run(Codeunit::"RF Runner", R);
        exit(StrSubstNo('%1 [%2] %3', R.Amount, R.GetFilters(), R.CurrentKey()));
    end;
}
"#;

#[test]
fn a_failed_run_with_its_result_used_leaves_the_callers_record_as_before() {
    assert_eq!(text(run_in(SRC, "C", "Failed")), "No 10 [] Code No");
}

#[test]
fn the_statement_form_under_asserterror_keeps_the_changes() {
    assert_eq!(text(run_in(SRC, "C", "Statement")), "99 [Code: Z] Amount");
}
