//! In-memory streams over Blob fields, end to end. Each test names the corpus test that pins it.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const TABLE: &str = r#"
table 60008 "ALT Blob"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Description; Text[100]) { }
        field(3; Data; Blob) { }
    }
    keys { key(PK; "Code") { } }
}
"#;

fn run(vars: &str, body: &str) -> Result<Value, Signal> {
    let src = format!("{TABLE}\ncodeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; }}");
    al_testkit::run_in(&src, "C", "F")
}

fn text(vars: &str, body: &str) -> String {
    match run(vars, body) {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

fn refused(vars: &str, body: &str) -> bool {
    matches!(run(vars, body), Err(Signal::Unsupported(_)))
}

const BASE: &str = "B: Record \"ALT Blob\"; O: OutStream; I: InStream; T: Text;";

macro_rules! base {
    ($extra:expr) => {
        concat!("B: Record \"ALT Blob\"; O: OutStream; I: InStream; T: Text; ", $extra)
    };
}

#[test]
fn write_text_then_read_text_round_trips_through_the_table() {
    // streams/TestInOutStream.al InOutStream_WriteText_ReadText_Roundtrips.
    let t = text(
        BASE,
        "B.Code := 'T1'; B.Data.CreateOutStream(O); O.WriteText('Hello World'); B.Insert();
         B.Get('T1'); B.CalcFields(Data); B.Data.CreateInStream(I); I.ReadText(T); exit(T);",
    );
    assert_eq!(t, "Hello World");
}

#[test]
fn sequential_write_text_calls_are_concatenated() {
    // streams/TestArrayStreamContracts.al InStream_Sequential_WriteText_ConcatenatesContent.
    let t = text(
        BASE,
        "B.Code := 'S1'; B.Data.CreateOutStream(O); O.WriteText('First'); O.WriteText('Second'); B.Insert();
         B.Get('S1'); B.CalcFields(Data); B.Data.CreateInStream(I); I.ReadText(T); exit(T);",
    );
    assert_eq!(t, "FirstSecond");
}

#[test]
fn an_unloaded_blob_reads_empty_until_calc_fields() {
    // streams/TestBlob.al Blob_Get_WithoutCalcFields_InStreamReadsEmpty; TestInOutStream.al
    // InOutStream_WriteText_BlobHasValue (HasValue is true after Get without CalcFields).
    let t = text(
        BASE,
        "B.Code := 'LAZY'; B.Data.CreateOutStream(O); O.WriteText('NotLoaded'); B.Insert();
         B.Get('LAZY'); B.Data.CreateInStream(I); I.ReadText(T);
         exit('[' + T + ']' + Format(B.Data.HasValue()));",
    );
    assert_eq!(t, "[]Yes");
}

#[test]
fn has_value_and_length() {
    // streams/TestBlob.al Blob_HasValue_Empty_ReturnsFalse, _AfterWrite_ReturnsTrue, Blob_Length_AfterWrite_GreaterThanZero.
    let t = text(
        BASE,
        "B.Code := 'B1'; B.Insert(); B.Get('B1'); exit(Format(B.Data.HasValue()));",
    );
    assert_eq!(t, "No");
    let t = text(
        BASE,
        "B.Code := 'B4'; B.Data.CreateOutStream(O); O.WriteText('LengthTest'); B.Insert(); B.Get('B4'); B.CalcFields(Data);
         exit(Format(B.Data.Length() > 0) + Format(B.Data.HasValue()));",
    );
    assert_eq!(t, "YesYes");
}

#[test]
fn eos_reset_position_and_binary_integers() {
    // streams/TestInOutStream.al InOutStream_EOS_EmptyStream_ReturnsTrue, InOutStream_Write_Integer_ReadInteger;
    // TestArrayStreamContracts.al InStream_EOS_AfterFullRead and InStream_ResetPosition_AllowsReread.
    let t = text(
        BASE,
        "B.Code := 'T4'; B.Data.CreateOutStream(O); O.WriteText(''); B.Insert(); B.Get('T4'); B.CalcFields(Data);
         B.Data.CreateInStream(I); I.ReadText(T); exit(Format(I.EOS()));",
    );
    assert_eq!(t, "Yes");
    let t = text(
        base!("N: Integer;"),
        "B.Code := 'T3'; B.Data.CreateOutStream(O); O.Write(42); B.Insert(); B.Get('T3'); B.CalcFields(Data);
         B.Data.CreateInStream(I); I.Read(N); exit(Format(N) + Format(I.EOS()));",
    );
    assert_eq!(t, "42Yes");
    let t = text(
        base!("T2: Text;"),
        "B.Code := 'R1'; B.Data.CreateOutStream(O); O.WriteText('ReadMe'); B.Insert(); B.Get('R1'); B.CalcFields(Data);
         B.Data.CreateInStream(I); I.ReadText(T); I.ResetPosition(); I.ReadText(T2); exit(T + '|' + T2);",
    );
    assert_eq!(t, "ReadMe|ReadMe");
}

#[test]
fn copy_stream_copies_what_the_source_stream_holds() {
    // streams/TestBlob.al Blob_CopyStream_WithoutCalcFields_DestinationIsEmpty.
    let t = text(
        base!("Src: Record \"ALT Blob\"; D: OutStream; DI: InStream; SI: InStream;"),
        "Src.Code := 'CS1'; Src.Data.CreateOutStream(O); O.WriteText('OriginalContent'); Src.Insert();
         Src.Get('CS1'); B.Code := 'CS2'; B.Data.CreateOutStream(D); Src.Data.CreateInStream(SI); CopyStream(D, SI); B.Insert();
         B.Get('CS2'); B.CalcFields(Data); B.Data.CreateInStream(DI); DI.ReadText(T); exit('[' + T + ']');",
    );
    assert_eq!(t, "[]");
    let t = text(
        base!("Src: Record \"ALT Blob\"; D: OutStream; DI: InStream; SI: InStream;"),
        "Src.Code := 'CS1'; Src.Data.CreateOutStream(O); O.WriteText('OriginalContent'); Src.Insert();
         Src.Get('CS1'); Src.CalcFields(Data); B.Code := 'CS2'; B.Data.CreateOutStream(D); Src.Data.CreateInStream(SI); CopyStream(D, SI); B.Insert();
         B.Get('CS2'); B.CalcFields(Data); B.Data.CreateInStream(DI); DI.ReadText(T); exit('[' + T + ']');",
    );
    assert_eq!(t, "[OriginalContent]");
}

#[test]
fn an_uncommitted_blob_write_stays_local_to_a_database_record() {
    // streams/TestBlobUncommittedWriteIsolation.al: second instance reads empty, re-Get discards,
    // Modify persists.
    let t = text(
        base!("R: Record \"ALT Blob\"; RI: InStream; RT: Text;"),
        "B.Init(); B.Code := 'LEAK'; B.Insert(true); B.Data.CreateOutStream(O); O.WriteText('LEAKED-BYTES');
         R.Get('LEAK'); R.CalcFields(Data); R.Data.CreateInStream(RI); RI.ReadText(RT);
         exit(Format(R.Data.HasValue()) + '[' + RT + ']');",
    );
    assert_eq!(t, "No[]");
    let t = text(
        BASE,
        "B.Init(); B.Code := 'REGET'; B.Insert(true); B.Data.CreateOutStream(O); O.WriteText('REGET-BYTES');
         B.Get('REGET'); B.CalcFields(Data); B.Data.CreateInStream(I); I.ReadText(T); exit(Format(B.Data.HasValue()) + '[' + T + ']');",
    );
    assert_eq!(t, "No[]");
    let t = text(
        base!("R: Record \"ALT Blob\"; RI: InStream; RT: Text;"),
        "B.Init(); B.Code := 'COMMIT'; B.Insert(true); B.Data.CreateOutStream(O); O.WriteText('COMMITTED-BYTES'); B.Modify(true);
         R.Get('COMMIT'); R.CalcFields(Data); R.Data.CreateInStream(RI); RI.ReadText(RT);
         exit(Format(R.Data.HasValue()) + RT + Format(StrLen(RT)));",
    );
    assert_eq!(t, "YesCOMMITTED-BYTES15");
}

#[test]
fn a_temporary_record_holds_the_blob_by_reference() {
    // streams/TestBlobUncommittedWriteIsolation.al TempBlob_UncommittedWrite_SameInstanceReGet_KeepsWrite:
    // the temporary store holds the record's own Blob, so an unpersisted write is visible after Get.
    let t = text(
        "B: Record \"ALT Blob\" temporary; O: OutStream; I: InStream; T: Text;",
        "B.Init(); B.Code := 'TREGET'; B.Insert(true); B.Data.CreateOutStream(O); O.WriteText('TEMP-BYTES');
         B.Get('TREGET'); B.CalcFields(Data); B.Data.CreateInStream(I); I.ReadText(T); exit(Format(B.Data.HasValue()) + T);",
    );
    assert_eq!(t, "YesTEMP-BYTES");
}

#[test]
fn calc_fields_keeps_an_uncommitted_in_memory_write() {
    // streams/TestBlobCalcFieldsUncommittedWrite.al.
    let t = text(
        BASE,
        "B.Init(); B.Code := 'UNCOMMITTED'; B.Insert(true); Clear(B.Data); B.Data.CreateOutStream(O); O.WriteText('HELLO-BYTES');
         B.CalcFields(Data); B.Data.CreateInStream(I); I.ReadText(T); exit(T);",
    );
    assert_eq!(t, "HELLO-BYTES");
    let t = text(
        base!("R: Record \"ALT Blob\"; RI: InStream; RT: Text;"),
        "B.Init(); B.Code := 'EMPTY'; B.Insert(true); R.Get('EMPTY'); R.CalcFields(Data);
         R.Data.CreateInStream(RI); RI.ReadText(RT); exit(Format(R.Data.HasValue()) + '[' + RT + ']');",
    );
    assert_eq!(t, "No[]");
}

#[test]
fn an_empty_blob_that_was_not_loaded_has_length_zero() {
    // streams/TestProbeStreamContracts.al:BlobLength_EmptyUnloaded.
    let t = text(
        BASE,
        "B.Code := 'X'; B.Insert(); B.Get('X'); exit(Format(B.Data.Length()) + Format(B.Data.HasValue()));",
    );
    assert_eq!(t, "0No");
}

#[test]
fn write_text_without_a_text_writes_carriage_return_line_feed() {
    // streams/TestStreamsRuntimeContracts.al:OutStream_WriteText_WithoutArgument_WritesCarriageReturnLineFeed.
    let t = text(
        base!("V: Byte; N: Integer; H: Text;"),
        "B.Data.CreateOutStream(O); O.WriteText('abc'); O.WriteText(); O.WriteText('def'); B.Data.CreateInStream(I);
         while not I.EOS() do begin I.Read(V); N := V; H += Format(N) + ','; end; exit(H);",
    );
    assert_eq!(t, "97,98,99,13,10,100,101,102,");
}

#[test]
fn the_default_encoding_is_code_page_437() {
    // streams/TestStreamsRuntimeContracts.al:WriteText_DefaultEncodingIsWindows1252_NotUtf8 and
    // :WriteText_ExplicitEncodings_Bytes: e acute is the one byte 82 by default (code page 437), two in UTF8, and UTF16 is
    // two bytes per character with no byte order mark.
    let vars = base!("V: Byte; N: Integer; H: Text; E: Char;");
    let bytes = |enc: &str| {
        format!(
            "E := 233; B.Data.CreateOutStream(O{enc}); O.WriteText('h' + Format(E)); B.Data.CreateInStream(I{enc});
             while not I.EOS() do begin I.Read(V); N := V; H += Format(N) + ','; end; exit(H);"
        )
    };
    assert_eq!(text(vars, &bytes("")), "104,130,");
    assert_eq!(text(vars, &bytes(", TextEncoding::UTF8")), "104,195,169,");
    assert_eq!(text(vars, &bytes(", TextEncoding::UTF16")), "104,0,233,0,");
    assert_eq!(text(vars, &bytes(", TextEncoding::Windows")), "104,233,");
    assert_eq!(text(vars, &bytes(", TextEncoding::MSDos")), "104,130,");
}

#[test]
fn read_text_ends_at_a_line_feed_and_drops_carriage_returns() {
    // streams/TestStreamsRuntimeContracts.al:InStream_ReadText_SplitsAtLineFeed_AndDropsALoneCarriageReturn.
    let lines = |content: &str| {
        text(
            base!("Cr: Text[1]; Lf: Text[1]; L: Text;"),
            &format!(
                "Cr[1] := 13; Lf[1] := 10; B.Data.CreateOutStream(O); O.WriteText({content}); B.Data.CreateInStream(I);
                 while not I.EOS() do begin I.ReadText(T); L += '[' + T + ']'; end; exit(L);"
            ),
        )
    };
    assert_eq!(lines("'a' + Lf + 'b'"), "[a][b]");
    assert_eq!(lines("'a' + Cr + Lf + 'b'"), "[a][b]");
    assert_eq!(lines("'a' + Lf + Cr + 'b'"), "[a][b]");
    assert_eq!(lines("'a' + Cr + 'b'"), "[ab]");
    assert_eq!(lines("'a' + Lf + Lf + 'b'"), "[a][][b]");
    assert_eq!(lines("'a' + Lf"), "[a]");
    assert_eq!(lines("'a' + Cr"), "[a]");
}

#[test]
fn create_out_stream_keeps_the_content_and_an_in_stream_is_live() {
    // streams/TestStreamsRuntimeContracts.al:BlobField_CreateOutStream_DoesNotEmptyExistingData and
    // :InStream_CreatedBeforeAWrite_SeesTheWrite.
    let t = text(
        base!("C: Text;"),
        "B.Code := 'S'; B.Insert(); B.Data.CreateOutStream(O); O.WriteText('first-long-text'); B.Modify();
         B.Data.CreateOutStream(O); O.WriteText('x'); B.Modify(); B.CalcFields(Data); B.Data.CreateInStream(I); I.ReadText(C);
         exit(C + '|' + Format(B.Data.Length()));",
    );
    assert_eq!(t, "xirst-long-text|15");
    let t = text(
        base!("C: Text;"),
        "B.Code := 'S'; B.Insert(); B.Data.CreateOutStream(O); O.WriteText('first-long-text');
         B.Data.CreateInStream(I); B.Data.CreateOutStream(O); O.WriteText('later'); I.ReadText(C); exit(C);",
    );
    assert_eq!(t, "later-long-text");
}

#[test]
fn binary_forms_of_the_types_the_probe_measured() {
    // streams/TestStreamsRuntimeContracts.al:OutStream_Write_BinaryFormsOfEachType (Integer, Boolean,
    // BigInteger, Time and Text).
    let hex = |write: &str| {
        text(
            base!("V: Byte; N: Integer; H: Text; Big: BigInteger;"),
            &format!(
                "Evaluate(Big, '5000000000'); B.Data.CreateOutStream(O); {write}; B.Data.CreateInStream(I);
                 while not I.EOS() do begin I.Read(V); N := V; H += Format(N) + ','; end; exit(H);"
            ),
        )
    };
    assert_eq!(hex("O.Write(258)"), "2,1,0,0,");
    assert_eq!(hex("O.Write(true)"), "1,0,0,0,");
    assert_eq!(hex("O.Write(Big)"), "0,242,5,42,1,0,0,0,");
    assert_eq!(hex("O.Write(143005T)"), "201,149,28,3,");
    assert_eq!(hex("O.Write('text')"), "116,101,120,116,0,");
}

// streams/TestStreamsBinaryDateDecimalContracts.al:80-114 (Date and Decimal as bytes, in hex) and :117-123
// (Decimal read back).
#[test]
fn binary_forms_of_a_date_and_a_decimal() {
    let hex = |write: &str| {
        let t = text(
            base!("V: Byte; N: Integer; H: Text;"),
            &format!(
                "B.Data.CreateOutStream(O); {write}; B.Data.CreateInStream(I);
                 while not I.EOS() do begin I.Read(V); N := V; H += Format(N) + ','; end; exit(H);"
            ),
        );
        t.split(',')
            .filter(|b| !b.is_empty())
            .map(|b| format!("{:02X}", b.parse::<u8>().unwrap()))
            .collect::<String>()
    };
    for (w, want) in [
        ("O.Write(0D)", "00000000"),
        ("O.Write(DMY2Date(1, 1, 1753))", "1F8A1300"),
        ("O.Write(DMY2Date(1, 1, 1900))", "932D1500"),
        ("O.Write(DMY2Date(1, 1, 2000))", "EB4A1600"),
        ("O.Write(DMY2Date(5, 3, 2024))", "E78F1600"),
        ("O.Write(DMY2Date(31, 12, 9999))", "91766F00"),
        ("O.Write(ClosingDate(DMY2Date(31, 12, 2024)))", "42921600"),
    ] {
        assert_eq!(hex(w), want, "{w}");
    }
    for (w, want) in [
        ("O.Write(0.0)", "000000000000000000000000"),
        ("O.Write(1.0)", "400000000000000000100000"),
        ("O.Write(-1.0)", "C00000000000000000100000"),
        ("O.Write(1.5)", "400000000000000000150000"),
        ("O.Write(100.0)", "420000000000000000100000"),
        ("O.Write(0.01)", "3E0000000000000000100000"),
        ("O.Write(0.00000001)", "380000000000000000100000"),
        ("O.Write(-0.5)", "BF0000000000000000500000"),
        ("O.Write(123456789.123456789)", "488967452391785634120000"),
        ("O.Write(999999999999999.99)", "4E9099999999999999990000"),
    ] {
        assert_eq!(hex(w), want, "{w}");
    }
    let t = text(
        base!("D: Decimal; H: Text;"),
        "B.Data.CreateOutStream(O); O.Write(123456789.123456789); O.Write(0.01); O.Write(-1.5);
         B.Data.CreateInStream(I); I.Read(D); H += Format(D, 0, 9) + ';'; I.Read(D); H += Format(D, 0, 9) + ';';
         I.Read(D); H += Format(D, 0, 9); exit(H);",
    );
    assert_eq!(t, "123456789.123456789;0.01;-1.5");
}

#[test]
fn array_helpers() {
    // streams/TestArrayStreamContracts.al CompressArray / CopyArray / ArrayLen.
    let t = text(
        "A: array[5] of Text[50]; N: Integer;",
        "A[1] := 'first'; A[3] := 'third'; A[5] := 'fifth'; N := CompressArray(A); exit(Format(N) + A[1] + A[2] + A[3] + '[' + A[4] + A[5] + ']');",
    );
    assert_eq!(t, "3firstthirdfifth[]");
    let t = text(
        "A: array[3] of Text[50]; N: Integer;",
        "A[1] := 'Zebra'; A[2] := 'Apple'; N := CompressArray(A); exit(A[1] + A[2]);",
    );
    assert_eq!(t, "ZebraApple");
    let t = text(
        "S: array[5] of Integer; D: array[3] of Integer; I: Integer;",
        "for I := 1 to 5 do S[I] := I * 10; CopyArray(D, S, 2, 3); exit(Format(D[1]) + Format(D[2]) + Format(D[3]));",
    );
    assert_eq!(t, "203040");
    let t = text("A: array[7] of Integer;", "exit(Format(ArrayLen(A)));");
    assert_eq!(t, "7");
    // streams/TestArrayStreamContracts.al ArrayLen_2D_FirstDimension / _SecondDimension.
    let t = text(
        "A: array[3, 4] of Integer;",
        "exit(Format(ArrayLen(A, 1)) + Format(ArrayLen(A, 2)));",
    );
    assert_eq!(t, "34");
    // streams/TestStreamsRuntimeContracts.al:ArrayLen_MultiDimensional_WithAndWithoutDimension: the total.
    let t = text("A: array[2, 3] of Integer;", "exit(Format(ArrayLen(A)));");
    assert_eq!(t, "6");
    // A dimension outside the array raises when the call runs (misc/lt-system-functions/Arrays.al:ArrayLen_Values).
    assert!(matches!(
        run("A: array[3, 4] of Integer;", "exit(Format(ArrayLen(A, 3)));"),
        Err(Signal::Error(_))
    ));
}

#[test]
fn pad_str_pads_on_the_right() {
    // streams/TestInOutStream.al InOutStream_LongText_PreservesContent: PadStr('', 200, 'X').
    let t = text(
        "S: Text;",
        "S := PadStr('', 5, 'X'); exit(S + Format(StrLen(PadStr('ab', 4))));",
    );
    assert_eq!(t, "XXXXX4");
}

#[test]
fn renaming_a_temporary_record_with_a_blob() {
    // streams/TestBlobRenameIsolation.al TempBlob_CommittedWrite_Rename_SameInstanceReGet_LosesBlobValue: a Blob persisted
    // with Modify is lost across a temporary Rename, while an unpersisted write survives
    // (TempBlob_UncommittedWrite_Rename_SameInstanceReGet_KeepsWrite); a stream over the record's Blob reads nothing right
    // after the Rename either way (streams/TestProbeStreamContracts.al:TempRename_UnpersistedBlobWrite, TempRename_PersistedBlobWrite).
    let vars = "B: Record \"ALT Blob\" temporary; O: OutStream; I: InStream; T: Text;";
    let after = "B.Rename('B'); B.Get('B'); B.CalcFields(Data); B.Data.CreateInStream(I); I.ReadText(T); exit(Format(B.Data.HasValue()) + '|' + T);";
    assert_eq!(
        text(
            vars,
            &format!(
                "B.Init(); B.Code := 'A'; B.Insert(true); B.Data.CreateOutStream(O); O.WriteText('x'); B.Modify(true); {after}"
            )
        ),
        "No|"
    );
    assert_eq!(
        text(
            vars,
            &format!("B.Init(); B.Code := 'A'; B.Insert(true); B.Data.CreateOutStream(O); O.WriteText('x'); {after}")
        ),
        "Yes|x"
    );
    // A database record keeps what Modify persisted across Rename.
    let t = text(
        base!("R: Record \"ALT Blob\"; RI: InStream; RT: Text;"),
        "B.Init(); B.Code := 'RNC1'; B.Insert(true); B.Data.CreateOutStream(O); O.WriteText('RENAME-COMMITTED-BYTES'); B.Modify(true); B.Rename('RNC2');
         R.Get('RNC2'); R.CalcFields(Data); R.Data.CreateInStream(RI); RI.ReadText(RT); exit(RT);",
    );
    assert_eq!(t, "RENAME-COMMITTED-BYTES");
    // Rename persists the whole buffer, so an unpersisted Blob write reaches the renamed row.
    let t = text(
        base!("R: Record \"ALT Blob\"; RI: InStream; RT: Text;"),
        "B.Init(); B.Code := 'RNL1'; B.Insert(true); B.Data.CreateOutStream(O); O.WriteText('RENAME-LEAKED-BYTES'); B.Rename('RNL2');
         R.Get('RNL2'); R.CalcFields(Data); R.Data.CreateInStream(RI); RI.ReadText(RT); exit(RT);",
    );
    assert_eq!(t, "RENAME-LEAKED-BYTES");
}

#[test]
fn text_encodings_decide_the_bytes_per_character() {
    // text/TestTextEncodingWindows.al: TextEncoding::Windows writes one byte per representable character
    // (cp1252: em dash, euro, e acute, no-break space) and round-trips them; UTF8 writes more bytes for the
    // same text and round-trips; ASCII costs one byte either way.
    let sample = "A' + Format(E1) + Format(E2) + Format(E3) + Format(E4) + 'Z";
    let vars = base!("E1: Char; E2: Char; E3: Char; E4: Char; WI: InStream; Len: Integer; Back: Text;");
    let prologue = "E1 := 8212; E2 := 8364; E3 := 233; E4 := 160;";
    let write = |enc: &str| {
        format!(
            "{prologue} B.Init(); B.Code := 'ROW'; B.Insert(); B.Data.CreateOutStream(O, TextEncoding::{enc}); O.WriteText('{sample}'); B.Modify();
             B.Get('ROW'); B.CalcFields(Data); Len := B.Data.Length(); B.Data.CreateInStream(WI, TextEncoding::{enc}); WI.ReadText(Back);
             exit(Format(Len) + '|' + Format(Back = '{sample}'));"
        )
    };
    assert_eq!(text(vars, &write("Windows")), "6|Yes");
    assert_eq!(text(vars, &write("UTF8")), "12|Yes");
    // UTF16 is two bytes per character (streams/TestStreamsRuntimeContracts.al:WriteText_ExplicitEncodings_Bytes).
    assert_eq!(text(vars, &write("UTF16")), "12|Yes");
}

#[test]
fn json_write_to_an_out_stream_writes_utf8_whatever_the_stream_encoding() {
    // streams/ProbeJsonAndInStreamBytes.al:WriteTo_OutStream_Windows_AsciiAndNonAscii, :..._Utf16_...,
    // :..._MsDos_...: the bytes are UTF-8 on every stream (BC 27.5).
    let read =
        "B.Data.CreateInStream(I); while not I.EOS() do begin I.Read(V); N := V; T += Format(N) + ','; end; exit(T);";
    for enc in [
        "",
        ", TextEncoding::UTF16",
        ", TextEncoding::MSDos",
        ", TextEncoding::Windows",
    ] {
        let t = text(
            base!("A: JsonArray; V: Byte; N: Integer;"),
            &format!("A.Add('\u{e9}'); B.Data.CreateOutStream(O{enc}); A.WriteTo(O); {read}"),
        );
        assert_eq!(t, "91,34,195,169,34,93,", "{enc}");
    }
}

#[test]
fn json_write_to_an_out_stream_writes_non_ascii_text_as_utf8_on_a_utf8_stream() {
    let t = text(
        base!("A: JsonArray; V: Byte; N: Integer;"),
        "A.Add('\u{e9}'); B.Data.CreateOutStream(O, TextEncoding::UTF8); A.WriteTo(O); B.Data.CreateInStream(I);
         while not I.EOS() do begin I.Read(V); N := V; T += Format(N) + ','; end; exit(T);",
    );
    assert_eq!(t, "91,34,195,169,34,93,");
}

#[test]
fn json_write_to_an_out_stream_writes_the_compact_text_the_text_overload_gives() {
    // The text is pinned by json/TestJsonArray.al:90 and json/TestJsonRuntimeContracts.al
    // (JsonObject_WriteTo_IsCompactAndKeepsInsertionOrder); a stream receives the same characters.
    // The stream encoding is not pinned for JSON (docs/probes-needed.md), so only text that is
    // identical in UTF-8 and the single-byte code pages is written to a Windows stream.
    let vars = base!("A: JsonArray; J: JsonObject;");
    let body = |enc: &str| {
        format!(
            "J.Add('k', 'v'); A.Add(1); A.Add(J); B.Code := 'J'; B.Data.CreateOutStream(O{enc}); A.WriteTo(O); B.Insert();
             B.Get('J'); B.CalcFields(Data); B.Data.CreateInStream(I{enc}); I.ReadText(T); exit(T);"
        )
    };
    assert_eq!(text(vars, &body("")), "[1,{\"k\":\"v\"}]");
    assert_eq!(text(vars, &body(", TextEncoding::UTF8")), "[1,{\"k\":\"v\"}]");
    assert_eq!(
        text(
            base!("J: JsonObject;"),
            "J.Add('a', 1); B.Data.CreateOutStream(O, TextEncoding::UTF8); J.WriteTo(O); B.Data.CreateInStream(I, TextEncoding::UTF8); I.ReadText(T); exit(T);"
        ),
        "{\"a\":1}"
    );
}

#[test]
fn json_write_to_an_out_stream_does_not_move_the_stream_and_is_refused_after_other_writes() {
    // misc/lt-probes-needed-3/PnC20JsonStreams.al:WriteTo_DefaultStream_And_Appending: a second WriteTo writes over the
    // first one's bytes, the blob is not emptied (`[1,2]1}`). After other writes the place is not pinned.
    let t = text(
        base!("A: JsonArray; J: JsonObject;"),
        "A.Add(1); A.Add(2); J.Add('a', 1); B.Data.CreateOutStream(O); J.WriteTo(O); A.WriteTo(O);
         B.Data.CreateInStream(I); I.ReadText(T); exit(T);",
    );
    assert_eq!(t, "[1,2]1}");
    assert!(refused(
        base!("A: JsonArray;"),
        "A.Add(1); B.Data.CreateOutStream(O); O.WriteText('x'); A.WriteTo(O); exit('');",
    ));
}

#[test]
fn json_read_from_an_in_stream_parses_what_the_stream_holds() {
    // The same parse as ReadFrom(Text) (json/TestJsonObject.al, TestJsonArray.al); the stream's
    // rest is read and the position does not move (misc/lt-probes-needed-3/PnC20JsonStreams.al:ReadFrom_RestOfStream). The bytes are taken as UTF-8 only where that
    // equals the stream's own reading (docs/probes-needed.md).
    let vars = base!("A: JsonArray; J: JsonObject; Ok: Boolean;");
    assert_eq!(
        text(
            vars,
            "B.Data.CreateOutStream(O, TextEncoding::UTF8); O.WriteText('[1,{\"k\":\"v\"}]'); B.Data.CreateInStream(I, TextEncoding::UTF8);
             Ok := A.ReadFrom(I); exit(Format(Ok) + Format(A.Count()) + Format(I.EOS()));"
        ),
        "Yes2No"
    );
    assert_eq!(
        text(
            vars,
            "B.Data.CreateOutStream(O); O.WriteText('{\"k\":5}'); B.Data.CreateInStream(I);
             Ok := J.ReadFrom(I); exit(Format(Ok) + Format(J.Contains('k')));"
        ),
        "YesYes"
    );
    // The default (Windows) stream with a non-ASCII byte that is not UTF-8, and a UTF-16 stream (its bytes hold
    // NUL characters), are refused: what BC reads there is not pinned.
    assert!(refused(
        vars,
        "B.Data.CreateOutStream(O); O.WriteText('[\"\u{e9}\"]'); B.Data.CreateInStream(I); Ok := A.ReadFrom(I); exit('');"
    ));
    assert!(refused(
        vars,
        "B.Data.CreateOutStream(O, TextEncoding::UTF16); O.WriteText('[1]'); B.Data.CreateInStream(I, TextEncoding::UTF16); Ok := A.ReadFrom(I); exit('');"
    ));
    // UTF-8 bytes on a default stream parse, whatever the stream's encoding
    // (streams/ProbeJsonAndInStreamBytes.al:ReadFrom_InStream_WithBomAndWithout).
    assert_eq!(
        text(
            vars,
            "B.Data.CreateOutStream(O, TextEncoding::UTF8); O.WriteText('[\"\u{e9}\"]'); B.Data.CreateInStream(I); Ok := A.ReadFrom(I); exit(Format(Ok) + Format(A.Count()));"
        ),
        "Yes1"
    );
}

#[test]
fn json_write_to_then_read_from_a_stream_round_trips() {
    // The install path of System Application's "Reten. Pol. Allowed Tbl. Impl." and "Upgrade Tag
    // Impl.": WriteTo(OutStream) into a Blob, then ReadFrom(InStream) of it.
    let t = text(
        base!("A: JsonArray; A2: JsonArray;"),
        "A.Add(1); A.Add('x'); B.Code := 'R'; B.Data.CreateOutStream(O, TextEncoding::UTF8); A.WriteTo(O); B.Insert();
         B.Get('R'); B.CalcFields(Data); B.Data.CreateInStream(I, TextEncoding::UTF8);
         if not A2.ReadFrom(I) then exit('fail'); A2.WriteTo(T); exit(T);",
    );
    assert_eq!(t, "[1,\"x\"]");
}

#[test]
fn read_into_a_text_reads_back_what_write_of_a_text_wrote() {
    // streams/TestStreamsRuntimeContracts.al:OutStream_Write_BinaryFormsOfEachType pins Write(Text)
    // as the characters and a zero byte. Read(Text) takes the characters up to that zero byte; a
    // stream without one is refused (what BC reads then is not pinned).
    let t = text(
        base!("A: Text; C: Text; E: Text;"),
        "B.Data.CreateOutStream(O, TextEncoding::UTF8); O.Write('abc'); O.Write('de'); B.Data.CreateInStream(I, TextEncoding::UTF8);
         I.Read(A); I.Read(C); exit(A + '|' + C + '|' + Format(I.EOS()));",
    );
    assert_eq!(t, "abc|de|Yes");
    // A stream without a zero byte gives the rest, with no error
    // (streams/ProbeJsonAndInStreamBytes.al:InStreamRead_Text_OnWriteTextContent_NoTerminator).
    assert_eq!(
        text(
            base!("A: Text;"),
            "B.Data.CreateOutStream(O); O.WriteText('abc'); B.Data.CreateInStream(I); I.Read(A); exit(A + Format(I.EOS()));"
        ),
        "abcYes"
    );
}

// misc/lt-probes-needed-5/PnE04Streams.al:118 ReadText_Utf8_NonAscii (`7|wörld` with a zero byte after it) and, on corpus branch
// al-runtime-rs-staging-pub-to, TO_ReadText_NonAscii_NoTerminator (`7|wörld` without one, assumption
// utf-eight-read-non-ascii-unterminated). A UTF-16 stream with non-ASCII characters stays refused.
#[test]
fn read_into_a_text_takes_non_ascii_characters_on_a_utf8_stream() {
    let vars = base!("N: Integer;");
    assert_eq!(
        text(
            vars,
            "B.Data.CreateOutStream(O, TextEncoding::UTF8); O.WriteText('w\u{f6}rld'); B.Data.CreateInStream(I, TextEncoding::UTF8);
             N := I.Read(T); exit(Format(N) + '|' + T);"
        ),
        "7|w\u{f6}rld"
    );
    assert!(refused(
        vars,
        "B.Data.CreateOutStream(O, TextEncoding::UTF16); O.WriteText('w\u{f6}r'); B.Data.CreateInStream(I, TextEncoding::UTF16);
         N := I.Read(T); exit('');"
    ));
}

// streams/lt-dd-stream-leads/DdStreamLeads.al:502 (Item68_Utf8Bom `read utf8`): Read(Text) of a UTF-8 stream skips the mark.
// File Management.GetFileContents reads a report's SaveAsXml file (EF BB BF first) this way for Library - Report Dataset.
#[test]
fn read_into_a_text_of_a_utf8_stream_skips_a_byte_order_mark() {
    let vars = base!("C: Char;");
    let body = "C := 65279; B.Data.CreateOutStream(O, TextEncoding::UTF8); O.WriteText(Format(C) + '<a/>');
        B.Data.CreateInStream(I, TextEncoding::UTF8); I.Read(T); exit(T);";
    assert_eq!(text(vars, body), "<a/>");
    // The count it returns over a mark is not pinned.
    let used = "C := 65279; B.Data.CreateOutStream(O, TextEncoding::UTF8); O.WriteText(Format(C) + '<a/>');
        B.Data.CreateInStream(I, TextEncoding::UTF8); exit(Format(I.Read(T)));";
    assert!(refused(vars, used));
}
