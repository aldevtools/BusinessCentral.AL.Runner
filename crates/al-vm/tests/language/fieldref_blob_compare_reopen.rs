//! `FldRef.Value <> xFldRef.Value` over every field of a row and a second RecordRef opened on the same row by its RecordId,
//! the loop of Change Log Management.LogModification (al-runner issue 6661). Blob compare: recordref/lt-y49-blob-compare/
//! Y49BlobCompare.al:141-283.

const SRC: &str = r#"
table 50717 BRow { fields { field(1; Id; Integer) { } field(2; Bin; Blob) { } field(3; Name; Text[30]) { } } keys { key(PK; Id) { } } }
codeunit 50771 C {
    local procedure Changed(var RecRef: RecordRef): Text
    var xRecRef: RecordRef; FldRef: FieldRef; xFldRef: FieldRef; i: Integer; Res: Text;
    begin
        xRecRef.Open(RecRef.Number);
        xRecRef.Get(RecRef.RecordId);
        for i := 1 to RecRef.FieldCount do begin
            FldRef := RecRef.FieldIndex(i);
            xFldRef := xRecRef.FieldIndex(i);
            if FldRef.Class = FieldClass::Normal then
                if FldRef.Value <> xFldRef.Value then
                    Res += FldRef.Name + ';';
        end;
        exit(Res);
    end;
    procedure Run(): Text
    var Row: Record BRow; RecRef: RecordRef; OutS: OutStream;
    begin
        Row.Id := 1; Row.Bin.CreateOutStream(OutS); OutS.WriteText('abc'); Row.Insert();
        Row.Id := 2; Row.Insert();
        Row.Get(1); Row.Name := 'x';
        RecRef.GetTable(Row);
        exit(Changed(RecRef));
    end;
    procedure RunEmpty(): Text
    var Row: Record BRow; RecRef: RecordRef;
    begin
        Row.Id := 2; Row.Insert();
        Row.Get(2); Row.Name := 'x';
        RecRef.GetTable(Row);
        exit(Changed(RecRef));
    end;
}
"#;

#[test]
fn blob_fields_of_the_same_stored_row_are_equal() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Run")), "Name;");
}

#[test]
fn empty_blob_fields_of_the_same_stored_row_are_equal() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "RunEmpty")), "Name;");
}
