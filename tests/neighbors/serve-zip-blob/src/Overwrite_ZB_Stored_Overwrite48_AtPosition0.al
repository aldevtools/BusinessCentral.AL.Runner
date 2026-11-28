// From corpus PR 1633 (probe-9246-zip-blob), ZipBlobDotNetOverwrite.al.
codeunit 59420 "NB SZ Overwrite 59420"
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
    procedure ZB_Stored_Overwrite48_AtPosition0()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Stored: Record "NB SZ Stored Zip File";
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        StoredOut: OutStream;
    begin
        Stored.DeleteAll();
        Stored.Init();
        Stored."No." := 1;
        Stored.Insert();
        Stored.Content.CreateOutStream(StoredOut);
        WriteA48(StoredOut, Writer);
        Base := Writer.BaseStream;
        Base.Position := 0;
        Writer.Write(PadStr('', 48, 'B'));
        Writer.Flush();
        Stored.Modify();
        Stored.Get(1);
        Stored.CalcFields(Content);
        Observer.Observe('stored length', Format(Stored.Content.Length));
        Observer.Observe('stored text', Helper.StoredText(Stored));
        Observer.Expect('stored length', '48');
        Observer.Expect('stored text', 'BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB');
        Observer.Verify();
    end;}
