// From corpus PR 1633 (probe-9246-zip-blob), ZipBlobUpdateBlobChanged.al.
// Served bytes 0-34 contradicted by corpus PR 1640 D6; refused until a cell measures this shape's bytes (al-runner issue 9351).
codeunit 59412 "NB SZ Changed 59412"
{
    Subtype = Test;
    TestPermissions = Disabled;


    local procedure OpenOnSeed(var Buffer: Record "NB SZ Zip File" temporary; var BufferOut: OutStream; var ZipArchive: DotNet NbSzZipArchive)
    var
        Helper: Codeunit "NB SZ Zip Helper";
        Seed: Record "NB SZ Zip File" temporary;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        SeedIn: InStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
    end;

    local procedure RewriteWithOther(var Buffer: Record "NB SZ Zip File" temporary)
    var
        Helper: Codeunit "NB SZ Zip Helper";
        Other: Record "NB SZ Zip File" temporary;
        OtherIn: InStream;
        NewOut: OutStream;
    begin
        Helper.SeedIn(Other, 'z.txt', OtherIn);
        Clear(Buffer.Content);
        Buffer.Content.CreateOutStream(NewOut);
        CopyStream(NewOut, OtherIn);
    end;
    [Test]
    procedure ZB_ClearAndRewrite_AppendOriginal_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Writer: DotNet NbSzStreamWriter;
        BufferOut: OutStream;
    begin
        OpenOnSeed(Buffer, BufferOut, ZipArchive);
        RewriteWithOther(Buffer);
        Writer := Writer.StreamWriter(BufferOut);
        Writer.Write('xyz');
        Writer.Flush();
        Observer.Observe('length after append', Format(Buffer.Content.Length));
        Observer.Observe('last bytes after append', Helper.LastBytes(Buffer));
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('last bytes', Helper.LastBytes(Buffer));
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Expect('length after append', '111');
        Observer.Expect('last bytes after append', '120,121,122');
        Observer.Expect('length', '194');
        Observer.Expect('last bytes', '0,0,0');
        Observer.Expect('entries', '2:a.txt;b.txt;');
        Observer.Verify();
    end;}
