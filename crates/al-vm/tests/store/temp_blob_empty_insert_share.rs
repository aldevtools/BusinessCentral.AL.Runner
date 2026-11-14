//! A temporary row inserted while its Blob is empty, then the same record variable moves to another key, writes its Blob and
//! inserts again (al-runner issue 6924, Excel Buffer.ReadSheet). The cells of corpus PR 1391
//! (tests/al-language/streams/TempBlobEmptyInsertShare.al, not pinned yet; assumption `temp-blob-empty-insert-key-change`):
//! the earlier rows keep their empty Blob. The neighbors the probe did not measure refuse where main shared the buffer, and
//! keep main's result where it did not.

const SRC: &str = r#"
table 50990 "Tbs Row"
{
    fields { field(1; Key1; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Key1) { } }
}
table 50991 "Tbs Temp Row"
{
    TableType = Temporary;
    fields { field(1; Key1; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Key1) { } }
}
table 50992 "Tbs Code Row"
{
    fields { field(1; Key1; Code[10]) { } field(2; Data; Blob) { } }
    keys { key(PK; Key1) { } }
}
codeunit 50993 Lib
{
    procedure Write(var R: Record "Tbs Row" temporary; T: Text)
    var O: OutStream;
    begin
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText(T);
    end;
    procedure ReadGet(var R: Record "Tbs Row" temporary; K: Integer): Text
    begin
        if not R.Get(K) then exit('none');
        exit(Content(R));
    end;
    procedure ReadFind(var R: Record "Tbs Row" temporary; K: Integer): Text
    var Result: Text;
    begin
        R.SetRange(Key1, K);
        if R.FindFirst() then Result := Content(R) else Result := 'none';
        R.SetRange(Key1);
        exit(Result);
    end;
    procedure Content(var R: Record "Tbs Row" temporary): Text
    var I: InStream; T: Text;
    begin
        R.CalcFields(Data);
        if not R.Data.HasValue() then exit('nohv');
        R.Data.CreateInStream(I, TextEncoding::UTF8);
        I.ReadText(T);
        exit('hv:' + T);
    end;
    procedure WriteTT(var R: Record "Tbs Temp Row"; T: Text)
    var O: OutStream;
    begin
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText(T);
    end;
    procedure ReadGetTT(var R: Record "Tbs Temp Row"; K: Integer): Text
    var I: InStream; T: Text;
    begin
        if not R.Get(K) then exit('none');
        R.CalcFields(Data);
        if not R.Data.HasValue() then exit('nohv');
        R.Data.CreateInStream(I, TextEncoding::UTF8);
        I.ReadText(T);
        exit('hv:' + T);
    end;
}
codeunit 50994 C
{
    var Lib: Codeunit Lib;

    // TempBlobEmptyInsertShare.al:142
    procedure KeyChangeClearWriteInsert(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; Clear(R.Data); Lib.Write(R, 'X'); R.Insert();
        exit(Lib.ReadGet(R, 1) + '|' + Lib.ReadGet(R, 2) + '|' + Lib.ReadFind(R, 1) + '|' + Lib.ReadFind(R, 2) + '|' + Format(R.Count()));
    end;

    // :167
    procedure ClearInsertEmptyKeyChange(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; Clear(R.Data); R.Insert();
        R.Key1 := 2; Clear(R.Data); Lib.Write(R, 'X'); R.Insert();
        exit(Lib.ReadFind(R, 1) + '|' + Lib.ReadFind(R, 2));
    end;

    // :188
    procedure KeyChangeWriteInsertNoClear(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; Lib.Write(R, 'X'); R.Insert();
        exit(Lib.ReadGet(R, 1) + '|' + Lib.ReadGet(R, 2));
    end;

    // :206
    procedure KeyChangeClearWriteNoInsert(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; Clear(R.Data); Lib.Write(R, 'X');
        exit(Lib.Content(R) + '|' + Lib.ReadGet(R, 1) + '|' + Lib.ReadGet(R, 2));
    end;

    // :226
    procedure KeyChangeWriteNoClearNoInsert(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; Lib.Write(R, 'X');
        exit(Lib.ReadGet(R, 1));
    end;

    // :241
    procedure SameKeyClearWrite(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        Clear(R.Data); Lib.Write(R, 'X');
        exit(Lib.ReadGet(R, 1));
    end;

    // :257
    procedure SameKeyWriteNoClear(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        Lib.Write(R, 'X');
        exit(Lib.ReadGet(R, 1));
    end;

    // :283
    procedure TwoEmptyRowsThirdWritten(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; R.Insert();
        R.Key1 := 3; Clear(R.Data); Lib.Write(R, 'X'); R.Insert();
        exit(Lib.ReadGet(R, 1) + '|' + Lib.ReadGet(R, 2) + '|' + Lib.ReadGet(R, 3));
    end;

    // :306
    procedure WrittenRowThenClearInsert(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; Clear(R.Data); Lib.Write(R, 'X'); R.Insert();
        R.Key1 := 3; Clear(R.Data); R.Insert();
        exit(Lib.ReadGet(R, 1) + '|' + Lib.ReadGet(R, 2) + '|' + Lib.ReadGet(R, 3));
    end;

    // :330
    procedure WrittenRowThenClearWriteInsert(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; Clear(R.Data); Lib.Write(R, 'X'); R.Insert();
        R.Key1 := 3; Clear(R.Data); Lib.Write(R, 'Y'); R.Insert();
        exit(Lib.ReadGet(R, 1) + '|' + Lib.ReadGet(R, 2) + '|' + Lib.ReadGet(R, 3));
    end;

    // :366
    procedure ValidateKey(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Validate(Key1, 1); R.Insert();
        R.Validate(Key1, 2); Clear(R.Data); Lib.Write(R, 'X'); R.Insert();
        exit(Lib.ReadGet(R, 1) + '|' + Lib.ReadGet(R, 2));
    end;

    // :385
    procedure InitThenKeyClear(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Init(); R.Key1 := 1; R.Insert();
        R.Init(); R.Key1 := 2; Clear(R.Data); Lib.Write(R, 'X'); R.Insert();
        exit(Lib.ReadGet(R, 1) + '|' + Lib.ReadGet(R, 2));
    end;

    // :406
    procedure InitThenKeyNoClear(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Init(); R.Key1 := 1; R.Insert();
        R.Init(); R.Key1 := 2; Lib.Write(R, 'X'); R.Insert();
        exit(Lib.ReadGet(R, 1) + '|' + Lib.ReadGet(R, 2));
    end;

    // :426
    procedure SharedCopyInsert(): Text
    var R: Record "Tbs Row" temporary; R2: Record "Tbs Row" temporary;
    begin
        R.DeleteAll(); R2.Copy(R, true);
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; Clear(R.Data); Lib.Write(R, 'X'); R.Insert();
        exit(Lib.ReadGet(R2, 1) + '|' + Lib.ReadGet(R2, 2));
    end;

    // :447
    procedure SharedCopyNoInsert(): Text
    var R: Record "Tbs Row" temporary; R2: Record "Tbs Row" temporary;
    begin
        R.DeleteAll(); R2.Copy(R, true);
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; Clear(R.Data); Lib.Write(R, 'X');
        exit(Lib.ReadGet(R2, 1));
    end;

    // :465
    procedure TempTableClear(): Text
    var R: Record "Tbs Temp Row";
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; Clear(R.Data); Lib.WriteTT(R, 'X'); R.Insert();
        exit(Lib.ReadGetTT(R, 1) + '|' + Lib.ReadGetTT(R, 2));
    end;

    // :484
    procedure TempTableNoClear(): Text
    var R: Record "Tbs Temp Row";
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; Lib.WriteTT(R, 'X'); R.Insert();
        exit(Lib.ReadGetTT(R, 1) + '|' + Lib.ReadGetTT(R, 2));
    end;

    // :502
    procedure TempTableSameKey(): Text
    var R: Record "Tbs Temp Row";
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        Clear(R.Data); Lib.WriteTT(R, 'X');
        exit(Lib.ReadGetTT(R, 1));
    end;

    // Not measured: a write after the second empty Insert, with an earlier row inserted empty (main: both rows read it).
    procedure WriteAfterSecondEmptyInsert(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; R.Insert();
        Lib.Write(R, 'X');
        exit(Lib.ReadGet(R, 1) + '|' + Lib.ReadGet(R, 2));
    end;

    // Not measured: a write after an empty Insert under the same key, then a key change and a Clear (main: row 1 emptied).
    procedure SameKeyWriteThenKeyChangeClear(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        Lib.Write(R, 'X');
        R.Key1 := 2; Clear(R.Data);
        exit(Lib.ReadGet(R, 1));
    end;

    // Not measured: a write after an empty Insert under the same key, then a key change and another write.
    procedure SameKeyWriteThenKeyChangeWrite(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        Lib.Write(R, 'X');
        R.Key1 := 2; Lib.Write(R, 'Y');
        exit(Lib.ReadGet(R, 1));
    end;

    // Not measured: a Code key (the probe measured Integer keys).
    procedure CodeKeyChangeWrite(): Text
    var R: Record "Tbs Code Row" temporary; O: OutStream;
    begin
        R.Key1 := 'A'; R.Insert();
        R.Key1 := 'B'; R.Data.CreateOutStream(O); O.WriteText('X'); R.Insert();
        R.Get('A'); R.CalcFields(Data);
        exit(Format(R.Data.HasValue()));
    end;

    // Not measured: Blob.Import into the shared buffer under another key.
    procedure ImportAfterKeyChange(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; R.Data.Import('a.txt');
        exit(Lib.ReadGet(R, 1));
    end;

    // Main's result kept: a stored table copies the Blob at Insert.
    procedure StoredKeyChangeWrite(): Text
    var R: Record "Tbs Row"; O: OutStream; I: InStream; T: Text;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; R.Data.CreateOutStream(O); O.WriteText('X'); R.Insert();
        R.Get(1); R.CalcFields(Data);
        exit(Format(R.Data.HasValue()));
    end;

    // Main's result kept: two empty rows, no write.
    procedure TwoEmptyRowsNoWrite(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.DeleteAll();
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; Clear(R.Data); R.Insert();
        exit(Lib.ReadGet(R, 1) + '|' + Lib.ReadGet(R, 2));
    end;
}
"#;

fn run(proc_name: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", proc_name))
}

#[test]
fn measured_cells_keep_the_earlier_rows_empty() {
    let cells: &[(&str, &str)] = &[
        ("KeyChangeClearWriteInsert", "nohv|hv:X|nohv|hv:X|2"),
        ("ClearInsertEmptyKeyChange", "nohv|hv:X"),
        ("KeyChangeWriteInsertNoClear", "nohv|hv:X"),
        ("KeyChangeClearWriteNoInsert", "nohv|nohv|none"),
        ("KeyChangeWriteNoClearNoInsert", "nohv"),
        ("SameKeyClearWrite", "hv:X"),
        ("SameKeyWriteNoClear", "hv:X"),
        ("TwoEmptyRowsThirdWritten", "nohv|nohv|hv:X"),
        ("WrittenRowThenClearInsert", "nohv|hv:X|nohv"),
        ("WrittenRowThenClearWriteInsert", "nohv|hv:X|hv:Y"),
        ("ValidateKey", "nohv|hv:X"),
        ("InitThenKeyClear", "nohv|hv:X"),
        ("InitThenKeyNoClear", "nohv|hv:X"),
        ("SharedCopyInsert", "nohv|hv:X"),
        ("SharedCopyNoInsert", "nohv"),
        ("TempTableClear", "nohv|hv:X"),
        ("TempTableNoClear", "nohv|hv:X"),
        ("TempTableSameKey", "hv:X"),
    ];
    let mut wrong = Vec::new();
    for (name, want) in cells {
        let got = run(name);
        if got != *want {
            wrong.push(format!("{name}: got {got}, want {want}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

const REFUSED: &str = "a write into the Blob Data of a temporary record of";

fn refused(proc_name: &str) -> String {
    match al_testkit::run_in(SRC, "C", proc_name) {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{proc_name}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_write_after_the_second_empty_insert_refuses() {
    // Main read 'hv:X|hv:X': row 1 shared the write made under key 2.
    assert!(refused("WriteAfterSecondEmptyInsert").starts_with(REFUSED));
}

#[test]
fn a_clear_of_content_shared_with_a_row_under_another_key_refuses() {
    // Main emptied row 1, which held 'X' written under its own key.
    assert!(refused("SameKeyWriteThenKeyChangeClear").starts_with(REFUSED));
}

#[test]
fn a_write_into_content_shared_with_a_row_under_another_key_refuses() {
    // Main read 'hv:Y' in row 1.
    assert!(refused("SameKeyWriteThenKeyChangeWrite").starts_with(REFUSED));
}

#[test]
fn a_key_of_another_type_refuses() {
    // Main read Yes: row 'A' shared the write made under 'B'.
    assert!(refused("CodeKeyChangeWrite").starts_with(REFUSED));
}

#[test]
fn blob_import_under_another_key_refuses() {
    assert!(refused("ImportAfterKeyChange").starts_with(REFUSED));
}

#[test]
fn shapes_outside_the_cells_keep_mains_result() {
    assert_eq!(run("StoredKeyChangeWrite"), "No");
    assert_eq!(run("TwoEmptyRowsNoWrite"), "nohv|nohv");
}
