//! A `for` loop whose control variable is a Date (misc/lt-system-functions/ControlFlow.al:607,
//! QX_ForLoop_DateControlVariable; the DemoTool's Create Sequence of Actions loops `for ActualDate := ... to ... do`).
//! The loop ends when the variable equals the end bound, else it moves one day by Date arithmetic: an end that is a
//! closing date leaves the day after it, 0D cannot move, and a closing date start raises ClosingDateOperation.

use al_testkit::{error_message, text};

const SRC: &str = r#"
codeunit 69990 C
{
    local procedure Visit(FromDate: Date; ToDate: Date; Down: Boolean): Text
    var D: Date; Visited: Text; Count: Integer;
    begin
        if Down then
            for D := FromDate downto ToDate do begin Count += 1; if Count <= 6 then Visited += Format(D, 0, 9) + ','; end
        else
            for D := FromDate to ToDate do begin Count += 1; if Count <= 6 then Visited += Format(D, 0, 9) + ','; end;
        exit('count=' + Format(Count) + ' first=' + Visited + ' after=' + Format(D, 0, 9));
    end;

    procedure Boundary(): Text begin exit(Visit(DMY2Date(30, 12, 1902), DMY2Date(2, 1, 1903), false)); end;
    procedure Downto(): Text begin exit(Visit(DMY2Date(3, 1, 2024), DMY2Date(30, 12, 2023), true)); end;
    procedure Empty(): Text begin exit(Visit(DMY2Date(5, 1, 2024), DMY2Date(1, 1, 2024), false)); end;
    procedure ZeroToZero(): Text begin exit(Visit(0D, 0D, false)); end;
    procedure ZeroToDate(): Text begin exit(Visit(0D, DMY2Date(2, 1, 1), false)); end;
    procedure DowntoZero(): Text begin exit(Visit(DMY2Date(2, 1, 1), 0D, true)); end;
    procedure ToClosing(): Text begin exit(Visit(DMY2Date(30, 12, 2023), ClosingDate(DMY2Date(31, 12, 2023)), false)); end;
    procedure FromClosing(): Text begin exit(Visit(ClosingDate(DMY2Date(30, 12, 2023)), DMY2Date(1, 1, 2024), false)); end;
}
"#;

fn call(r: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    al_testkit::run_in(SRC, "C", r)
}

#[test]
fn a_date_loop_visits_each_day_and_ends_on_the_end_bound() {
    assert_eq!(
        text(call("Boundary")),
        "count=4 first=1902-12-30,1902-12-31,1903-01-01,1903-01-02, after=1903-01-02"
    );
    assert_eq!(
        text(call("Downto")),
        "count=5 first=2024-01-03,2024-01-02,2024-01-01,2023-12-31,2023-12-30, after=2023-12-30"
    );
    assert_eq!(text(call("Empty")), "count=0 first= after=2024-01-05");
    assert_eq!(text(call("ZeroToZero")), "count=1 first=, after=");
    assert_eq!(text(call("ToClosing")), "count=2 first=2023-12-30,2023-12-31, after=2024-01-01");
}

#[test]
fn a_date_loop_that_cannot_move_raises() {
    assert_eq!(error_message(call("ZeroToDate")), "The date is not valid.");
    assert_eq!(error_message(call("DowntoZero")), "The date is not valid.");
    assert_eq!(
        error_message(call("FromClosing")),
        "The operation cannot be performed on a closing date value."
    );
}
