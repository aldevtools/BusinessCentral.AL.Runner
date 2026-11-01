//! RecordRef.Copy(Record) into an opened RecordRef takes the record's filters, key and view, as Copy from a RecordRef does
//! (misc/lt-probes-needed-6/PnF27RecordRefCopy.al:117-158 Copy_FromRecord, Copy_FromRecord_WithView). A temporary target or
//! source is not pinned (gap G5-15) and stays refused.

use al_builtins::native::Signal;
use al_testkit::text;

fn run(dst_temp: bool, src_temp: bool) -> Result<al_builtins::value::Value, Signal> {
    let src = format!(
        r#"
table 69982 "RCR Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Grp; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} key(G; Grp, Id) {{ }} }} }}
codeunit 69983 C
{{
    procedure F(): Text
    var Row: Record "RCR Row"; Tmp: Record "RCR Row" temporary; Dst: RecordRef; I: Integer;
    begin
        for I := 1 to 9 do begin Row.Id := I; Row.Grp := 10 - I; Row.Insert(); end;
        Row.SetRange(Grp, 6, 8);
        Row.SetCurrentKey(Grp, Id);
        Dst.Open(Database::"RCR Row", {dst_temp});
        if {src_temp} then Dst.Copy(Tmp) else Dst.Copy(Row);
        Dst.FindFirst();
        exit(Format(Dst.Count()) + '|' + Dst.GetFilters() + '|' + Format(Dst.CurrentKeyIndex()) + '|' + Format(Dst.Field(1).Value()));
    end;
}}
"#
    );
    al_testkit::run_in(&src, "C", "F")
}

#[test]
fn copy_from_a_record_takes_its_filters_and_key() {
    assert_eq!(text(run(false, false)), "3|Grp: 6..8|2|4");
}

#[test]
fn copy_with_a_temporary_side_stays_refused() {
    assert!(matches!(run(true, false), Err(Signal::Unsupported(_))));
    assert!(matches!(run(false, true), Err(Signal::Unsupported(_))));
}
