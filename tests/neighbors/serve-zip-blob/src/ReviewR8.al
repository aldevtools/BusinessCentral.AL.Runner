// Review round 8 of the serve-zip-blob pull request (Copilot on train PR 9391): an OutStream copied after .NET got it.
// Corpus PR 1633 never copies the stream it hands .NET (ZipBlobUpdateFreshOutStream.al:146-170,
// ZipBlobUpdateBlobChanged.al:19-32, ZipBlobDotNetOverwrite.al:17-34), before or after the .NET object is made.
// Each test only observes: head gives main's answer, or refuses where the archive was already seeded with the Blob.

codeunit 59458 "NB SZ R8 Helper"
{
    // A by-value OutStream parameter: the call copies the stream.
    procedure TakeByValue(Out: OutStream)
    begin
    end;
}

codeunit 59459 "NB SZ R8 Copied 59459"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Part 3 (ZipBlobDotNetOverwrite.al:17-49) with `Copy := BufferOut` after the StreamWriter wrote 48 x 'A'.
    [Test]
    procedure Overwrite48_CopyAfterWriter()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
        Copy: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        Writer := Writer.StreamWriter(BufferOut);
        Writer.Write(PadStr('', 48, 'A'));
        Writer.Flush();
        Copy := BufferOut;
        Base := Writer.BaseStream;
        Base.Position := 0;
        Writer.Write(PadStr('', 48, 'B'));
        Writer.Flush();
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // Part 3 with the stream handed to a by-value parameter after the StreamWriter wrote 48 x 'A'.
    [Test]
    procedure Overwrite48_ByValueAfterWriter()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R8: Codeunit "NB SZ R8 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        Writer := Writer.StreamWriter(BufferOut);
        Writer.Write(PadStr('', 48, 'A'));
        Writer.Flush();
        R8.TakeByValue(BufferOut);
        Base := Writer.BaseStream;
        Base.Position := 0;
        Writer.Write(PadStr('', 48, 'B'));
        Writer.Flush();
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // Part 1 (ZipBlobUpdateFreshOutStream.al:146-172) with `Copy := Out` after the archive was made.
    [Test]
    procedure Fresh_Update4_CopyAfterCtor()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        Out: OutStream;
        Copy: OutStream;
    begin
        R2.SeedBuffer(Buffer);
        Buffer.Content.CreateOutStream(Out);
        ZipArchive := ZipArchive.ZipArchive(Out, Mode.Update, false, Enc.UTF8());
        Copy := Out;
        Helper.AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 2 (ZipBlobUpdateBlobChanged.al:98) with `Copy := BufferOut` after the archive was made, then the Clear.
    [Test]
    procedure Changed_ClearOnly_CopyAfterCtor()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Seed: Record "NB SZ Zip File" temporary;
        Buffer: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        SeedIn: InStream;
        BufferOut: OutStream;
        Copy: OutStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        Copy := BufferOut;
        Clear(Buffer.Content);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;

    // Part 2 (ZipBlobUpdateBlobChanged.al:152) with `Copy := BufferOut` between the two Disposes.
    [Test]
    procedure Changed_TwoArchives_CopyBetweenDisposes()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Seed: Record "NB SZ Zip File" temporary;
        Buffer: Record "NB SZ Zip File" temporary;
        First: DotNet NbSzZipArchive;
        Second: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        SeedIn: InStream;
        BufferOut: OutStream;
        Copy: OutStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
        First := First.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Second := Second.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(First, 'b.txt');
        Helper.AddEntry(Second, 'c.txt');
        First.Dispose();
        Copy := BufferOut;
        Second.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;
}
