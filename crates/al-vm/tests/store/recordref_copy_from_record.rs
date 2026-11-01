//! RecordRef.Copy(Record) into an opened RecordRef takes the record's filters, key and view, as Copy from a RecordRef does
//! (misc/lt-probes-needed-6/PnF27RecordRefCopy.al:117-158 Copy_FromRecord, Copy_FromRecord_WithView). Into a RecordRef opened
//! as temporary, from a Record that is temporary or not, it stays temporary (record/lt-y51-recordref-copy-temp/Y51RecRefCopyTemp.al).
//! A temporary Record into a non-temporary RecordRef is not pinned and stays refused.

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
fn a_temporary_record_into_a_non_temporary_ref_stays_refused() {
    assert!(matches!(run(false, true), Err(Signal::Unsupported(_))));
}

/// The Quality Management loop: Copy(Row, false) then Insert into a temporary RecordRef (Y51RecRefCopyTemp.al,
/// RecRefCopy_RecordIntoTemporary_LoopInsert: the ref stays temporary, takes the filter, the real table keeps its rows).
#[test]
fn copy_from_a_record_into_a_temporary_ref_then_insert() {
    let src = r#"
table 69982 "RCR Row" { fields { field(1; Id; Integer) { } field(2; Grp; Integer) { } } keys { key(PK; Id) { } key(G; Grp, Id) { } } }
codeunit 69983 C
{
    procedure F(): Text
    var Row: Record "RCR Row"; Db: Record "RCR Row"; Dst: RecordRef; I: Integer; Ids: Text;
    begin
        for I := 1 to 5 do begin Row.Id := I; Row.Grp := (I + 1) div 2; Row.Insert(); end;
        Row.SetRange(Grp, 1, 2);
        Dst.Open(Database::"RCR Row", true);
        Row.FindSet();
        repeat
            Dst.Copy(Row, false);
            Dst.Insert();
        until Row.Next() = 0;
        if Dst.FindSet() then
            repeat Ids += Format(Dst.Field(1).Value) + ','; until Dst.Next() = 0;
        exit(Format(Dst.IsTemporary()) + '|' + Format(Dst.Count()) + '|' + Dst.GetFilters() + '|' + Ids + '|' + Format(Db.Count()));
    end;
}
"#;
    assert_eq!(text(al_testkit::run_in(src, "C", "F")), "Yes|4|Grp: 1..2|1,2,3,4,|5");
}
