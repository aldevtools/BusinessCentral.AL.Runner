//! WorkDate(Date) used as an expression returns the new work date (al-runner issue 6862).
//! Values: session-system/session/R6862WorkDateExpression.al (BC 28.5 and 29.0).

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 50330 T
{
    procedure ReturnsNew(): Text
    var
        Result: Date;
    begin
        WorkDate(20310201D);
        Result := WorkDate(20320315D);
        exit(Format(Result, 0, 9) + ' ' + Format(WorkDate(), 0, 9));
    end;

    procedure Positions(): Text
    var
        V: Variant;
    begin
        V := WorkDate(20320318D);
        exit(Format(Pass(WorkDate(20320317D)), 0, 9) + ' ' + Format(V, 0, 9)
            + ' ' + Format(Wrap(20320316D), 0, 9) + ' ' + Format(WorkDate(WorkDate(20320322D)), 0, 9));
    end;

    procedure Empty(): Text
    begin
        WorkDate(20310201D);
        exit(Format(WorkDate(0D) = Today()) + ' ' + Format(WorkDate() = Today()));
    end;

    local procedure Pass(D: Date): Date
    begin
        exit(D);
    end;

    local procedure Wrap(D: Date): Date
    begin
        exit(WorkDate(D));
    end;
}
"#;

#[test]
fn returns_the_new_date_and_sets_it() {
    assert_eq!(text(run_in(SRC, "T", "ReturnsNew")), "2032-03-15 2032-03-15");
}

#[test]
fn works_in_every_position_of_an_expression() {
    assert_eq!(
        text(run_in(SRC, "T", "Positions")),
        "2032-03-17 2032-03-18 2032-03-16 2032-03-22"
    );
}

#[test]
fn the_empty_date_returns_and_sets_today() {
    assert_eq!(text(run_in(SRC, "T", "Empty")), "Yes Yes");
}
