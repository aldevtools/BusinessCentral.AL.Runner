//! RecordRef.Field(FieldName) and RecordRef.FieldExist(FieldName) find a field by its name, case-insensitively and
//! without trimming; a caption is not a name. Field raises FieldNotFound with the supplied text
//! (misc/lt-open-overloads-collections-isolated/P107RecRefNames.al:14-97). A field number a table lacks raises the
//! same error (recordref/lt-refs/RfTMeta.al:90-123), and so does SetLoadFields/AddLoadFields with one unknown number
//! (P107RecRefNames.al:159-171).

const TABLE: &str = "table 50721 \"LT Name Rec\" { fields { field(1; \"Code\"; Code[20]) { Caption = 'Code Cap'; }
    field(2; Name; Text[50]) { Caption = 'Name Cap'; } field(3; Qty; Integer) { } } keys { key(PK; \"Code\") { } } }";

fn run(body: &str) -> String {
    let src = format!(
        "{TABLE} codeunit 50722 C {{ procedure F(): Text var RecRef: RecordRef; FldRef: FieldRef; s: Text; Nm: Text; begin
            RecRef.Open(50721); {body} end; }}"
    );
    al_testkit::text(al_testkit::run_in(&src, "C", "F"))
}

#[test]
fn field_by_name_is_case_insensitive() {
    assert_eq!(
        run("Nm := 'Qty'; FldRef := RecRef.Field(Nm); s := FldRef.Name + '|';
            s += Format(RecRef.Field('Name').Number) + Format(RecRef.Field('name').Number) + Format(RecRef.Field('NAME').Number);
            exit(s);"),
        "Qty|222"
    );
}

#[test]
fn field_by_unknown_name_raises_field_not_found() {
    assert_eq!(
        run(
            "asserterror RecRef.Field('Code Cap'); s := GetLastErrorText() + '|' + GetLastErrorCode() + '|';
            asserterror RecRef.Field(''); s += GetLastErrorText() + '|';
            asserterror RecRef.Field(' Name'); s += GetLastErrorText();
            exit(s);"
        ),
        "The supplied field number 'Code Cap' cannot be found in the 'LT Name Rec' table.|FieldNotFound|\
         The supplied field number '' cannot be found in the 'LT Name Rec' table.|\
         The supplied field number ' Name' cannot be found in the 'LT Name Rec' table."
    );
}

#[test]
fn field_exist_by_name() {
    assert_eq!(
        run("exit(Format(RecRef.FieldExist('Name')) + Format(RecRef.FieldExist('QTY')) + Format(RecRef.FieldExist('Name Cap'))
            + Format(RecRef.FieldExist(' Name ')) + Format(RecRef.FieldExist('')) + Format(RecRef.FieldExist('1')));"),
        "YesYesNoNoNoNo"
    );
}

#[test]
fn field_by_unknown_number_raises_field_not_found() {
    assert_eq!(
        run("asserterror RecRef.Field(2147483647); s := GetLastErrorText() + '|';
            asserterror RecRef.Field(23); exit(s + GetLastErrorText() + '|' + GetLastErrorCode());"),
        "The supplied field number '2147483647' cannot be found in the 'LT Name Rec' table.|\
         The supplied field number '23' cannot be found in the 'LT Name Rec' table.|FieldNotFound"
    );
}

#[test]
fn load_fields_with_an_unknown_number_raise_field_not_found() {
    assert_eq!(
        run(
            "asserterror RecRef.SetLoadFields(99); s := GetLastErrorText() + '|' + GetLastErrorCode() + '|';
            asserterror RecRef.AddLoadFields(99); exit(s + GetLastErrorText());"
        ),
        "The supplied field number '99' cannot be found in the 'LT Name Rec' table.|FieldNotFound|\
         The supplied field number '99' cannot be found in the 'LT Name Rec' table."
    );
}

/// SetTable(Rec, ShareTable) (P107RecRefNames.al:175-253).
#[test]
fn set_table_with_share_table() {
    let src = format!(
        "{TABLE} table 50723 \"LT Name Dest\" {{ fields {{ field(1; \"Code\"; Code[20]) {{ }} }} keys {{ key(PK; \"Code\") {{ }} }} }}
        codeunit 50724 C {{
        procedure Normal(): Text var RecRef: RecordRef; Rec: Record \"LT Name Rec\"; Rec2: Record \"LT Name Rec\"; Dest: Record \"LT Name Dest\"; s: Text; begin
            Rec.Code := 'C'; Rec.Insert();
            RecRef.Open(50721); RecRef.FindLast(); RecRef.SetTable(Rec, false); s := Rec.Code + '|';
            asserterror RecRef.SetTable(Rec2, true); s += GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Rec2.Code + '|';
            asserterror RecRef.SetTable(Dest, true); s += GetLastErrorCode() + '|';
            asserterror RecRef.SetTable(Dest, false); exit(s + GetLastErrorCode()); end;
        procedure Temp(): Text var RecRef: RecordRef; Tmp: Record \"LT Name Rec\" temporary; Target: Record \"LT Name Rec\" temporary; begin
            Tmp.Code := 'T1'; Tmp.Insert(); Tmp.Code := 'T2'; Tmp.Insert(); RecRef.GetTable(Tmp);
            Target.Code := 'X'; Target.Insert();
            RecRef.SetTable(Target, true); exit(Format(Target.Count()) + Format(Target.IsTemporary())); end; }}"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src, "C", "Normal")),
        "C|The COPY function can only be used with the shareTable argument set to true if both records are temporary.|Argument||TableIdMismatch|TableIdMismatch"
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "Temp")), "2Yes");
}

/// RecordRef.Open with a Variant takes Open(Integer): an Integer, a BigInteger and a numeric Text open the table, a table
/// name raises TypeConversion:Format, 0 opens nothing (P107RecRefNames.al:255-305).
#[test]
fn open_with_a_variant() {
    let src = format!(
        "{TABLE} codeunit 50725 C {{
        procedure F(): Text var V: Variant; Big: BigInteger; s: Text; begin
            V := 50721; s := Try(V) + ',';
            Big := 50721; V := Big; s += Try(V) + ',';
            V := '50721'; s += Try(V) + ',';
            V := 'LT Name Rec'; s += Try(V) + ',';
            V := 0; exit(s + Try(V)); end;
        local procedure Try(V: Variant): Text var Opened: Text; begin
            ClearLastError();
            if TryOpen(V, Opened) then exit('ok|' + Opened);
            exit('error|' + Opened + '|' + GetLastErrorText() + '|' + GetLastErrorCode()); end;
        [TryFunction] local procedure TryOpen(V: Variant; var Opened: Text) var RecRef: RecordRef; begin
            Opened := 'before open'; RecRef.Open(V); Opened := 'opened';
            Opened := Format(RecRef.IsTemporary()) + '|' + Format(RecRef.Number()) + '|' + RecRef.Name(); end; }}"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&src, "C", "F")),
        "ok|No|50721|LT Name Rec,ok|No|50721|LT Name Rec,ok|No|50721|LT Name Rec,\
         error|before open|The expression Variant cannot be type-converted to a Integer value.|TypeConversion:Format,\
         error|opened|The record is not open.|RecordNotOpened"
    );
}

/// FieldIndex(0) is the timestamp field, number 0 (misc/lt-probes-needed-4/PnD12_RemovedFieldRecordRef.al:120-136).
#[test]
fn field_index_zero_is_the_timestamp() {
    assert_eq!(
        run("FldRef := RecRef.FieldIndex(0); exit(FldRef.Name + '/' + Format(FldRef.Number));"),
        "timestamp/0"
    );
}
