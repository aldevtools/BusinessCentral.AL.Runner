// Review round 1 of the serve-zip-blob pull request: shapes next to corpus PR 1633 that the probe did not measure. Each
// test only observes: head gives main's status, or the refusal differences.tsv names.

codeunit 59444 "NB SZ R1 Helper"
{
    // 48 x 'A' through StreamWriter(OutStream) on an empty temporary Blob (ZipBlobDotNetOverwrite.al:17-22), then Base.
    procedure WriteA48(var Buffer: Record "NB SZ Zip File" temporary; var BufferOut: OutStream; var Writer: DotNet NbSzStreamWriter; var Base: DotNet NbSzStream)
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        Writer := Writer.StreamWriter(BufferOut);
        Writer.Write(PadStr('', 48, 'A'));
        Writer.Flush();
        Base := Writer.BaseStream;
    end;

    // ZipBlobUpdateBlobChanged.al:19-32 with the archive's leaveOpen and encoding given.
    procedure OpenOnSeed(var Buffer: Record "NB SZ Zip File" temporary; var BufferOut: OutStream; var ZipArchive: DotNet NbSzZipArchive; LeaveOpen: Boolean; Enc: DotNet NbSzEncoding)
    var
        Helper: Codeunit "NB SZ Zip Helper";
        Seed: Record "NB SZ Zip File" temporary;
        Mode: DotNet NbSzZipMode;
        SeedIn: InStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, LeaveOpen, Enc);
        Helper.AddEntry(ZipArchive, 'b.txt');
    end;
}

codeunit 59431 "NB SZ R1 Overwrite 59431"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // H1: another writer (BinaryWriter) writes after Position := 0; the probe measured the StreamWriter's write only.
    [Test]
    procedure Overwrite_BinaryWriterAfterPosition0()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Bw: DotNet NbSzBinaryWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        R1.WriteA48(Buffer, BufferOut, Writer, Base);
        Base.Position := 0;
        Bw := Bw.BinaryWriter(Base);
        Bw.Write(PadStr('', 10, 'B'));
        Bw.Flush();
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // H1: a .NET MemoryStream copies itself into the stream after Position := 0 (the stream is an argument, not the target).
    [Test]
    procedure Overwrite_CopyToAfterPosition0()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        SrcWriter: DotNet NbSzStreamWriter;
        Src: DotNet NbSzMemoryStream;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        Src := Src.MemoryStream();
        SrcWriter := SrcWriter.StreamWriter(Src);
        SrcWriter.Write(PadStr('', 48, 'B'));
        SrcWriter.Flush();
        Src.Position := 0;
        R1.WriteA48(Buffer, BufferOut, Writer, Base);
        Base.Position := 0;
        Src.CopyTo(Base);
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // H2: Position := 0, then Position := 20; the write reaches the end (28 bytes), so only the start position differs.
    [Test]
    procedure Overwrite_Position0ThenPosition20()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        R1.WriteA48(Buffer, BufferOut, Writer, Base);
        Base.Position := 0;
        Base.Position := 20;
        Writer.Write(PadStr('', 28, 'B'));
        Writer.Flush();
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // H2: Position := 0, then ReadByte moves the stream to 1 (an AL call on the stream itself); the write reaches the end
    // (47 bytes). Seek(5, Begin) does not compile here: `Begin` is an AL keyword.
    [Test]
    procedure Overwrite_Position0ThenReadByte()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        R1.WriteA48(Buffer, BufferOut, Writer, Base);
        Base.Position := 0;
        Base.ReadByte();
        Writer.Write(PadStr('', 47, 'B'));
        Writer.Flush();
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // H2: a second write-over cycle after the measured one (the probe measured one).
    [Test]
    procedure Overwrite_SecondCycleAtPosition0()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        R1.WriteA48(Buffer, BufferOut, Writer, Base);
        Base.Position := 0;
        Writer.Write(PadStr('', 48, 'B'));
        Writer.Flush();
        Base.Position := 0;
        Writer.Write(PadStr('', 48, 'C'));
        Writer.Flush();
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // H5: the 10-byte overwrite leaves the .NET stream at 10 of 48; an AL write after it is not measured.
    [Test]
    procedure Overwrite10_ThenALWriteText()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        R1.WriteA48(Buffer, BufferOut, Writer, Base);
        Base.Position := 0;
        Writer.Write(PadStr('', 10, 'B'));
        Writer.Flush();
        BufferOut.WriteText('Z');
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // M1: SetLength to another length than the measured 10.
    [Test]
    procedure Overwrite_SetLength0()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        R1.WriteA48(Buffer, BufferOut, Writer, Base);
        Base.SetLength(0);
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;
}

codeunit 59432 "NB SZ R1 Changed 59432"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // H3: leaveOpen true; the probe measured false.
    [Test]
    procedure Changed_LeaveOpenTrue_ClearOnly_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Enc: DotNet NbSzEncoding;
        BufferOut: OutStream;
    begin
        R1.OpenOnSeed(Buffer, BufferOut, ZipArchive, true, Enc.UTF8());
        Clear(Buffer.Content);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;

    // H3: ASCII; the probe measured UTF8.
    [Test]
    procedure Changed_Ascii_ClearOnly_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Enc: DotNet NbSzEncoding;
        BufferOut: OutStream;
    begin
        R1.OpenOnSeed(Buffer, BufferOut, ZipArchive, false, Enc.ASCII());
        Clear(Buffer.Content);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;

    // H4: a rewrite of as many bytes through a new CreateOutStream, without a Clear first.
    [Test]
    procedure Changed_RewriteWithoutClear_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Enc: DotNet NbSzEncoding;
        OtherIn: InStream;
        BufferOut: OutStream;
        NewOut: OutStream;
    begin
        R1.OpenOnSeed(Buffer, BufferOut, ZipArchive, false, Enc.UTF8());
        Helper.SeedIn(Other, 'z.txt', OtherIn);
        Buffer.Content.CreateOutStream(NewOut);
        CopyStream(NewOut, OtherIn);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;

    // H4: Clear, then as many bytes written through the archive's own OutStream (no new CreateOutStream).
    [Test]
    procedure Changed_ClearThenWriteOriginalOut_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R1: Codeunit "NB SZ R1 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Enc: DotNet NbSzEncoding;
        OtherIn: InStream;
        BufferOut: OutStream;
    begin
        R1.OpenOnSeed(Buffer, BufferOut, ZipArchive, false, Enc.UTF8());
        Helper.SeedIn(Other, 'z.txt', OtherIn);
        Clear(Buffer.Content);
        CopyStream(BufferOut, OtherIn);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;
}

codeunit 59433 "NB SZ R1 Fresh 59433"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // H6: Import after CreateOutStream also leaves the stream at 0 with content; the probe measured CreateOutStream last.
    [Test]
    procedure Fresh_ImportAfterCreateOutStream_Update4()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Seed: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
        ZipFile: File;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        SeedIn: InStream;
        OtherIn: InStream;
        FileOut: OutStream;
        BufferOut: OutStream;
    begin
        Helper.SeedIn(Other, 'z.txt', OtherIn);
        ZipFile.Create('nbsz-other.zip');
        ZipFile.CreateOutStream(FileOut);
        CopyStream(FileOut, OtherIn);
        ZipFile.Close();
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        Buffer.Content.Import('nbsz-other.zip');
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;

    // H7: a Clear between a fresh archive's constructor and its Dispose. Head refuses (the seed was decided at the
    // constructor); main passed with an archive that never read the Blob. Cells D5-D7 for a probe writer.
    [Test]
    procedure Fresh_ClearBeforeDispose()
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
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        Clear(Buffer.Content);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;
}
