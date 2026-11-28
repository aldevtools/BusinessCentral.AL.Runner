// Review round 2 of the serve-zip-blob pull request: two OutStreams on one Blob, and a Blob a read gave the record. The
// probe (corpus PR 1633) hands .NET the newest OutStream only, writes the rewrite through the new OutStream only, and uses a
// temporary record that was never inserted or read. Each test only observes: head gives main's result.

codeunit 59445 "NB SZ R2 Helper"
{
    // ZipBlobUpdateFreshOutStream.al:36-46: the seed zip copied into the Blob through a stream that is gone afterwards.
    procedure SeedBuffer(var Buffer: Record "NB SZ Zip File" temporary)
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

    procedure UpdateAddDispose(var Out: OutStream)
    var
        Helper: Codeunit "NB SZ Zip Helper";
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
    begin
        ZipArchive := ZipArchive.ZipArchive(Out, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
    end;
}

codeunit 59446 "NB SZ R2 Fresh 59446"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // HIGH 1, cell F1: the archive gets the older of two live OutStreams; the newest open is the other one.
    [Test]
    procedure Fresh_TwoCreateOutStream_ArchiveOverFirst_Update4()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Out1: OutStream;
        Out2: OutStream;
    begin
        R2.SeedBuffer(Buffer);
        Buffer.Content.CreateOutStream(Out1);
        Buffer.Content.CreateOutStream(Out2);
        R2.UpdateAddDispose(Out1);
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // HIGH 1: the zip copied through the first OutStream, a second CreateOutStream, then the archive over the first one.
    [Test]
    procedure Fresh_TwoCreateOutStream_CopyThenArchiveOverFirst_Update4()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Seed: Record "NB SZ Zip File" temporary;
        SeedIn: InStream;
        Out1: OutStream;
        Out2: OutStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(Out1);
        CopyStream(Out1, SeedIn);
        Buffer.Content.CreateOutStream(Out2);
        R2.UpdateAddDispose(Out1);
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;
}

codeunit 59447 "NB SZ R2 Changed 59447"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // HIGH 1, cell F2: after the Clear and a new CreateOutStream, the rewrite goes through the archive's own OutStream
    // (ZipBlobUpdateBlobChanged.al:37-46 writes through the new one).
    [Test]
    procedure Changed_ClearNewOut_WriteThroughOriginalOut_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Enc: DotNet NbSzEncoding;
        BufferOut: OutStream;
        NewOut: OutStream;
        OtherIn: InStream;
    begin
        R1.OpenOnSeed(Buffer, BufferOut, ZipArchive, false, Enc.UTF8());
        Helper.SeedIn(Other, 'z.txt', OtherIn);
        Clear(Buffer.Content);
        Buffer.Content.CreateOutStream(NewOut);
        CopyStream(BufferOut, OtherIn);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;

    // HIGH 2: the row was inserted and modified, and Get gave the record its Blob before the stream was opened
    // (ZipBlobUpdateBlobChanged.al:19-31 uses a record never inserted or read).
    [Test]
    procedure Changed_TempGetModified_ClearOnly_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Seed: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        SeedIn: InStream;
        BufferOut: OutStream;
    begin
        Buffer."No." := 1;
        Buffer.Insert();
        Helper.MakeZip(Buffer, 'a.txt');
        Buffer.Modify();
        Buffer.Get(1);
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        Clear(Buffer.Content);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;
}
