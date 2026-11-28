// Review round 4 (Copilot on the merge train) of the serve-zip-blob pull request. Corpus PR 1633 measures each served
// rewrite and then reads the Blob. It does not measure a write after the rewrite (BC keeps the AL and the .NET write
// positions apart: SgDotNetStreams.al:154-172), a second .NET use of the stream after it, two archives over different
// OutStreams, a first archive with leaveOpen true or another encoding, or two StreamWriters over one OutStream. Each test
// only observes: head gives main's status, or the refusal differences.tsv names.

codeunit 59448 "NB SZ R4 Helper"
{
    // ZipBlobDotNetOverwrite.al:25-49: 48 x 'A', Position := 0, 48 x 'B' (the measured overwrite).
    procedure Overwrite48(var Buffer: Record "NB SZ Zip File" temporary; var BufferOut: OutStream; var Writer: DotNet NbSzStreamWriter)
    var
        R1: Codeunit "NB SZ R1 Helper";
        Base: DotNet NbSzStream;
    begin
        R1.WriteA48(Buffer, BufferOut, Writer, Base);
        Base.Position := 0;
        Writer.Write(PadStr('', 48, 'B'));
        Writer.Flush();
    end;

    // ZipBlobUpdateBlobChanged.al:136-160 with each archive's stream, leaveOpen and encoding given.
    procedure TwoArchives(var Buffer: Record "NB SZ Zip File" temporary; var FirstOut: OutStream; FirstLeaveOpen: Boolean; FirstEnc: DotNet NbSzEncoding; var SecondOut: OutStream)
    var
        Helper: Codeunit "NB SZ Zip Helper";
        First: DotNet NbSzZipArchive;
        Second: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
    begin
        First := First.ZipArchive(FirstOut, Mode.Update, FirstLeaveOpen, FirstEnc);
        Second := Second.ZipArchive(SecondOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(First, 'b.txt');
        Helper.AddEntry(Second, 'c.txt');
        First.Dispose();
        Second.Dispose();
    end;

    procedure Seed(var Buffer: Record "NB SZ Zip File" temporary; var BufferOut: OutStream)
    var
        Helper: Codeunit "NB SZ Zip Helper";
        SeedFile: Record "NB SZ Zip File" temporary;
        SeedIn: InStream;
    begin
        Helper.SeedIn(SeedFile, 'a.txt', SeedIn);
        CopyStream(BufferOut, SeedIn);
    end;
}

codeunit 59449 "NB SZ R4 After Rewrite 59449"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Copilot 2: an AL WriteText through the original OutStream after the measured overwrite.
    [Test]
    procedure Overwrite48_ThenALWriteText()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R4: Codeunit "NB SZ R4 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        BufferOut: OutStream;
    begin
        R4.Overwrite48(Buffer, BufferOut, Writer);
        BufferOut.WriteText('Z');
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // Copilot 2, other writer: the same StreamWriter appends after the measured overwrite.
    [Test]
    procedure Overwrite48_ThenNetAppend()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R4: Codeunit "NB SZ R4 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        BufferOut: OutStream;
    begin
        R4.Overwrite48(Buffer, BufferOut, Writer);
        Writer.Write('C');
        Writer.Flush();
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // Copilot 2, other reader of the AL position: the OutStream handed to .NET again after the measured overwrite.
    [Test]
    procedure Overwrite48_ThenSecondStreamWriter()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R4: Codeunit "NB SZ R4 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Second: DotNet NbSzStreamWriter;
        BufferOut: OutStream;
    begin
        R4.Overwrite48(Buffer, BufferOut, Writer);
        Second := Second.StreamWriter(BufferOut);
        Second.Write('C');
        Second.Flush();
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // Copilot 2, other reader of the AL position: the OutStream assigned to a DotNet Stream after the measured overwrite.
    [Test]
    procedure Overwrite48_ThenStreamView()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R4: Codeunit "NB SZ R4 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        View: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        R4.Overwrite48(Buffer, BufferOut, Writer);
        View := BufferOut;
        View.WriteByte(67);
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // Copilot 2, the other served rewrite: an AL WriteText through the fresh OutStream after the archive's Dispose.
    [Test]
    procedure Fresh_Update4_Dispose_ThenALWriteText()
    var
        Observer: Codeunit "NB SZ Observer";
        R2: Codeunit "NB SZ R2 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        BufferOut: OutStream;
    begin
        R2.SeedBuffer(Buffer);
        Buffer.Content.CreateOutStream(BufferOut);
        R2.UpdateAddDispose(BufferOut);
        BufferOut.WriteText('Z');
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;
}

codeunit 59450 "NB SZ R4 Two Archives 59450"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Copilot 3: two archives over two OutStreams opened before the seed copy (the probe used one OutStream).
    [Test]
    procedure TwoArchives_DifferentOutStreams()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R4: Codeunit "NB SZ R4 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Enc: DotNet NbSzEncoding;
        FirstOut: OutStream;
        SecondOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(FirstOut);
        Buffer.Content.CreateOutStream(SecondOut);
        R4.Seed(Buffer, SecondOut);
        R4.TwoArchives(Buffer, FirstOut, false, Enc.UTF8(), SecondOut);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Copilot 3: the first archive has leaveOpen true (the probe measured false for both).
    [Test]
    procedure TwoArchives_FirstLeaveOpenTrue()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R4: Codeunit "NB SZ R4 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Enc: DotNet NbSzEncoding;
        BufferOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        R4.Seed(Buffer, BufferOut);
        R4.TwoArchives(Buffer, BufferOut, true, Enc.UTF8(), BufferOut);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Copilot 3: the first archive uses ASCII (the probe measured UTF8 for both).
    [Test]
    procedure TwoArchives_FirstAscii()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R4: Codeunit "NB SZ R4 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Enc: DotNet NbSzEncoding;
        BufferOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        R4.Seed(Buffer, BufferOut);
        R4.TwoArchives(Buffer, BufferOut, false, Enc.ASCII(), BufferOut);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;
}

codeunit 59451 "NB SZ R4 Two Writers 59451"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // Copilot 4: two StreamWriter(OutStream) over one OutStream before either writes; the first does the measured overwrite.
    [Test]
    procedure TwoStreamWriters_FirstOverwrites48()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Other: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        Writer := Writer.StreamWriter(BufferOut);
        Other := Other.StreamWriter(BufferOut);
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
