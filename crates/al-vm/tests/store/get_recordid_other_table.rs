//! Record.Get with the RecordId of another table (al-runner issue 5795): false with the result used, an error otherwise
//! (record/lt-record-conversion-assumptions/GetRecordId.al:123-164).

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
    var R: Record "LT P98 GetRow"; O: Record "LT P98 GetOther"; T: Text;
    begin
        O.Id := 5; O.Insert();
        ClearLastError();
        if R.Get(O.RecordId) then;
        T := GetLastErrorText() + '|' + GetLastErrorCode() + '#';
        asserterror Error('earlier');
        if R.Get(O.RecordId) then;
        exit(T + GetLastErrorText() + '|' + GetLastErrorCode());
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
fn a_used_get_sets_no_last_error() {
    // record/lt-record-conversion-assumptions/Y8GetRecordIdCaption.al:65-86 Y8_Get_OtherTable_Used_LastError.
    assert_eq!(text(run_in(SRC, "C", "UsedThenLastErrorText")), "|#earlier|Dialog");
}

#[test]
fn the_error_names_the_tables_by_their_captions() {
    // Y8GetRecordIdCaption.al:48-63 Y8_Get_OtherTable_Unused_WithCaptions.
    assert_eq!(
        text(run_in(SRC, "C", "UnusedCaptioned")),
        "A RecordID from table 'Some Caption' cannot be used with a record from table 'LT P98 GetRow'."
    );
}
