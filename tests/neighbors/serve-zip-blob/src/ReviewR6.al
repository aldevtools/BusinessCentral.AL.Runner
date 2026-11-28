// Review round 6 of the serve-zip-blob pull request: where an OutStream lives. Corpus PR 1633 hands .NET a scalar
// OutStream variable that CreateOutStream wrote, directly or through a `var` parameter, in all three parts
// (ZipBlobUpdateFreshOutStream.al, ZipBlobUpdateBlobChanged.al:19-32, ZipBlobDotNetOverwrite.al:17-34). It does not
// measure an OutStream stored in an array element (by CreateOutStream, by a `var` write-back, inside a `var` array
// parameter, in a global array assigned whole) or a copy of it in parts 2 and 3. Each test only observes: head gives
// main's status, except Overwrite48_OverVarScalar (the measured scalar shape, served; differences.tsv).

codeunit 59454 "NB SZ R6 Helper"
{
    procedure MakeOut(var Buffer: Record "NB SZ Zip File" temporary; var Out: OutStream)
    begin
        Buffer.Content.CreateOutStream(Out);
    end;

    // Part 1 with CreateOutStream into an element of a `var` array parameter.
    procedure UpdateFirstOfVarArray(var Buffer: Record "NB SZ Zip File" temporary; var Outs: array[1] of OutStream)
    var
        R2: Codeunit "NB SZ R2 Helper";
    begin
        Buffer.Content.CreateOutStream(Outs[1]);
        R2.UpdateAddDispose(Outs[1]);
    end;

    // Part 1 inside a procedure whose `var` OutStream parameter the caller bound to an array element: CreateOutStream and
    // the archive both work on the parameter (the probe's `var` parameter shape, ZipBlobUpdateFreshOutStream.al:150-170).
    procedure MakeAndUpdate(var Buffer: Record "NB SZ Zip File" temporary; var Out: OutStream)
    var
        R2: Codeunit "NB SZ R2 Helper";
    begin
        Buffer.Content.CreateOutStream(Out);
        R2.UpdateAddDispose(Out);
    end;

    // ZipBlobUpdateBlobChanged.al:19-32 (OpenOnSeed) with the archive made over Out, which the caller opened.
    procedure SeedAndOpen(var Out: OutStream; var ZipArchive: DotNet NbSzZipArchive)
    var
        Helper: Codeunit "NB SZ Zip Helper";
        Seed: Record "NB SZ Zip File" temporary;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        SeedIn: InStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        CopyStream(Out, SeedIn);
        ZipArchive := ZipArchive.ZipArchive(Out, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
    end;

    // ZipBlobDotNetOverwrite.al:17-49 (48 x 'A', Position := 0, 48 x 'B') with the StreamWriter made over Out.
    procedure Overwrite48Over(var Out: OutStream)
    var
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
    begin
        Writer := Writer.StreamWriter(Out);
        Writer.Write(PadStr('', 48, 'A'));
        Writer.Flush();
        Base := Writer.BaseStream;
        Base.Position := 0;
        Writer.Write(PadStr('', 48, 'B'));
        Writer.Flush();
    end;
}

codeunit 59455 "NB SZ R6 Holders 59455"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        GlobalOuts: array[1] of OutStream;

    // Part 1 over an array element that a `var` parameter's CreateOutStream wrote back.
    [Test]
    procedure Fresh_Update4_ElementByVarMaker()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        R6: Codeunit "NB SZ R6 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Outs: array[1] of OutStream;
    begin
        R2.SeedBuffer(Buffer);
        R6.MakeOut(Buffer, Outs[1]);
        R2.UpdateAddDispose(Outs[1]);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 1 in a callee that gets an array element as its `var` OutStream parameter (the caller holds it in an array).
    [Test]
    procedure Fresh_Update4_VarParamBoundToElement()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        R6: Codeunit "NB SZ R6 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Outs: array[1] of OutStream;
    begin
        R2.SeedBuffer(Buffer);
        R6.MakeAndUpdate(Buffer, Outs[1]);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 1 over an element of a `var` array parameter.
    [Test]
    procedure Fresh_Update4_VarArrayParam()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        R6: Codeunit "NB SZ R6 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Outs: array[1] of OutStream;
    begin
        R2.SeedBuffer(Buffer);
        R6.UpdateFirstOfVarArray(Buffer, Outs);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 1 over an element of a global array assigned whole from a local one.
    [Test]
    procedure Fresh_Update4_GlobalArrayCopy()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Outs: array[1] of OutStream;
    begin
        R2.SeedBuffer(Buffer);
        Buffer.Content.CreateOutStream(Outs[1]);
        GlobalOuts := Outs;
        R2.UpdateAddDispose(GlobalOuts[1]);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 2, :98 Clear only, with the archive over an assigned copy of the OutStream.
    [Test]
    procedure ClearOnly_OverAssignedCopy()
    var
        Observer: Codeunit "NB SZ Observer";
        R6: Codeunit "NB SZ R6 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        BufferOut: OutStream;
        Copy: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        Copy := BufferOut;
        R6.SeedAndOpen(Copy, ZipArchive);
        Clear(Buffer.Content);
        ZipArchive.Dispose();
        Observer.Observe('has value', Format(Buffer.Content.HasValue()));
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;

    // Part 2, :98 Clear only, with the archive over an array element.
    [Test]
    procedure ClearOnly_OverArrayElement()
    var
        Observer: Codeunit "NB SZ Observer";
        R6: Codeunit "NB SZ R6 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Outs: array[1] of OutStream;
    begin
        Buffer.Content.CreateOutStream(Outs[1]);
        R6.SeedAndOpen(Outs[1], ZipArchive);
        Clear(Buffer.Content);
        ZipArchive.Dispose();
        Observer.Observe('has value', Format(Buffer.Content.HasValue()));
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;

    // Part 2, :152, with both archives over one array element.
    [Test]
    procedure TwoArchives_OverArrayElement()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R4: Codeunit "NB SZ R4 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Enc: DotNet NbSzEncoding;
        Outs: array[1] of OutStream;
    begin
        Buffer.Content.CreateOutStream(Outs[1]);
        R4.Seed(Buffer, Outs[1]);
        R4.TwoArchives(Buffer, Outs[1], false, Enc.UTF8(), Outs[1]);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 3 with the StreamWriter over an assigned copy of the OutStream.
    [Test]
    procedure Overwrite48_OverAssignedCopy()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R6: Codeunit "NB SZ R6 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        BufferOut: OutStream;
        Copy: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        Copy := BufferOut;
        R6.Overwrite48Over(Copy);
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // Part 3 with the StreamWriter over an array element.
    [Test]
    procedure Overwrite48_OverArrayElement()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R6: Codeunit "NB SZ R6 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Outs: array[1] of OutStream;
    begin
        Buffer.Content.CreateOutStream(Outs[1]);
        R6.Overwrite48Over(Outs[1]);
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;

    // Control: part 3 over the scalar variable through a `var` parameter, as measured (head serves it).
    [Test]
    procedure Overwrite48_OverVarScalar()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R6: Codeunit "NB SZ R6 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        BufferOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        R6.Overwrite48Over(BufferOut);
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;
}
