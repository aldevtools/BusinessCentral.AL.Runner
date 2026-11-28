// Shapes next to corpus PR 1633 that the probe did not measure. Each test only observes: it checks that head gives main's
// status (or the refusal differences.tsv names), not a BC value.

codeunit 59421 "NB SZ Fresh ReadMode 59421"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ZB_Temp_FreshOutStream_ReadMode()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Seed: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        SeedIn: InStream;
        BufferOut: OutStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Read);
        Observer.Observe('entries in archive', Format(ZipArchive.Entries.Count));
        Observer.Verify();
    end;
}

codeunit 59422 "NB SZ Fresh Unicode 59422"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ZB_Temp_FreshOutStream_Update4_Unicode()
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
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.Unicode());
        Helper.AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;
}

codeunit 59423 "NB SZ Changed Stored 59423"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ZB_Stored_GetCalcFields_ClearAndRewrite_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Stored: Record "NB SZ Stored Zip File";
        Seed: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        SeedIn: InStream;
        OtherIn: InStream;
        StoredOut: OutStream;
        NewOut: OutStream;
    begin
        Stored.DeleteAll();
        Stored.Init();
        Stored."No." := 1;
        Stored.Insert();
        Stored.Get(1);
        Stored.CalcFields(Content);
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Stored.Content.CreateOutStream(StoredOut);
        CopyStream(StoredOut, SeedIn);
        ZipArchive := ZipArchive.ZipArchive(StoredOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        Helper.SeedIn(Other, 'z.txt', OtherIn);
        Clear(Stored.Content);
        Stored.Content.CreateOutStream(NewOut);
        CopyStream(NewOut, OtherIn);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Stored.Content.Length));
        Observer.Verify();
    end;
}

codeunit 59424 "NB SZ Changed NetAppend 59424"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ZB_NetAppend_NoALChange_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Seed: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        Writer: DotNet NbSzStreamWriter;
        SeedIn: InStream;
        BufferOut: OutStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        Writer := Writer.StreamWriter(BufferOut);
        Writer.Write('xyz');
        Writer.Flush();
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;
}

codeunit 59425 "NB SZ Changed OtherLen 59425"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ZB_ClearAndRewrite_OtherLength_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Seed: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        SeedIn: InStream;
        OtherIn: InStream;
        BufferOut: OutStream;
        NewOut: OutStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        Helper.SeedIn(Other, 'a-much-longer-entry-name.txt', OtherIn);
        Clear(Buffer.Content);
        Buffer.Content.CreateOutStream(NewOut);
        CopyStream(NewOut, OtherIn);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;
}

codeunit 59426 "NB SZ Overwrite Utf8 59426"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ZB_Overwrite48_AtPosition0_WriterWithEncoding()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Enc: DotNet NbSzEncoding;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        Writer := Writer.StreamWriter(BufferOut, Enc.UTF8());
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

codeunit 59427 "NB SZ Fresh ALWrite 59427"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ZB_Temp_FreshOutStream_Update4_ALWriteBeforeDispose()
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
        BufferOut.WriteText('x');
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Verify();
    end;
}

codeunit 59428 "NB SZ Changed StoredNew 59428"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // A record of a stored table that no read or Insert gave its Blob: the probe measured the Clear-and-rewrite on a
    // temporary record only, so head keeps main's status.
    [Test]
    procedure ZB_StoredNeverInserted_ClearAndRewrite_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Stored: Record "NB SZ Stored Zip File";
        Seed: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        SeedIn: InStream;
        OtherIn: InStream;
        StoredOut: OutStream;
        NewOut: OutStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Stored.Content.CreateOutStream(StoredOut);
        CopyStream(StoredOut, SeedIn);
        ZipArchive := ZipArchive.ZipArchive(StoredOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        Helper.SeedIn(Other, 'z.txt', OtherIn);
        Clear(Stored.Content);
        Stored.Content.CreateOutStream(NewOut);
        CopyStream(NewOut, OtherIn);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Stored.Content.Length));
        Observer.Verify();
    end;
}

// Served bytes 0-34 contradicted by corpus PR 1640 D6; refused until a cell measures this shape's bytes (al-runner issue 9351).
codeunit 59429 "NB SZ Changed DeclTemp 59429"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // A record of a stored table declared temporary: a temporary record, as in the probe.
    [Test]
    procedure ZB_DeclaredTemporary_ClearAndRewrite_Dispose()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Stored Zip File" temporary;
        Seed: Record "NB SZ Zip File" temporary;
        Other: Record "NB SZ Zip File" temporary;
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        Enc: DotNet NbSzEncoding;
        SeedIn: InStream;
        OtherIn: InStream;
        BufferOut: OutStream;
        NewOut: OutStream;
    begin
        Helper.SeedIn(Seed, 'a.txt', SeedIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, SeedIn);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        Helper.SeedIn(Other, 'z.txt', OtherIn);
        Clear(Buffer.Content);
        Buffer.Content.CreateOutStream(NewOut);
        CopyStream(NewOut, OtherIn);
        ZipArchive.Dispose();
        Observer.Observe('length', Format(Buffer.Content.Length));
        Observer.Observe('entries', Helper.StoredEntries(Buffer));
        Observer.Expect('length', '194');
        Observer.Expect('entries', '2:a.txt;b.txt;');
        Observer.Verify();
    end;
}

codeunit 59430 "NB SZ Overwrite Direct 59430"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // AL writes through the stream itself (WriteByte) after Position := 0: the probe measured the StreamWriter's write only.
    [Test]
    procedure ZB_Overwrite_AtPosition0_DirectWriteByte()
    var
        Observer: Codeunit "NB SZ Observer";
        Helper: Codeunit "NB SZ Zip Helper";
        Buffer: Record "NB SZ Zip File" temporary;
        Writer: DotNet NbSzStreamWriter;
        Base: DotNet NbSzStream;
        BufferOut: OutStream;
    begin
        Buffer.Content.CreateOutStream(BufferOut);
        Writer := Writer.StreamWriter(BufferOut);
        Writer.Write(PadStr('', 48, 'A'));
        Writer.Flush();
        Base := Writer.BaseStream;
        Base.Position := 0;
        Base.WriteByte(66);
        Observer.Observe('text', Helper.BlobText(Buffer));
        Observer.Verify();
    end;
}
