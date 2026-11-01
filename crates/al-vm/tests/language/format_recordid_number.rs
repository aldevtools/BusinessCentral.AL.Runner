//! Format of a RecordId with a standard format number: 0, 1, 2 and 9 are pinned, 3 to 8 raise
//! (types/lt-implicit-conversions/P79RecordId.al:183-200, :330-346). Any other number, 10 (the bookmark form, gap G5-10) included, is
//! not pinned and refused.

use al_builtins::native::Signal;

fn run(n: i32) -> Result<al_builtins::value::Value, Signal> {
    let src = format!(
        r#"
table 69986 "FRN Row" {{ fields {{ field(1; "No."; Code[20]) {{ }} }} keys {{ key(PK; "No.") {{ }} }} }}
codeunit 69987 C
{{
    procedure F(): Text
    var R: Record "FRN Row";
    begin
        R."No." := 'A-1'; R.Insert();
        exit(Format(R.RecordId, 0, {n}));
    end;
}}
"#
    );
    al_testkit::run_in(&src, "C", "F")
}

#[test]
fn format_numbers_zero_and_nine_format_the_key() {
    assert_eq!(al_testkit::text(run(0)), "FRN Row: A-1");
    assert_eq!(al_testkit::text(run(9)), "FRN Row: A-1");
}

#[test]
fn format_number_ten_and_others_are_refused() {
    for n in [10, 11, -1] {
        assert!(matches!(run(n), Err(Signal::Unsupported(_))), "{n}");
    }
}
