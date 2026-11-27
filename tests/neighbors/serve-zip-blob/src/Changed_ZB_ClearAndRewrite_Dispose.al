// From corpus PR 1633 (probe-9246-zip-blob), ZipBlobUpdateBlobChanged.al.
codeunit 59411 "NB SZ Changed 59411"
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
    procedure ZB_ClearAndRewrite_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        BufferOut: OutStream;
    begin
        OpenOnSeed(Buffer, BufferOut, ZipArchive);
        RewriteWithOther(Buffer);
        Observer.Observe('length before Dispose', Format(Buffer.Content.Length));
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Expect('length before Dispose', '108');
        Observer.Expect('length', '194');
        Observer.Expect('entries', '2:a.txt;b.txt;');
        Observer.Verify();
    end;}
