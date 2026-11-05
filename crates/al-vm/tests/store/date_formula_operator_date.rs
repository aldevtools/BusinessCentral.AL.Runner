//! `'<=' + Format(Date)` where a DateFormula reads it (al-runner issue 6998): SetFilter on a DateFormula field and Evaluate into a
//! DateFormula raise the time-unit error, Evaluate with its result used answers false; a formula without brackets is a DateFormula
//! filter value (record/filters/DateFormulaOperatorFilterText.al, staging r6980).

use al_testkit::{error_message, text};

const SRC: &str = r#"
table 69910 Row
{
    fields { field(1; Id; Integer) { } field(2; DF; DateFormula) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69910 C
{
    local procedure Add(Id: Integer; F: Text)
    var R: Record Row;
    begin
        R.Id := Id; Evaluate(R.DF, F); R.Insert();
    end;

    local procedure Ids(var R: Record Row): Text
    var Result: Text;
    begin
        if not R.FindSet() then exit('none');
        repeat Result += Format(R.Id); until R.Next() = 0;
        exit(Result);
    end;

    procedure Steps(): Text
    var R: Record Row; Out: Text;
    begin
        Add(1, ''); Add(2, '<1D>'); Add(3, '<1M>'); Add(4, '<-1W>');
        R.SetFilter(DF, '1M');
        Out := R.GetFilter(DF) + ' ' + Ids(R);
        R.SetFilter(DF, '<=1M');
        Out += ' | ' + R.GetFilter(DF) + ' ' + Ids(R);
        R.SetFilter(DF, '<>1M');
        Out += ' | ' + R.GetFilter(DF) + ' ' + Ids(R);
        exit(Out);
    end;

    procedure FilterDate()
    var R: Record Row;
    begin
        R.SetFilter(DF, '<=' + Format(20241104D));
    end;

    procedure EvaluateUsed(): Text
    var F: DateFormula;
    begin
        exit(Format(Evaluate(F, '<=' + Format(20241104D))));
    end;

    procedure EvaluateStatement()
    var F: DateFormula;
    begin
        Evaluate(F, '<=' + Format(20241104D));
    end;
}
"#;

const UNITS: &str = "\\\\Time units can be: D,WD,W,M,Q, or Y.\\C, meaning current time unit based on date, can be specified as a prefix to the time unit.";

#[test]
fn a_step_without_brackets_filters_the_stored_formulas() {
    assert_eq!(text(al_testkit::run(SRC, "Steps")), "1M 3 | <=1M 1234 | <>1M 124");
}

#[test]
fn less_or_equal_and_a_date_on_a_date_formula_field_raises() {
    assert_eq!(
        error_message(al_testkit::run(SRC, "FilterDate")),
        format!(
            "The filter \"<=11/04/24\" is not valid for the DF field on the Row table. The date formula '11/04/24' should include a time unit.{UNITS}"
        )
    );
}

#[test]
fn evaluate_of_less_or_equal_and_a_date() {
    assert_eq!(text(al_testkit::run(SRC, "EvaluateUsed")), "No");
    assert_eq!(
        error_message(al_testkit::run(SRC, "EvaluateStatement")),
        format!("The date formula '<=11/04/24' should include a time unit.{UNITS}")
    );
}
