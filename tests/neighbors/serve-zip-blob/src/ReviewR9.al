// Review round 9 of the serve-zip-blob pull request (Copilot on train PR 9403): the OutStream variable .NET got is opened
// again on another Blob before the rewrite. Corpus PR 1633 never opens that variable again (ZipBlobUpdateFreshOutStream.al:
// 146-170, ZipBlobUpdateBlobChanged.al:19-32, ZipBlobDotNetOverwrite.al:17-34). The other Blob is given as many edits as
// the first, so the variable's new stamp equals the one the bridge saw. Each test only observes: head gives main's answer,
// or refuses where the archive was already seeded with the Blob.

codeunit 59460 "NB SZ R9 Reopened 59460"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Part 3 (ZipBlobDotNetOverwrite.al:17-49) with `Copy := BufferOut; Other.Content.CreateOutStream(BufferOut)` after the
    // StreamWriter wrote 48 x 'A'.
    [Test]
    procedure Overwrite48_CopyThenReopenOther()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
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
        Other.Content.CreateOutStream(BufferOut);
        Base := Writer.BaseStream;
        Base.Position := 0;
        Writer.Write(PadStr('', 48, 'B'));
        Writer.Flush();
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Observe('other', Helper.BlobText(Other));
        Observer.Verify();
    end;

    // Part 3 with `Other.Content.CreateOutStream(BufferOut)` after the StreamWriter wrote 48 x 'A', no copy.
    [Test]
    procedure Overwrite48_ReopenOther()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        Writer := Writer.StreamWriter(BufferOut);
        Writer.Write(PadStr('', 48, 'A'));
        Writer.Flush();
        Other.Content.CreateOutStream(BufferOut);
        Base := Writer.BaseStream;
        Base.Position := 0;
        Writer.Write(PadStr('', 48, 'B'));
        Writer.Flush();
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Observe('other', Helper.BlobText(Other));
        Observer.Verify();
    end;

    // Part 2 (ZipBlobUpdateBlobChanged.al:98) with `Copy := BufferOut; Other.Content.CreateOutStream(BufferOut)` after the
    // archive was made, then the Clear.
    [Test]
    procedure Changed_ClearOnly_CopyThenReopenOther()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Seed: Record "NB SZ Zip File" temporary;
        Buffer: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
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
        Other.Content.CreateOutStream(BufferOut);
        Clear(Buffer.Content);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('other', Format(Other.Content.Length));
        Observer.Verify();
    end;

    // Part 1 (ZipBlobUpdateFreshOutStream.al:146-172) with `Other.Content.CreateOutStream(Out)` after the archive was made;
    // Other is seeded as Buffer is, so both opens leave the same edit count.
    [Test]
    procedure Fresh_Update4_ReopenOther()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        Out: OutStream;
    begin
        R2.SeedBuffer(Buffer);
        R2.SeedBuffer(Other);
        Buffer.Content.CreateOutStream(Out);
        ZipArchive := ZipArchive.ZipArchive(Out, Mode.Update, false, Enc.UTF8());
        Other.Content.CreateOutStream(Out);
        Helper.AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;
}
