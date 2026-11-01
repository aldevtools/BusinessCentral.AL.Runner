//! An InStream whose owner (the Blob field of a local temporary record) went out of scope reads as an empty stream for every
//! use, and a cleared one raises NotInitialized. Every literal is what real BC answered in the corpus:
//! streams/lt-y62-gone-instream/Y62GoneInStream.al (codeunit 93265, writer Y62).

use al_testkit::text;

const SRC: &str = r#"
table 50192 B
{
    TableType = Temporary;
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}

codeunit 50192 C
{
    var
        Sink: Record B temporary;

    local procedure Gone(var InS: InStream)
    var
        Rec: Record B temporary;
        OutS: OutStream;
    begin
        Rec.Data.CreateOutStream(OutS);
        OutS.WriteText('Dummy test file content');
        Rec.Data.CreateInStream(InS);
    end;

    [TryFunction]
    local procedure TryLength(var InS: InStream; var Txt: Text)
    begin
        Txt := Format(InS.Length());
    end;

    procedure Uses(): Text
    var
        InS: InStream;
        Content: HttpContent;
        OutS: OutStream;
        SinkIn: InStream;
        T: Text;
        Out: Text;
    begin
        Gone(InS);
        Content.WriteFrom(InS);
        Content.ReadAs(T);
        Out := 'readas=[' + T + ']|len=' + Format(InS.Length()) + '|eos=' + Format(InS.EOS());
        Clear(InS);
        Gone(InS);
        Sink.Data.CreateOutStream(OutS);
        CopyStream(OutS, InS);
        Sink.Data.CreateInStream(SinkIn);
        SinkIn.ReadText(T);
        Out += ';sink=[' + T + ']|hasvalue=' + Format(Sink.Data.HasValue());
        Clear(InS);
        Gone(InS);
        T := 'preset';
        Out += ';read=' + Format(InS.Read(T, 5)) + '[' + T + ']|pos=' + Format(InS.Position());
        Clear(InS);
        if TryLength(InS, T) then
            Out += ';cleared=' + T
        else
            Out += ';cleared=' + GetLastErrorText() + '|' + GetLastErrorCode();
        exit(Out);
    end;
}
"#;

/// GoneStream_HttpContentWriteFrom, GoneStream_CopyStream_ToOutStream, GoneStream_ReadAndPosition and GoneStream_Clear.
#[test]
fn a_stream_whose_owner_went_out_of_scope_is_an_empty_stream_and_a_cleared_one_raises() {
    assert_eq!(
        text(al_testkit::run(SRC, "Uses")),
        "readas=[]|len=0|eos=Yes;sink=[]|hasvalue=No;read=0[]|pos=1;\
         cleared=Microsoft.Dynamics.Nav.Runtime.NavInStream variable not initialized.|NotInitialized"
    );
}
