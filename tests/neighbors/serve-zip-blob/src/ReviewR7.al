// Review round 7 of the serve-zip-blob pull request: an OutStream that a procedure returns. Corpus PR 1633 hands .NET a
// scalar OutStream variable that CreateOutStream wrote, directly (ZipBlobUpdateFreshOutStream.al:146-170) or through a
// `var` parameter (ZipBlobUpdateBlobChanged.al:19-32, ZipBlobDotNetOverwrite.al:17-34). It never hands .NET a procedure's
// return value. Each test only observes: head gives main's status.

codeunit 59456 "NB SZ R7 Helper"
{
    // A named return value that CreateOutStream writes.
    procedure OpenOut(var Buffer: Record "NB SZ Zip File" temporary) Out: OutStream
    begin
        Buffer.Content.CreateOutStream(Out);
    end;

    // An unnamed return value handed back by `exit`.
    procedure OpenOutExit(var Buffer: Record "NB SZ Zip File" temporary): OutStream
    var
        O: OutStream;
    begin
        Buffer.Content.CreateOutStream(O);
        exit(O);
    end;
}

codeunit 59457 "NB SZ R7 Returned 59457"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Part 1 (ZipBlobUpdateFreshOutStream.al:146-172 steps) with the archive made over OpenOut's named return value.
    [Test]
    procedure Fresh_Update4_OverReturnedStream()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        R7: Codeunit "NB SZ R7 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
    begin
        R2.SeedBuffer(Buffer);
        ZipArchive := ZipArchive.ZipArchive(R7.OpenOut(Buffer), Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 1 with the archive made over OpenOutExit's `exit(O)` value.
    [Test]
    procedure Fresh_Update4_OverExitStream()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        R7: Codeunit "NB SZ R7 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
    begin
        R2.SeedBuffer(Buffer);
        ZipArchive := ZipArchive.ZipArchive(R7.OpenOutExit(Buffer), Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 3 (ZipBlobDotNetOverwrite.al:17-49: 48 x 'A', Position := 0, 48 x 'B') on an empty Blob, with the StreamWriter
    // made over OpenOut's return value.
    [Test]
    procedure Overwrite48_OverReturnedStream()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R7: Codeunit "NB SZ R7 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
    begin
        Writer := Writer.StreamWriter(R7.OpenOut(Buffer));
        Writer.Write(PadStr('', 48, 'A'));
        Writer.Flush();
        Base := Writer.BaseStream;
        Base.Position := 0;
        Writer.Write(PadStr('', 48, 'B'));
        Writer.Flush();
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;
}
