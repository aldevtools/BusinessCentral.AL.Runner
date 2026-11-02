//! The Date filter letter `Y` is the fiscal year of the work date from the Accounting Period table, and `T-nD` the Tuesday of ISO
//! week n of the ISO year of the work date (writer Y81, record/lt-y81-values/Y81DateFilterLetters.al:126 Y81_Y_FiscalYearOfWorkDate,
//! :274 Y81_Letters_TFamily; needs corpus bump). Rows from 2060 stand in for the test's own periods: the corpus company's demo
//! rows end in 2027, so a fiscal year that starts later is decided by the rows the runtime holds.

use al_testkit::{run, text};

const SRC: &str = r#"
table 50 "Accounting Period"
{
    fields { field(1; "Starting Date"; Date) { } field(2; Name; Text[10]) { } field(3; "New Fiscal Year"; Boolean) { } }
    keys { key(PK; "Starting Date") { Clustered = true; } }
}
table 69972 "Y DF"
{
    fields { field(1; Code; Code[10]) { } field(2; D; Date) { } }
    keys { key(PK; Code) { Clustered = true; } }
}
codeunit 69972 C
{
    local procedure Period(Start: Date; NewYear: Boolean)
    var P: Record "Accounting Period";
    begin
        P."Starting Date" := Start;
        P."New Fiscal Year" := NewYear;
        P.Insert();
    end;

    [TryFunction]
    local procedure TryY(Txt: Text; var R: Text)
    var Row: Record "Y DF";
    begin
        Row.SetFilter(D, Txt);
        R := Row.GetFilter(D) + '|' + Format(Row.GetRangeMin(D), 0, 9) + '|' + Format(Row.GetRangeMax(D), 0, 9);
    end;

    local procedure At(WorkDay: Date; Txt: Text): Text
    var R: Text;
    begin
        WorkDate(WorkDay);
        ClearLastError();
        if TryY(Txt, R) then exit(R);
        exit('E ' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure Fiscal(): Text
    begin
        Period(20600101D, true);
        Period(20600201D, false);
        Period(20610401D, true);
        Period(20610501D, false);
        Period(20620101D, true);
        exit(At(20600615D, 'Y') + '#' + At(20610331D, 'Y') + '#' + At(20610401D, 'Y') + '#' + At(20620101D, 'Y') + '#' + At(20700101D, 'Y'));
    end;

    procedure Tuesdays(): Text
    begin
        exit(At(20240615D, 'T-1D') + '#' + At(20260615D, 'T-1D') + '#' + At(20260101D, 'T-1D') + '#' + At(20260615D, 'T-3D') + '#' + At(20260615D, 'T-D') + '#' + At(20300515D, 'T-1D'));
    end;
}
"#;

#[test]
fn y_is_the_fiscal_year_of_the_work_date_from_the_accounting_periods() {
    assert_eq!(
        text(run(SRC, "Fiscal")),
        "01/01/2060..03/31/2061|2060-01-01|2061-03-31#01/01/2060..03/31/2061|2060-01-01|2061-03-31#04/01/2061..12/31/2061|2061-04-01|2061-12-31#\
         01/01/2062..12/31/9999|2062-01-01|9999-12-31#01/01/2062..12/31/9999|2062-01-01|9999-12-31"
    );
}

#[test]
fn t_minus_n_d_is_the_tuesday_of_the_iso_week_n_of_the_work_date() {
    assert_eq!(
        text(run(SRC, "Tuesdays")),
        "01/02/24|2024-01-02|2024-01-02#12/30/25|2025-12-30|2025-12-30#12/30/25|2025-12-30|2025-12-30#\
         01/13/26|2026-01-13|2026-01-13#06/16/26|2026-06-16|2026-06-16#01/01/30|2030-01-01|2030-01-01"
    );
}
