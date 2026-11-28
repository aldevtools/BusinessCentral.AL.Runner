// From corpus PR 1633 (probe-9246-zip-blob), ZipBlobDotNetOverwrite.al.
codeunit 59417 "NB SZ Overwrite 59417"
{
    Subtype = Test;
    TestPermissions = Disabled;


    local procedure WriteA48(var BufferOut: OutStream; var Writer: DotNet NbSzStreamWriter)
    begin
        Writer := Writer.StreamWriter(BufferOut);
        Writer.Write(PadStr('', 48, 'A'));
        Writer.Flush();
    end;
    [Test]
    procedure ZB_Overwrite48_AtPosition0()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        WriteA48(BufferOut, Writer);
        Base := Writer.BaseStream;
        Observer.Observe('can seek', Format(Base.CanSeek));
        Observer.Observe('position after write', Format(Base.Position));
        Observer.Observe('length after write', Format(Buffer.Content.Length));
        Base.Position := 0;
        Writer.Write(PadStr('', 48, 'B'));
        Writer.Flush();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Expect('can seek', 'Yes');
        Observer.Expect('position after write', '48');
        Observer.Expect('length after write', '48');
        Observer.Expect('length', '48');
        Observer.Expect('text', 'BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB');
        Observer.Verify();
    end;}
