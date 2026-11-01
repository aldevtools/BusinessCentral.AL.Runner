//! `FieldRef.Value <> FieldRef.Value` of two Blob fields compares what each record variable holds: without CalcFields only
//! whether the stored row has content, with CalcFields (or after a write) the bytes, and a loaded value against one that is not
//! differs (recordref/lt-y49-blob-compare/Y49BlobCompare.al:120-277, corpus master). An empty loaded Blob against a row without
//! content that is not loaded is not pinned and refused.

const SRC: &str = r#"
table 50717 BRow { fields { field(1; Id; Integer) { } field(2; Bin; Blob) { } field(3; Pic; Media) { } } keys { key(PK; Id) { } } }
codeunit 50771 C {
    local procedure Add(Id: Integer; Content: Text)
    var Row: Record BRow; OutS: OutStream;
    begin
        Row.Id := Id;
        if Content <> '' then begin Row.Bin.CreateOutStream(OutS); OutS.WriteText(Content); end;
        Row.Insert();
    end;
    local procedure Seed()
    begin
        Add(1, 'abc'); Add(2, 'abc'); Add(3, 'xyz'); Add(4, '');
    end;
    local procedure Cmp(var A: Record BRow; var B: Record BRow; No: Integer): Text
    var RA: RecordRef; RB: RecordRef;
    begin
        RA.GetTable(A); RB.GetTable(B);
        exit(Format(RA.Field(No).Value <> RB.Field(No).Value) + '|' + Format(RA.Field(No).Value = RB.Field(No).Value) + ';');
    end;
    procedure CopyInVariable(): Text
    var Rec: Record BRow; X: Record BRow; Res: Text; OutS: OutStream;
    begin
        Seed();
        Rec.Get(1); X := Rec;
        Res := Cmp(Rec, X, 2);
        Rec.CalcFields(Bin); Res += Cmp(Rec, X, 2);
        X.CalcFields(Bin); Res += Cmp(Rec, X, 2);
        Rec.Bin.CreateOutStream(OutS); OutS.WriteText('new'); Res += Cmp(Rec, X, 2);
        exit(Res);
    end;
    procedure Rows(): Text
    var A: Record BRow; B: Record BRow; Res: Text;
    begin
        Seed();
        A.Get(1); B.Get(3); Res := Cmp(A, B, 2);
        B.Get(4); Res += Cmp(A, B, 2);
        A.Get(4); Res += Cmp(A, B, 2);
        A.Get(1); A.CalcFields(Bin); B.Get(3); B.CalcFields(Bin); Res += Cmp(A, B, 2);
        B.Get(2); B.CalcFields(Bin); Res += Cmp(A, B, 2);
        B.Get(3); Res += Cmp(A, B, 2);
        exit(Res);
    end;
    procedure Fresh(): Text
    var A: Record BRow; B: Record BRow; Res: Text; OutS: OutStream;
    begin
        Res := Cmp(A, B, 2) + Cmp(A, B, 3);
        A.Bin.CreateOutStream(OutS); OutS.WriteText('abc'); Res += Cmp(A, B, 2);
        B.Bin.CreateOutStream(OutS); OutS.WriteText('abc'); Res += Cmp(A, B, 2);
        exit(Res);
    end;
    procedure TempAgainstStored(): Text
    var Stored: Record BRow; Temp: Record BRow temporary; Res: Text;
    begin
        Seed();
        Stored.Get(1); Stored.CalcFields(Bin);
        Temp := Stored; Temp.Insert();
        Clear(Stored); Stored.Get(1);
        Res := Cmp(Temp, Stored, 2);
        Temp.Get(1); Res += Cmp(Temp, Stored, 2);
        Temp.CalcFields(Bin); Res += Cmp(Temp, Stored, 2);
        Stored.CalcFields(Bin); Res += Cmp(Temp, Stored, 2);
        Clear(Temp); Temp.Reset(); Temp.DeleteAll();
        Stored.Get(1); Temp.TransferFields(Stored); Temp.Insert();
        Res += Cmp(Temp, Stored, 2);
        exit(Res);
    end;
    procedure EmptyLoadedAgainstNotLoaded(): Text
    var A: Record BRow; B: Record BRow;
    begin
        Seed();
        B.Get(4);
        exit(Cmp(A, B, 2));
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn a_blob_copied_into_a_variable_equals_until_one_side_is_loaded_or_written() {
    assert_eq!(run("CopyInVariable"), "No|Yes;Yes|No;No|Yes;Yes|No;");
}

#[test]
fn blobs_not_loaded_compare_whether_they_have_content_and_loaded_ones_their_bytes() {
    assert_eq!(run("Rows"), "No|Yes;Yes|No;No|Yes;Yes|No;No|Yes;Yes|No;");
}

/// Y49BlobCompare.al:152-187.
#[test]
fn a_temporary_copy_against_the_stored_row() {
    assert_eq!(run("TempAgainstStored"), "No|Yes;No|Yes;Yes|No;No|Yes;Yes|No;");
}

#[test]
fn fresh_records_compare_the_bytes_written() {
    assert_eq!(run("Fresh"), "No|Yes;No|Yes;Yes|No;No|Yes;");
}

#[test]
fn an_empty_loaded_blob_against_an_empty_row_not_loaded_is_refused() {
    match al_testkit::run_in(SRC, "C", "EmptyLoadedAgainstNotLoaded") {
        Err(al_builtins::native::Signal::Unsupported(r)) => assert!(r.contains("Blob"), "{r}"),
        other => panic!("{other:?}"),
    }
}
