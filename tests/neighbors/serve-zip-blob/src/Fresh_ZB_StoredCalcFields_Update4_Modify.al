// From corpus PR 1633 (probe-9246-zip-blob), ZipBlobUpdateFreshOutStream.al.
codeunit 59404 "NB SZ Fresh 59404"
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
    procedure ZB_StoredCalcFields_Update4_Modify()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Stored: Record "NB SZ Stored Zip File";
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        StoredOut: OutStream;
    begin
        SeedStored(Stored);
        Stored.Get(1);
        Stored.CalcFields(Content);
        Observer.Observe('seed length', Format(Stored.Content.Length));
        Stored.Content.CreateOutStream(StoredOut);
        Observer.Observe('length after CreateOutStream', Format(Stored.Content.Length));
        ZipArchive := ZipArchive.ZipArchive(StoredOut, Mode.Update, false, Enc.UTF8());
        Observer.Observe('entries in archive', Format(ZipArchive.Entries.Count));
        Helper.AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        Observer.Observe('length after Dispose', Format(Stored.Content.Length));
        Stored.Modify();
        Stored.Get(1);
        Stored.CalcFields(Content);
        Observer.Observe('stored length', Format(Stored.Content.Length));
        Observer.Observe('stored entries', Helper.StoredEntries(Stored));
        Observer.Expect('seed length', '108');
        Observer.Expect('length after CreateOutStream', '108');
        Observer.Expect('entries in archive', '1');
        Observer.Expect('length after Dispose', '194');
        Observer.Expect('stored length', '194');
        Observer.Expect('stored entries', '2:a.txt;b.txt;');
        Observer.Verify();
    end;}
