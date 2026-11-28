// From corpus PR 1633 (probe-9246-zip-blob), ZipBlobUpdateFreshOutStream.al.
codeunit 59407 "NB SZ Fresh 59407"
{
    Subtype = Test;
    TestPermissions = Disabled;


    local procedure SeedStored(var Stored: Record "NB SZ Stored Zip File")
    var
        Helper: Codeunit "NB SZ Zip Helper";
        Seed: Record "NB SZ Zip File" temporary;
        SeedIn: InStream;
        StoredOut: OutStream;
    begin
        Stored.DeleteAll();
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Stored.Init();
        Stored."No." := 1;
        Stored.Content.CreateOutStream(StoredOut);
        CopyStream(StoredOut, SeedIn);
        Stored.Insert();
    end;

    local procedure SeedBuffer(var Buffer: Record "NB SZ Zip File" temporary)
    var
        Helper: Codeunit "NB SZ Zip Helper";
        Seed: Record "NB SZ Zip File" temporary;
        SeedIn: InStream;
        BufferOut: OutStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
    end;
    [Test]
    procedure ZB_Temp_Update4()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        BufferOut: OutStream;
    begin
        SeedBuffer(Buffer);
        Observer.Observe('seed length', Format(Buffer.Content.Length));
        Buffer.Content.CreateOutStream(BufferOut);
        Observer.Observe('length after CreateOutStream', Format(Buffer.Content.Length));
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Observer.Observe('entries in archive', Format(ZipArchive.Entries.Count));
        Helper.AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Expect('seed length', '108');
        Observer.Expect('length after CreateOutStream', '108');
        Observer.Expect('entries in archive', '1');
        Observer.Expect('length', '194');
        Observer.Expect('entries', '2:a.txt;b.txt;');
        Observer.Verify();
    end;}
