//! A stream write to a Blob of a record read with `Get` and no `CalcFields`, over stored content (AL Runner issue 5681).
//! No corpus test pins what BC keeps then when the new bytes are shorter: only the new bytes, or the new bytes over the
//! stored ones. A look at the Blob is refused until a write reaches the stored content's end.

use al_testkit::{run_in, text};

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
fn shorter_write_after_get_without_calcfields_is_refused() {
    assert_refused(run_in(SRC, "C", "WriteAfterGet"));
    assert_refused(run_in(SRC, "C", "ShortWritesTwice"));
    assert_refused(run_in(SRC, "C", "LengthAfterShortWrite"));
}

#[test]
fn write_after_get_that_reaches_the_stored_end_runs() {
    // record/lt-blob-calcfields-length/BlobCalcFieldsLength.al:BlobCalcFields_AfterModifyAndClear_Result.
    assert_eq!(text(run_in(SRC, "C", "WriteLongerAfterGet")), "BBBBBBBBBBBB");
    assert_eq!(text(run_in(SRC, "C", "ShortWritesThenCovering")), "CCCCCCCCCC");
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
