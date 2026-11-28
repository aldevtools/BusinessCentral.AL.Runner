// From corpus PR 1633 (probe-9246-zip-blob), ZipBlobDotNetOverwrite.al.
codeunit 59418 "NB SZ Overwrite 59418"
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
    procedure ZB_SetLength10()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        BufferOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        WriteA48(BufferOut, Writer);
        Writer.BaseStream.SetLength(10);
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Expect('length', '10');
        Observer.Expect('text', 'AAAAAAAAAA');
        Observer.Verify();
    end;}
