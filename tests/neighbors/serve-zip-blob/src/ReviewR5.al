// Review round 5 (Copilot follow-up) of the serve-zip-blob pull request: a copy of the OutStream. Corpus PR 1633 hands
// .NET the OutStream variable CreateOutStream wrote (directly or through a `var` parameter). It does not measure a copy:
// `Out2 := BufferOut`, the stream handed to a by-value parameter, or an array of streams copied whole or handed by value.
// Each test only observes. The scalar copies give main's status; the three array tests are still served (open,
// al-runner issue 9373; differences.tsv).

codeunit 59452 "NB SZ R5 Helper"
{
    // ZipBlobUpdateFreshOutStream.al:150-170 with the archive made over a by-value OutStream parameter.
    procedure UpdateAddDisposeByValue(Out: OutStream)
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

    // The archive over the first element of a by-value array of OutStreams.
    procedure UpdateFirstOfArrayByValue(Outs: array[1] of OutStream)
    var
        R2: Codeunit "NB SZ R2 Helper";
    begin
        R2.UpdateAddDispose(Outs[1]);
    end;

    // ZipBlobUpdateBlobChanged.al:152-180 with both archives made over a by-value OutStream parameter.
    procedure TwoArchivesByValue(Out: OutStream)
    var
        Helper: Codeunit "NB SZ Zip Helper";
        First: DotNet NbSzZipArchive;
        Second: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
    begin
        First := First.ZipArchive(Out, Mode.Update, false, Enc.UTF8());
        Second := Second.ZipArchive(Out, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(First, 'b.txt');
        Helper.AddEntry(Second, 'c.txt');
        First.Dispose();
        Second.Dispose();
    end;
}

codeunit 59453 "NB SZ R5 Copies 59453"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // :152 with the second archive over an assigned copy of the OutStream.
    [Test]
    procedure TwoArchives_SecondOverAssignedCopy()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R4: Codeunit "NB SZ R4 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Enc: DotNet NbSzEncoding;
        BufferOut: OutStream;
        Out2: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        R4.Seed(Buffer, BufferOut);
        Out2 := BufferOut;
        R4.TwoArchives(Buffer, BufferOut, false, Enc.UTF8(), Out2);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // :152 with both archives over an assigned copy of the OutStream.
    [Test]
    procedure TwoArchives_BothOverAssignedCopy()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R4: Codeunit "NB SZ R4 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Enc: DotNet NbSzEncoding;
        BufferOut: OutStream;
        Out2: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        R4.Seed(Buffer, BufferOut);
        Out2 := BufferOut;
        R4.TwoArchives(Buffer, Out2, false, Enc.UTF8(), Out2);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // :152 with both archives over a by-value OutStream parameter.
    [Test]
    procedure TwoArchives_ByValueParameter()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R4: Codeunit "NB SZ R4 Helper";
        R5: Codeunit "NB SZ R5 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        BufferOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        R4.Seed(Buffer, BufferOut);
        R5.TwoArchivesByValue(BufferOut);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 1 (fresh CreateOutStream) with the archive over an assigned copy of the OutStream.
    [Test]
    procedure Fresh_Update4_OverAssignedCopy()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        BufferOut: OutStream;
        Out2: OutStream;
    begin
        R2.SeedBuffer(Buffer);
        Buffer.Content.CreateOutStream(BufferOut);
        Out2 := BufferOut;
        R2.UpdateAddDispose(Out2);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 1 with the archive over the original OutStream after a copy was assigned from it.
    [Test]
    procedure Fresh_Update4_OverOriginalAfterCopy()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        BufferOut: OutStream;
        Out2: OutStream;
    begin
        R2.SeedBuffer(Buffer);
        Buffer.Content.CreateOutStream(BufferOut);
        Out2 := BufferOut;
        R2.UpdateAddDispose(BufferOut);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 1 with the archive over a by-value OutStream parameter.
    [Test]
    procedure Fresh_Update4_ByValueParameter()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        R5: Codeunit "NB SZ R5 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        BufferOut: OutStream;
    begin
        R2.SeedBuffer(Buffer);
        Buffer.Content.CreateOutStream(BufferOut);
        R5.UpdateAddDisposeByValue(BufferOut);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 1 with the archive over an element of an array assigned whole from the array CreateOutStream wrote.
    [Test]
    procedure Fresh_Update4_OverArrayCopyElement()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Outs: array[1] of OutStream;
        Copies: array[1] of OutStream;
    begin
        R2.SeedBuffer(Buffer);
        Buffer.Content.CreateOutStream(Outs[1]);
        Copies := Outs;
        R2.UpdateAddDispose(Copies[1]);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 1 with the archive over an element of an array of OutStreams handed by value.
    [Test]
    procedure Fresh_Update4_ArrayByValue()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        R5: Codeunit "NB SZ R5 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Outs: array[1] of OutStream;
    begin
        R2.SeedBuffer(Buffer);
        Buffer.Content.CreateOutStream(Outs[1]);
        R5.UpdateFirstOfArrayByValue(Outs);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;

    // Part 1 over an element of the array CreateOutStream wrote (no copy): served, as the measured variable.
    [Test]
    procedure Fresh_Update4_OverArrayElement()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        R2: Codeunit "NB SZ R2 Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Outs: array[1] of OutStream;
    begin
        R2.SeedBuffer(Buffer);
        Buffer.Content.CreateOutStream(Outs[1]);
        R2.UpdateAddDispose(Outs[1]);
        Observer.Observe('entries', Helper.BufferEntries(Buffer));
        Observer.Verify();
    end;
}
