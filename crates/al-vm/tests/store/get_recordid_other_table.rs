//! Record.Get with the RecordId of another table (al-runner issue 5795): false with the result used, an error otherwise
//! (record/lt-record-conversion-assumptions/GetRecordId.al:123-164).

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 70338 "LT P98 GetRow"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}
table 70339 "LT P98 GetOther"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}
table 70337 "Captioned Row"
{
    Caption = 'Some Caption';
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}
codeunit 70340 C
{
    [TryFunction]
    local procedure TryGet(var R: Record "LT P98 GetRow"; Id: RecordId)
    begin
        R.Get(Id);
    end;

    procedure Used(): Text
    var R: Record "LT P98 GetRow"; O: Record "LT P98 GetOther"; Ok: Boolean;
    begin
        R.Id := 5; R.Insert();
        O.Id := 5; O.Insert();
        Ok := R.Get(O.RecordId);
        exit(Format(Ok));
    end;

    procedure UsedThenLastErrorText(): Text
    var R: Record "LT P98 GetRow"; O: Record "LT P98 GetOther";
    begin
        O.Id := 5; O.Insert();
        if R.Get(O.RecordId) then;
        exit(GetLastErrorText());
    end;

    procedure Unused(): Text
    var R: Record "LT P98 GetRow"; O: Record "LT P98 GetOther";
    begin
        O.Id := 5; O.Insert();
        asserterror R.Get(O.RecordId);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure InTry(): Text
    var R: Record "LT P98 GetRow"; O: Record "LT P98 GetOther"; Ok: Boolean;
    begin
        O.Id := 5; O.Insert();
        Ok := TryGet(R, O.RecordId);
        exit(Format(Ok) + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure UnusedCaptioned(): Text
    var R: Record "LT P98 GetRow"; O: Record "Captioned Row";
    begin
        O.Id := 5; O.Insert();
        asserterror R.Get(O.RecordId);
        exit(GetLastErrorText());
    end;
}
"#;

const TEXT: &str = "A RecordID from table 'LT P98 GetOther' cannot be used with a record from table 'LT P98 GetRow'.";

#[test]
fn a_used_get_answers_false() {
    // GetRecordId.al:147-164 (P98_Get_RecordId_OtherTable_ResultUsed).
    assert_eq!(text(run_in(SRC, "C", "Used")), "No");
}

#[test]
fn an_unused_get_raises_the_pinned_error() {
    // GetRecordId.al:122-145 (P98_Get_RecordId_OtherTable).
    assert_eq!(text(run_in(SRC, "C", "Unused")), format!("{TEXT}|DB:Ok"));
    assert_eq!(text(run_in(SRC, "C", "InTry")), format!("No|{TEXT}|DB:Ok"));
}

#[test]
fn what_a_used_get_leaves_in_the_last_error_is_not_pinned() {
    let r = run_in(SRC, "C", "UsedThenLastErrorText");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn a_table_with_a_caption_is_not_pinned_by_name_or_caption() {
    let r = run_in(SRC, "C", "UnusedCaptioned");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
