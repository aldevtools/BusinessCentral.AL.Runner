//! A stream write to a Blob of a record read with `Get` and no `CalcFields`, over stored content (AL Runner issue 5681).
//! The first write stores exactly the new bytes; after that, and after `CalcFields`, a stream writes in place
//! (record/lt-blob-calcfields-length/BlobCalcFieldsLength.al:222-350). What a temporary record or a `CalcFields` between
//! `CreateOutStream` and the write does is not pinned: a look at the Blob is refused there.

use al_testkit::{int, run_in, text};

fn assert_refused(r: Result<al_builtins::value::Value, Signal>) {
    match r {
        Err(Signal::Unsupported(reason)) => assert!(reason.contains("without CalcFields"), "{reason}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
use al_builtins::native::Signal;

const SRC: &str = r#"
table 50120 "Blob Unloaded T"
{
    fields { field(1; K; Integer) { } field(2; B; Blob) { } }
    keys { key(PK; K) { } }
}
codeunit 50121 C
{
    local procedure Store()
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        T.K := 1;
        T.Insert();
        T.B.CreateOutStream(OutS);
        OutS.WriteText('AAAAAAAAAA');
        T.Modify();
    end;

    local procedure ReadBack(): Text
    var
        T: Record "Blob Unloaded T";
        InS: InStream;
        S: Text;
    begin
        T.Get(1);
        T.CalcFields(B);
        T.B.CreateInStream(InS);
        InS.ReadText(S);
        exit(S);
    end;

    procedure WriteAfterGet(): Text
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        Store();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        OutS.WriteText('BB');
        T.Modify();
        exit(ReadBack());
    end;

    procedure WriteLongerAfterGet(): Text
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        Store();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        OutS.WriteText('BBBBBBBBBBBB');
        T.Modify();
        exit(ReadBack());
    end;

    procedure ShortWritesThenCovering(): Text
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        Store();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        OutS.WriteText('BB');
        T.B.CreateOutStream(OutS);
        OutS.WriteText('C');
        OutS.WriteText('CCCCCCCCC');
        T.Modify();
        exit(ReadBack());
    end;

    procedure ShortWritesTwice(): Text
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        Store();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        OutS.WriteText('BB');
        T.B.CreateOutStream(OutS);
        OutS.WriteText('C');
        T.Modify();
        exit(ReadBack());
    end;

    procedure LengthAfterShortWrite(): Integer
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        Store();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        OutS.WriteText('BB');
        exit(T.B.Length());
    end;

    procedure StreamNotWritten(): Text
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        T.K := 1;
        T.Insert();
        T.B.CreateOutStream(OutS);
        OutS.WriteText('stored');
        T.Modify();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        exit(Format(T.B.Length()) + '|' + Format(T.B.HasValue()));
    end;

    procedure StreamNotWrittenModify(): Text
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        Store();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        T.Modify();
        exit(ReadBack());
    end;

    procedure OneStreamTwoTexts(): Text
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        Store();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        OutS.WriteText('one');
        OutS.WriteText('more');
        T.Modify();
        exit(ReadBack());
    end;

    procedure WriteModifyTwice(): Text
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        Store();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        OutS.WriteText('third');
        T.Modify();
        T.B.CreateOutStream(OutS);
        OutS.WriteText('x');
        T.Modify();
        exit(ReadBack());
    end;

    procedure CalcFieldsBetweenStreamAndWrite(): Text
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        Store();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        T.CalcFields(B);
        OutS.WriteText('BB');
        T.Modify();
        exit(ReadBack());
    end;

    procedure TemporaryShorterWrite(): Integer
    var
        T: Record "Blob Unloaded T" temporary;
        OutS: OutStream;
    begin
        T.K := 1;
        T.Insert();
        T.B.CreateOutStream(OutS);
        OutS.WriteText('AAAAAAAAAA');
        T.Modify();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        OutS.WriteText('BB');
        exit(T.B.Length());
    end;

    procedure RecordRefCalcFieldLeavesRow(): Text
    var
        T: Record "Blob Unloaded T";
        RecRef: RecordRef;
        FldRef: FieldRef;
        OutS: OutStream;
    begin
        T.K := 1;
        T.Insert();
        T.B.CreateOutStream(OutS);
        OutS.WriteText('stored');
        T.Modify();
        T.Get(1);
        RecRef.GetTable(T);
        FldRef := RecRef.Field(2);
        FldRef.CalcField();
        exit(Format(T.B.Length()));
    end;

    procedure AssignFromUnloaded(): Text
    var
        S: Record "Blob Unloaded T";
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        Store();
        T.K := 2;
        T.Insert();
        S.Get(1);
        T.Get(2);
        T.B := S.B;
        exit(Format(T.B.HasValue()) + '|' + Format(T.B.Length()));
    end;

    procedure WriteAfterCalcFields(): Text
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        Store();
        T.Get(1);
        T.CalcFields(B);
        T.B.CreateOutStream(OutS);
        OutS.WriteText('BB');
        T.Modify();
        exit(ReadBack());
    end;

    procedure WriteAfterGetOfEmptyBlob(): Text
    var
        T: Record "Blob Unloaded T";
        OutS: OutStream;
    begin
        T.K := 1;
        T.Insert();
        T.Get(1);
        T.B.CreateOutStream(OutS);
        OutS.WriteText('BB');
        T.Modify();
        exit(ReadBack());
    end;
}
"#;

#[test]
fn first_write_after_get_without_calcfields_stores_only_the_new_bytes() {
    // BlobCalcFieldsLength.al:222 RF_GetWithoutCalcFields_ShorterWrite_Modify: 'new' over 'a long stored value' is 'new'.
    assert_eq!(text(run_in(SRC, "C", "WriteAfterGet")), "BB");
    assert_eq!(int(run_in(SRC, "C", "LengthAfterShortWrite")), 2);
    // :250 RF_GetWithoutCalcFields_LongerWrite_Modify.
    assert_eq!(text(run_in(SRC, "C", "WriteLongerAfterGet")), "BBBBBBBBBBBB");
}

#[test]
fn later_streams_write_in_place_and_one_stream_appends() {
    // :313 RF_GetWithoutCalcFields_TwoWrites: 'first write' then 'two' is 'twost write'; 'one' + 'more' is 'onemore';
    // 'third value', Modify, 'x', Modify is 'xhird value'.
    assert_eq!(text(run_in(SRC, "C", "ShortWritesTwice")), "CB");
    assert_eq!(text(run_in(SRC, "C", "ShortWritesThenCovering")), "CCCCCCCCCC");
    assert_eq!(text(run_in(SRC, "C", "OneStreamTwoTexts")), "onemore");
    assert_eq!(text(run_in(SRC, "C", "WriteModifyTwice")), "xhird");
}

#[test]
fn an_unwritten_stream_leaves_the_stored_value() {
    // :350 RF_GetWithoutCalcFields_OutStreamNotWritten_Modify: 'after create' has=Yes|len=12 for 'stored', read back 'stored'.
    assert_eq!(text(run_in(SRC, "C", "StreamNotWritten")), "12|Yes");
    assert_eq!(text(run_in(SRC, "C", "StreamNotWrittenModify")), "AAAAAAAAAA");
}

#[test]
fn a_recordref_copy_loads_its_own_blob() {
    // :369 RF_GetWithoutCalcFields_TempBlobToRecord_Modify: 'row after from record' is still the stored size.
    assert_eq!(text(run_in(SRC, "C", "RecordRefCalcFieldLeavesRow")), "12");
}

#[test]
fn assigning_an_unloaded_blob_gives_an_empty_one() {
    // :404 RF_AssignFromUnloadedBlob_Modify: 'target after assign' has=No|len=0.
    assert_eq!(text(run_in(SRC, "C", "AssignFromUnloaded")), "No|0");
}

#[test]
fn unpinned_shapes_are_refused() {
    assert_refused(run_in(SRC, "C", "CalcFieldsBetweenStreamAndWrite"));
    assert_refused(run_in(SRC, "C", "TemporaryShorterWrite"));
}

#[test]
fn write_after_calcfields_overwrites_the_leading_bytes() {
    // streams/TestStreamsRuntimeContracts.al:197 BlobField_CreateOutStream_DoesNotEmptyExistingData.
    assert_eq!(text(run_in(SRC, "C", "WriteAfterCalcFields")), "BBAAAAAAAA");
}

#[test]
fn write_after_get_of_an_empty_blob_runs() {
    assert_eq!(text(run_in(SRC, "C", "WriteAfterGetOfEmptyBlob")), "BB");
}
