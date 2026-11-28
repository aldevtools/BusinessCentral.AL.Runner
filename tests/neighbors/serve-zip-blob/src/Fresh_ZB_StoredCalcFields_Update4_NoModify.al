// From corpus PR 1633 (probe-9246-zip-blob), ZipBlobUpdateFreshOutStream.al.
codeunit 59406 "NB SZ Fresh 59406"
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
    procedure ZB_StoredCalcFields_Update4_NoModify()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Stored: Record "NB SZ Stored Zip File";
        Again: Record "NB SZ Stored Zip File";
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        StoredOut: OutStream;
    begin
        SeedStored(Stored);
        Stored.Get(1);
        Stored.CalcFields(Content);
        Stored.Content.CreateOutStream(StoredOut);
        ZipArchive := ZipArchive.ZipArchive(StoredOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        Observer.Observe('length after Dispose', Format(Stored.Content.Length));
        Observer.Observe('entries of the variable', Helper.StoredEntries(Stored));
        Again.Get(1);
        Again.CalcFields(Content);
        Observer.Observe('length in the database', Format(Again.Content.Length));
        Observer.Expect('length after Dispose', '194');
        Observer.Expect('entries of the variable', '2:a.txt;b.txt;');
        Observer.Expect('length in the database', '108');
        Observer.Verify();
    end;}
