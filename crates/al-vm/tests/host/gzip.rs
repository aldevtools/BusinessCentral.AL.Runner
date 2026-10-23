//! `GZipStream` and `CompressionMode`, the DotNet types behind System Application's "Data Compression"
//! (`GZipCompress`, `GZipDecompress`), served by the runtime. Every literal is what the real service tier
//! answered in the corpus staging branch: tests/al-language/types/PZGzip.al (BC 29.0, staging 2c09ecd).
//! The AL below is my own minimal version of the System Application's two procedures.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = r#"
dotnet { assembly("netstandard") {
    type("System.IO.Compression.GZipStream"; GZipStream) { }
    type("System.IO.Compression.CompressionMode"; CompressionMode) { }
} }
table 50150 "GZ Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 50101 "GZ Impl"
{
    var GZipStream: DotNet GZipStream;
    procedure Compress(InputInStream: InStream; CompressedOutStream: OutStream)
    var DotNetCompressionMode: DotNet CompressionMode;
    begin
        GZipStream := GZipStream.GZipStream(CompressedOutStream, DotNetCompressionMode::Compress);
        CopyStream(GZipStream, InputInStream);
        GZipStream.Dispose();
    end;
    procedure CompressClose(InputInStream: InStream; CompressedOutStream: OutStream)
    var DotNetCompressionMode: DotNet CompressionMode;
    begin
        GZipStream := GZipStream.GZipStream(CompressedOutStream, DotNetCompressionMode::Compress);
        CopyStream(GZipStream, InputInStream);
        GZipStream.Close();
    end;
    procedure Decompress(InputInStream: InStream; DecompressedOutStream: OutStream)
    var DotNetCompressionMode: DotNet CompressionMode;
    begin
        GZipStream := GZipStream.GZipStream(InputInStream, DotNetCompressionMode::Decompress);
        GZipStream.CopyTo(DecompressedOutStream);
        GZipStream.Dispose();
    end;
}
codeunit 50100 C
{
    procedure Fill(var B: Record "GZ Blob"; Value: Text)
    var O: OutStream;
    begin
        B.Data.CreateOutStream(O);
        O.WriteText(Value);
    end;
    procedure Pack(var Plain: Record "GZ Blob"; var Packed: Record "GZ Blob")
    var I: InStream; O: OutStream; Impl: Codeunit "GZ Impl";
    begin
        Plain.CalcFields(Data); Plain.Data.CreateInStream(I);
        Clear(Packed.Data); Packed.Data.CreateOutStream(O);
        Impl.Compress(I, O);
    end;
    procedure PackClose(var Plain: Record "GZ Blob"; var Packed: Record "GZ Blob")
    var I: InStream; O: OutStream; Impl: Codeunit "GZ Impl";
    begin
        Plain.CalcFields(Data); Plain.Data.CreateInStream(I);
        Clear(Packed.Data); Packed.Data.CreateOutStream(O);
        Impl.CompressClose(I, O);
    end;
    procedure Unpack(var Packed: Record "GZ Blob"; var Plain: Record "GZ Blob")
    var I: InStream; O: OutStream; Impl: Codeunit "GZ Impl";
    begin
        Packed.CalcFields(Data); Packed.Data.CreateInStream(I);
        Clear(Plain.Data); Plain.Data.CreateOutStream(O);
        Impl.Decompress(I, O);
    end;
    procedure Len(var B: Record "GZ Blob"): Integer
    var I: InStream;
    begin
        B.CalcFields(Data);
        if not B.Data.HasValue() then exit(0);
        B.Data.CreateInStream(I);
        exit(I.Length());
    end;
    procedure ReadAll(var B: Record "GZ Blob"): Text
    var I: InStream; T: Text;
    begin
        if Len(B) = 0 then exit('');
        B.Data.CreateInStream(I);
        I.ReadText(T, Len(B));
        exit(T);
    end;
    BODY
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = SRC.replace("BODY", body);
    al_testkit::run_in(&src, "C", "F")
}

/// PZGzip.al: GZip_TextRoundTrip, GZip_CompressedStartsWithTheGzipMagicNumber.
#[test]
fn text_round_trips_and_the_packed_bytes_start_with_31_and_139() {
    let r = text(run(
        "procedure F(): Text var P: Record \"GZ Blob\"; Z: Record \"GZ Blob\"; B: Record \"GZ Blob\"; S: Text; i: Integer; St: InStream; b1: Byte; b2: Byte; n1: Integer; n2: Integer; begin
            for i := 1 to 200 do S += 'Hello gzip, aeoeue ';
            Fill(P, S); Pack(P, Z); Unpack(Z, B);
            Z.Data.CreateInStream(St); St.Read(b1); St.Read(b2); n1 := b1; n2 := b2;
            exit(Format(Len(Z) < Len(P)) + '/' + Format(Len(B) = Len(P)) + '/' + Format(ReadAll(B) = S) + '/' + Format(n1) + '/' + Format(n2));
        end;",
    ));
    assert_eq!(text_of(&r), "Yes/Yes/Yes/31/139");
}

/// Windows BC 29.0 probe cu 61289 SC_GZipStream_Close_ConfigPackageShape (Config. Pckg. Compression Mgt.ProcessGZip,
/// ConfigPckgCompressionMgt.Codeunit.al:80): Close of a compressing stream writes the packed bytes as Dispose does.
#[test]
fn close_of_a_compressing_stream_packs_as_dispose_does_and_marks_its_assumption() {
    use al_builtins::assumptions::{Assumption, take_hits};
    take_hits();
    let r = text(run(
        "procedure F(): Text var P: Record \"GZ Blob\"; Z: Record \"GZ Blob\"; B: Record \"GZ Blob\"; S: Text; begin
            S := 'Hello gzip close'; Fill(P, S); PackClose(P, Z); Unpack(Z, B);
            exit(Format(Len(Z) > 0) + '/' + Format(ReadAll(B) = S));
        end;",
    ));
    assert_eq!(text_of(&r), "Yes/Yes");
    assert!(take_hits().contains(&Assumption::GZipStreamCloseIsDispose));
}

fn text_of(s: &str) -> &str {
    s
}

/// PZGzip.al: GZip_BinaryRoundTrip_AllByteValues.
#[test]
fn all_256_byte_values_round_trip() {
    let r = text(run(
        "procedure F(): Text var P: Record \"GZ Blob\"; Z: Record \"GZ Blob\"; B: Record \"GZ Blob\"; Os: OutStream; St: InStream; v: Byte; i: Integer; Same: Boolean; begin
            P.Data.CreateOutStream(Os);
            for i := 0 to 255 do begin v := i; Os.Write(v); end;
            Pack(P, Z); Unpack(Z, B);
            B.Data.CreateInStream(St); Same := true;
            for i := 0 to 255 do begin St.Read(v); if v <> i then Same := false; end;
            exit(Format(Len(P)) + '/' + Format(Len(B)) + '/' + Format(Same));
        end;",
    ));
    assert_eq!(r, "256/256/Yes");
}

/// PZGzip.al: GZip_EmptyInput. Empty input compresses to no bytes; decompressing an empty stream gives an
/// empty result and no error.
#[test]
fn empty_input_packs_to_nothing_and_unpacks_to_nothing() {
    let r = text(run(
        "procedure F(): Text var P: Record \"GZ Blob\"; Z: Record \"GZ Blob\"; B: Record \"GZ Blob\"; begin
            Pack(P, Z); Unpack(Z, B);
            exit(Format(Len(P)) + '/' + Format(Len(Z)) + '/' + Format(Len(B)));
        end;",
    ));
    assert_eq!(r, "0/0/0");
}

/// PZGzip.al: GZip_Decompress_InvalidInput. Text that is not gzip raises an InvalidData failure of CopyTo.
#[test]
fn text_that_is_not_gzip_raises_the_invalid_data_failure() {
    let r = text(run(
        "procedure F(): Text var P: Record \"GZ Blob\"; B: Record \"GZ Blob\"; begin
            Fill(P, 'this is not gzip data at all');
            asserterror Unpack(P, B);
            exit(GetLastErrorText() + '|' + GetLastErrorCode());
        end;",
    ));
    assert_eq!(
        r,
        "A call to System.IO.Compression.GZipStream.CopyTo failed with this message: The archive entry was compressed using an unsupported compression method.|DotNetInvoke:InvalidData"
    );
}

/// PZGzip.al: GZip_Decompress_EmptyAndTruncated. A truncated stream decompresses without an error to a
/// shorter result.
#[test]
fn a_truncated_stream_unpacks_to_a_shorter_result_without_an_error() {
    let r = text(run(
        "procedure F(): Text var P: Record \"GZ Blob\"; Z: Record \"GZ Blob\"; Cut: Record \"GZ Blob\"; B: Record \"GZ Blob\"; S: Text; i: Integer; St: InStream; Os: OutStream; v: Byte; begin
            for i := 1 to 100 do S += 'truncate me ';
            Fill(P, S); Pack(P, Z);
            Z.Data.CreateInStream(St); Cut.Data.CreateOutStream(Os);
            for i := 1 to Len(Z) div 2 do begin St.Read(v); Os.Write(v); end;
            Unpack(Cut, B);
            exit(Format(Len(B) < Len(P)));
        end;",
    ));
    assert_eq!(r, "Yes");
}
