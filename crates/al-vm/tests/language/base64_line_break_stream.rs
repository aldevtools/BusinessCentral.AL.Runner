//! `Base64Convert.ToBase64(InStream, true[, OutStream])` runs only on the streams and targets
//! tests/al-language/streams/Base64LineBreaks.al measured (al-builtins `stream_line_breaks`): the one InStream on the Blob
//! of "Temp Blob Impl." after one `WriteText`, used only by `ReadText(Text, 10)` (:53-61, :186-189), and a target that is
//! empty, holds one `WriteText`, or has an unused InStream (:284-286, :364-367, :385-389). Each neighbor refuses.
//!
//! The app is named "System Application" and holds a "Temp Blob Impl." shaped as System Application's (its temporary record's
//! Blob field takes the streams) and an impl that asks the stream what "Base64 Convert Impl." asks (Length, EOS, Length,
//! Base64ConvertImpl.Codeunit.al:75-78) before it calls the native.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{AppSource, Built, app_named, compile_apps};

const SYSTEM_APP: &str = r#"
codeunit 50101 Base64Convert
{
    [Native] procedure ToBase64(InStream: InStream; InsertLineBreaks: Boolean): Text begin end;
    [Native] procedure ToBase64(InStream: InStream; InsertLineBreaks: Boolean; OutStream: OutStream) begin end;
}
table 50102 "Temp Blob" { fields { field(1; "Primary Key"; Integer) { } field(2; Blob; Blob) { } } keys { key(PK; "Primary Key") { } } }
table 50103 "Plain Blob" { fields { field(1; Id; Integer) { } field(2; B; Blob) { } } keys { key(PK; Id) { } } }
codeunit 50104 "Temp Blob Impl."
{
    var TempBlob: Record "Temp Blob" temporary;
    procedure CreateInStream(var InStream: InStream) begin TempBlob.Blob.CreateInStream(InStream); end;
    procedure CreateOutStream(var OutStream: OutStream) begin TempBlob.Blob.CreateOutStream(OutStream); end;
}
codeunit 50105 "B64 Impl"
{
    var Native: Codeunit Base64Convert;
    procedure ToBase64(InStream: InStream): Text
    var L: Integer;
    begin
        if (InStream.Length < 1) or InStream.EOS then
            exit('');
        L := InStream.Length;
        exit(Native.ToBase64(InStream, true));
    end;
    procedure ToBase64Out(InStream: InStream; OutStream: OutStream)
    var L: Integer;
    begin
        if (InStream.Length < 1) or InStream.EOS then
            exit;
        L := InStream.Length;
        Native.ToBase64(InStream, true, OutStream);
    end;
}
codeunit 50100 C
{
    var Impl: Codeunit "B64 Impl";
    local procedure Digits(): Text
    var R: Text;
    begin
        while StrLen(R) < 100 do R += '0123456789';
        exit(R);
    end;
    local procedure Fill(var Blob: Codeunit "Temp Blob Impl."; var InS: InStream)
    var OutS: OutStream;
    begin
        Blob.CreateOutStream(OutS);
        OutS.WriteText(Digits());
        Blob.CreateInStream(InS);
    end;
    procedure Pinned(): Text
    var Blob: Codeunit "Temp Blob Impl."; InS: InStream; T: Text;
    begin
        Fill(Blob, InS);
        InS.ReadText(T, 10);
        exit(Format(StrLen(Impl.ToBase64(InS))));
    end;
    procedure EosBefore(): Text
    var Blob: Codeunit "Temp Blob Impl."; InS: InStream;
    begin
        Fill(Blob, InS);
        if InS.EOS then exit('eos');
        exit(Impl.ToBase64(InS));
    end;
    procedure BinaryRead(): Text
    var Blob: Codeunit "Temp Blob Impl."; InS: InStream; C: Char;
    begin
        Fill(Blob, InS);
        InS.Read(C);
        exit(Impl.ToBase64(InS));
    end;
    procedure AssignedStream(): Text
    var Blob: Codeunit "Temp Blob Impl."; InS: InStream; InS2: InStream;
    begin
        Fill(Blob, InS);
        InS2 := InS;
        exit(Impl.ToBase64(InS));
    end;
    procedure TwoWrites(): Text
    var Blob: Codeunit "Temp Blob Impl."; OutS: OutStream; InS: InStream;
    begin
        Blob.CreateOutStream(OutS);
        OutS.WriteText(CopyStr(Digits(), 1, 50));
        OutS.WriteText(CopyStr(Digits(), 51, 50));
        Blob.CreateInStream(InS);
        exit(Impl.ToBase64(InS));
    end;
    procedure SecondInStream(): Text
    var Blob: Codeunit "Temp Blob Impl."; InS: InStream; InS2: InStream;
    begin
        Fill(Blob, InS);
        Blob.CreateInStream(InS2);
        exit(Impl.ToBase64(InS));
    end;
    procedure TempRecordOutsideTheImpl(): Text
    var Row: Record "Plain Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        Row.B.CreateOutStream(OutS);
        OutS.WriteText(Digits());
        Row.B.CreateInStream(InS);
        exit(Impl.ToBase64(InS));
    end;
    local procedure Target(var OutS: OutStream; Shape: Integer): Integer
    var Source: Codeunit "Temp Blob Impl."; Dest: Codeunit "Temp Blob Impl."; InS: InStream; TIn: InStream; T: Text;
    begin
        Fill(Source, InS);
        Dest.CreateOutStream(OutS);
        case Shape of
            1: OutS.WriteText('AB');
            2: Dest.CreateInStream(TIn);
            3: begin OutS.WriteText('AB'); OutS.WriteText('CD'); end;
            4: begin OutS.WriteText('AB'); Dest.CreateInStream(TIn); TIn.ReadText(T); end;
        end;
        Impl.ToBase64Out(InS, OutS);
        exit(1);
    end;
    procedure TargetShape(Shape: Integer): Text
    var OutS: OutStream;
    begin
        exit(Format(Target(OutS, Shape)));
    end;
}
"#;

fn session() -> Built {
    Built::new(compile_apps(vec![system_app("System Application")]))
}

/// The app under `name`, with a GUID id: the VM names the app of the calling object only for an app whose id is a GUID.
fn system_app(name: &str) -> AppSource {
    AppSource {
        id: "63ca2fa4-4f03-4f2b-a480-172fef340d3f".into(),
        ..app_named(name, SYSTEM_APP)
    }
}

fn refused(r: Result<Value, Signal>, what: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains("Base64Convert.ToBase64"), "{what}: {why}"),
        other => panic!("{what}: expected a refusal, got {other:?}"),
    }
}

fn target(shape: i32) -> Result<Value, Signal> {
    let mut b = session();
    let (oi, ri) = b.routine_in("C", "TargetShape");
    b.call(oi, ri, vec![Value::Integer(shape)]).map(|(v, _)| v)
}

#[test]
fn the_pinned_stream_converts_the_whole_stream() {
    // Base64LineBreaks.al:186-193 (BC28PLUS): 10 bytes read, 138 characters.
    let r = session().run_in("C", "Pinned");
    assert!(matches!(&r, Ok(Value::Text(t)) if &**t == "138"), "{r:?}");
}

#[test]
fn a_stream_used_by_anything_but_read_text_with_a_length_refuses() {
    refused(session().run_in("C", "EosBefore"), "EOS before the call");
    refused(session().run_in("C", "BinaryRead"), "a binary Read");
}

#[test]
fn a_stream_given_to_another_variable_refuses() {
    refused(session().run_in("C", "AssignedStream"), "InS2 := InS");
}

#[test]
fn a_blob_written_twice_or_read_by_a_second_stream_refuses() {
    refused(session().run_in("C", "TwoWrites"), "two WriteText calls");
    refused(session().run_in("C", "SecondInStream"), "a second InStream on the Blob");
}

#[test]
fn a_blob_no_temp_blob_impl_of_the_system_application_holds_refuses() {
    refused(
        session().run_in("C", "TempRecordOutsideTheImpl"),
        "a temporary record of the test",
    );
    let mut other_app = Built::new(compile_apps(vec![system_app("Other App")]));
    refused(other_app.run_in("C", "Pinned"), "\"Temp Blob Impl.\" of another app");
}

#[test]
fn only_the_measured_targets_take_the_output() {
    // Empty (:284-286), holding `AB` (:385-389), an unused InStream on the empty target (:364-367).
    for shape in [0, 1, 2] {
        let r = target(shape);
        assert!(matches!(&r, Ok(Value::Text(t)) if &**t == "1"), "shape {shape}: {r:?}");
    }
    refused(target(3), "a target written twice");
    refused(target(4), "a target an InStream read before");
}
