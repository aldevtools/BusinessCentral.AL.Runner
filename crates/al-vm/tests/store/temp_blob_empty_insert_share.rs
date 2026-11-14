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
table 50995 "Tbs Two Blobs"
{
    fields { field(1; Key1; Integer) { } field(2; Data; Blob) { } field(3; Other; Blob) { } }
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
    var Lib: Codeunit Lib; G: Record "Tbs Row" temporary;

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
    // Not measured: an OutStream created before the key change and written after it (main: row 1 read the write).
    procedure StreamBeforeKeyChangeWriteInsert(): Text
    var R: Record "Tbs Row" temporary; O: OutStream;
    begin
        R.Key1 := 1; R.Insert();
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        R.Key1 := 2; O.WriteText('X'); R.Insert();
        exit(Lib.ReadGet(R, 1));
    end;

    // Not measured: the same without the second Insert.
    procedure StreamBeforeKeyChangeWriteNoInsert(): Text
    var R: Record "Tbs Row" temporary; O: OutStream;
    begin
        R.Key1 := 1; R.Insert();
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        R.Key1 := 2; O.WriteText('X');
        exit(Lib.ReadGet(R, 1));
    end;

    // Not measured: the stream created before the first Insert, written after the key change.
    procedure StreamBeforeInsertWriteAfterKeyChange(): Text
    var R: Record "Tbs Row" temporary; O: OutStream;
    begin
        R.Key1 := 1;
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        R.Insert();
        R.Key1 := 2; O.WriteText('X'); R.Insert();
        exit(Lib.ReadGet(R, 1));
    end;

    // Not measured: a by-value copy of the record writes under another key (main: the caller's row 1 read the write).
    procedure ByValueCopyWrites(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.Key1 := 1; R.Insert();
        R.Key1 := 2; ByValue(R);
        exit(Lib.ReadGet(R, 1));
    end;

    local procedure ByValue(R: Record "Tbs Row" temporary)
    var O: OutStream;
    begin
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('X');
        R.Insert();
    end;

    // Not measured and not told apart from :426 (the entry says so): a second variable takes the buffer with Copy(R, true)
    // after the Insert, moves on and writes; it takes the assumed path.
    procedure SecondVariableWrites(): Text
    var R: Record "Tbs Row" temporary; R2: Record "Tbs Row" temporary;
    begin
        R.Key1 := 1; R.Insert();
        R2.Copy(R, true);
        R2.Key1 := 2; Lib.Write(R2, 'X'); R2.Insert();
        exit(Lib.ReadGet(R, 1));
    end;

    // Not measured: a Variant holds the record while it moves on and writes.
    procedure VariantHolds(): Text
    var R: Record "Tbs Row" temporary; V: Variant;
    begin
        R.Key1 := 1; R.Insert();
        V := R;
        R.Key1 := 2; Lib.Write(R, 'X'); R.Insert();
        exit(Lib.ReadGet(R, 1));
    end;

    // Not measured: an element of a record array.
    procedure ArrayElement(): Text
    var R: array[2] of Record "Tbs Row" temporary; O: OutStream;
    begin
        R[1].Key1 := 1; R[1].Insert();
        R[1].Key1 := 2;
        R[1].Data.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('X');
        R[1].Insert();
        exit(Lib.ReadGet(R[1], 1));
    end;

    // Not measured: a table with a second Blob field.
    procedure SecondBlobField(): Text
    var R: Record "Tbs Two Blobs" temporary; O: OutStream;
    begin
        R.Key1 := 1; R.Insert();
        R.Key1 := 2;
        R.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('X');
        R.Insert();
        R.Get(1); R.CalcFields(Other);
        exit(Format(R.Other.HasValue()));
    end;

    // Not measured: CopyStream into the stream instead of WriteText.
    procedure CopyStreamInto(): Text
    var R: Record "Tbs Row" temporary; Src: Record "Tbs Row" temporary; O: OutStream; I: InStream;
    begin
        Src.Key1 := 9; Lib.Write(Src, 'X');
        Src.Data.CreateInStream(I, TextEncoding::UTF8);
        R.Key1 := 1; R.Insert();
        R.Key1 := 2;
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        CopyStream(O, I);
        R.Insert();
        exit(Lib.ReadGet(R, 1));
    end;

    // Not measured: the record that opened the stream is gone while a row it inserted and a variable sharing its table hold the
    // buffer (main: row 1 read the write).
    procedure StreamOutlivesRecord(): Text
    var O: OutStream;
    begin
        OpenOn(O);
        O.WriteText('X');
        exit(Lib.ReadGet(G, 1));
    end;

    local procedure OpenOn(var O: OutStream)
    var R: Record "Tbs Row" temporary;
    begin
        R.Key1 := 1; R.Insert();
        G.Copy(R, true);
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
    end;

    // Main's result kept: a stream on a record never inserted, written after a key change.
    procedure StreamNoRowKeyChange(): Text
    var R: Record "Tbs Row" temporary; O: OutStream;
    begin
        R.Key1 := 1;
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        R.Key1 := 2; O.WriteText('X'); R.Insert();
        exit(Lib.ReadGet(R, 2));
    end;

    // Main's result kept: a live InStream on the buffer, then a write, no row.
    procedure InStreamThenWriteNoRow(): Text
    var R: Record "Tbs Row" temporary; O: OutStream; I: InStream; T: Text;
    begin
        Lib.Write(R, 'A');
        R.Data.CreateInStream(I, TextEncoding::UTF8);
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('B');
        R.Data.CreateInStream(I, TextEncoding::UTF8);
        I.ReadText(T);
        exit(T);
    end;

    // Main's result kept: a stream written many times after an empty Insert, same key.
    procedure SameKeyStreamWrittenTwice(): Text
    var R: Record "Tbs Row" temporary; O: OutStream;
    begin
        R.Key1 := 1; R.Insert();
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('X'); O.WriteText('Y');
        exit(Lib.ReadGet(R, 1));
    end;

    // Not measured: a by-value copy of a record no row holds writes its Blob (main: 'nohv').
    procedure ByValueCopyNoRow(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.Key1 := 1;
        exit(ByValueWrite(R));
    end;

    local procedure ByValueWrite(R: Record "Tbs Row" temporary): Text
    begin
        Lib.Write(R, 'X');
        exit(Lib.Content(R));
    end;

    // TempBlobEmptyInsertShare.al:527 (Excel Buffer's ParseCellValue): the detached stream written with Write of one Text.
    procedure KeyChangeWriteOfText(): Text
    var R: Record "Tbs Row" temporary; O: OutStream; T: Text;
    begin
        R.Key1 := 1; R.Insert();
        R.Key1 := 2;
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        T := 'X';
        O.Write(T);
        R.Insert();
        exit(Lib.ReadGet(R, 1));
    end;

    // Not measured: the detached stream written with Write of an Integer (main: row 1 read the write).
    procedure KeyChangeWriteOfInteger(): Text
    var R: Record "Tbs Row" temporary; O: OutStream;
    begin
        R.Key1 := 1; R.Insert();
        R.Key1 := 2;
        R.Data.CreateOutStream(O, TextEncoding::UTF8);
        O.Write(5);
        R.Insert();
        exit(Lib.ReadGet(R, 1));
    end;

    // Not measured: a by-value copy of a record whose own row holds the buffer under the same key writes (main: the caller's row
    // read the write).
    procedure ByValueCopySourceRowSameKey(): Text
    var R: Record "Tbs Row" temporary;
    begin
        R.Key1 := 1; R.Insert();
        ByValueWrite(R);
        exit(Lib.ReadGet(R, 1));
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
        ("KeyChangeWriteOfText", "nohv"),
    ];
    let mut wrong = Vec::new();
    for (name, want) in cells {
        let got = match al_testkit::run_in(SRC, "C", name) {
            Ok(al_builtins::value::Value::Text(s)) => s.to_string(),
            other => format!("{other:?}"),
        };
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

#[test]
fn a_stream_created_before_the_key_change_and_written_after_it_refuses() {
    // Main read 'hv:X' in row 1 in all three.
    assert!(refused("StreamBeforeKeyChangeWriteInsert").starts_with(REFUSED));
    assert!(refused("StreamBeforeKeyChangeWriteNoInsert").starts_with(REFUSED));
    assert!(refused("StreamBeforeInsertWriteAfterKeyChange").starts_with(REFUSED));
}

#[test]
fn a_stream_that_outlives_its_record_refuses_while_a_row_holds_the_buffer() {
    assert!(
        refused("StreamOutlivesRecord")
            .starts_with("a write through an OutStream on a Blob of a temporary record that is gone")
    );
}

#[test]
fn a_by_value_copy_that_writes_refuses() {
    // Main read 'hv:X' in the caller's row 1.
    assert!(refused("ByValueCopyWrites").starts_with(REFUSED));
    assert!(refused("ByValueCopySourceRowSameKey").starts_with(REFUSED));
}

#[test]
fn a_by_value_copy_whose_holders_the_caller_accounts_for_keeps_mains_result() {
    // No row holds the buffer; the caller's variable does, and the copy's source accounts for it.
    assert_eq!(run("ByValueCopyNoRow"), "nohv");
}

#[test]
fn a_detached_stream_takes_only_the_measured_writes() {
    assert!(refused("KeyChangeWriteOfInteger").starts_with(REFUSED));
}

#[test]
fn unmeasured_holders_and_places_refuse() {
    // Main read 'hv:X' (or Yes) in row 1 in each.
    for p in ["VariantHolds", "ArrayElement", "CopyStreamInto"] {
        assert!(refused(p).starts_with(REFUSED), "{p}");
    }
    assert!(refused("SecondBlobField").starts_with("a write into the Blob Other of a temporary record of"));
}

#[test]
fn a_second_variable_sharing_the_table_is_not_told_apart() {
    assert_eq!(run("SecondVariableWrites"), "nohv");
}

#[test]
fn streams_without_a_row_under_another_key_keep_mains_result() {
    assert_eq!(run("StreamNoRowKeyChange"), "hv:X");
    assert_eq!(run("InStreamThenWriteNoRow"), "B");
    assert_eq!(run("SameKeyStreamWrittenTwice"), "hv:XY");
}
