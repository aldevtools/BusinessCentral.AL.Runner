dotnet
{
    assembly("netstandard")
    {
        type("System.IO.Compression.ZipArchive"; NbR1ZipArchive) { }
        type("System.IO.Compression.ZipArchiveMode"; NbR1ZipMode) { }
        type("System.IO.Compression.ZipArchiveEntry"; NbR1ZipEntry) { }
        type("System.Text.Encoding"; NbR1Encoding) { }
    }
}

table 50365 "NB R1 Zip File"
{
    DataClassification = SystemMetadata;
    TableType = Temporary;

    fields
    {
        field(1; "No."; Integer) { }
        field(2; Content; Blob) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 50366 "NB R1 Stored Zip File"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Integer) { }
        field(2; Content; Blob) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// Review round 1 neighbors of al-runner issue 8360: AL changes the Blob between the Update constructor and Dispose, two Update
// archives over one OutStream, and the Blob field of a stored table. One test per codeunit: a refusal mid-run stops the codeunit.
codeunit 50367 "NB R1 Zip Helper"
{
    procedure MakeZip(var ZipFile: Record "NB R1 Zip File" temporary; Name: Text)
    var
        ZipArchive: DotNet NbR1ZipArchive;
        Mode: DotNet NbR1ZipMode;
        ZipEntry: DotNet NbR1ZipEntry;
        ZipOut: OutStream;
    begin
        ZipFile.Content.CreateOutStream(ZipOut);
        ZipArchive := ZipArchive.ZipArchive(ZipOut, Mode.Create);
        ZipEntry := ZipArchive.CreateEntry(Name);
        ZipArchive.Dispose();
    end;

    procedure CopyZip(var ZipFile: Record "NB R1 Zip File" temporary; var Buffer: Record "NB R1 Zip File" temporary; var BufferOut: OutStream)
    var
        ZipIn: InStream;
    begin
        MakeZip(ZipFile, 'a.txt');
        ZipFile.Content.CreateInStream(ZipIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, ZipIn);
    end;

    procedure AddEntry(ZipArchive: DotNet NbR1ZipArchive; Name: Text)
    var
        ZipEntry: DotNet NbR1ZipEntry;
    begin
        ZipEntry := ZipArchive.CreateEntry(Name);
    end;

    procedure EntryCount(var InS: InStream): Integer
    var
        ZipArchive: DotNet NbR1ZipArchive;
        Mode: DotNet NbR1ZipMode;
    begin
        ZipArchive := ZipArchive.ZipArchive(InS, Mode.Read);
        exit(ZipArchive.Entries.Count);
    end;

    procedure BufferEntries(var Buffer: Record "NB R1 Zip File" temporary): Integer
    var
        BufferIn: InStream;
    begin
        Buffer.Content.CreateInStream(BufferIn);
        exit(EntryCount(BufferIn));
    end;
}

codeunit 50368 "NB R1 Clear Rewrite"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Update4Arg_ClearAndRewriteSameLength_Dispose()
    var
        Helper: Codeunit "NB R1 Zip Helper";
        ZipFile: Record "NB R1 Zip File" temporary;
        Other: Record "NB R1 Zip File" temporary;
        Buffer: Record "NB R1 Zip File" temporary;
        ZipArchive: DotNet NbR1ZipArchive;
        Mode: DotNet NbR1ZipMode;
        Enc: DotNet NbR1Encoding;
        BufferOut: OutStream;
        NewOut: OutStream;
        OtherIn: InStream;
    begin
        Helper.CopyZip(ZipFile, Buffer, BufferOut);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        // A zip of the same length with another entry name, written by AL over the field.
        Helper.MakeZip(Other, 'z.txt');
        Clear(Buffer.Content);
        Buffer.Content.CreateOutStream(NewOut);
        Other.Content.CreateInStream(OtherIn);
        CopyStream(NewOut, OtherIn);
        ZipArchive.Dispose();
        if Helper.BufferEntries(Buffer) <> 1 then
            Error('entries %1', Helper.BufferEntries(Buffer));
    end;
}

codeunit 50369 "NB R1 Clear Only"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Update4Arg_ClearOnly_Dispose()
    var
        Helper: Codeunit "NB R1 Zip Helper";
        ZipFile: Record "NB R1 Zip File" temporary;
        Buffer: Record "NB R1 Zip File" temporary;
        ZipArchive: DotNet NbR1ZipArchive;
        Mode: DotNet NbR1ZipMode;
        Enc: DotNet NbR1Encoding;
        BufferOut: OutStream;
    begin
        Helper.CopyZip(ZipFile, Buffer, BufferOut);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        Clear(Buffer.Content);
        ZipArchive.Dispose();
        if Buffer.Content.HasValue() then
            Error('the field has a value');
    end;
}

codeunit 50370 "NB R1 Two Archives"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TwoUpdateArchives_SameOutStream_Dispose()
    var
        Helper: Codeunit "NB R1 Zip Helper";
        ZipFile: Record "NB R1 Zip File" temporary;
        Buffer: Record "NB R1 Zip File" temporary;
        First: DotNet NbR1ZipArchive;
        Second: DotNet NbR1ZipArchive;
        Mode: DotNet NbR1ZipMode;
        Enc: DotNet NbR1Encoding;
        BufferOut: OutStream;
    begin
        Helper.CopyZip(ZipFile, Buffer, BufferOut);
        First := First.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Second := Second.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(First, 'b.txt');
        Helper.AddEntry(Second, 'c.txt');
        First.Dispose();
        Second.Dispose();
        if Helper.BufferEntries(Buffer) <> 2 then
            Error('entries %1', Helper.BufferEntries(Buffer));
    end;
}

codeunit 50371 "NB R1 Stored Table"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure StoredTable_Update4Arg_AddEntry_Dispose()
    var
        Helper: Codeunit "NB R1 Zip Helper";
        ZipFile: Record "NB R1 Zip File" temporary;
        Stored: Record "NB R1 Stored Zip File";
        ZipArchive: DotNet NbR1ZipArchive;
        Mode: DotNet NbR1ZipMode;
        Enc: DotNet NbR1Encoding;
        ZipIn: InStream;
        StoredOut: OutStream;
        StoredIn: InStream;
    begin
        Helper.MakeZip(ZipFile, 'a.txt');
        ZipFile.Content.CreateInStream(ZipIn);
        Stored."No." := 1;
        Stored.Content.CreateOutStream(StoredOut);
        CopyStream(StoredOut, ZipIn);
        Stored.Insert();
        Stored.Get(1);
        Stored.CalcFields(Content);
        Stored.Content.CreateOutStream(StoredOut);
        ZipArchive := ZipArchive.ZipArchive(StoredOut, Mode.Update, false, Enc.UTF8());
        Helper.AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        Stored.Modify();
        Stored.Get(1);
        Stored.CalcFields(Content);
        Stored.Content.CreateInStream(StoredIn);
        if Helper.EntryCount(StoredIn) <> 2 then
            Error('entries %1', Helper.EntryCount(StoredIn));
    end;
}

// The control: the helpers give a zip with one entry in the Blob, read back without an Update archive.
codeunit 50372 "NB R1 Control"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Control_CopiedZip_HasOneEntry()
    var
        Helper: Codeunit "NB R1 Zip Helper";
        ZipFile: Record "NB R1 Zip File" temporary;
        Buffer: Record "NB R1 Zip File" temporary;
        BufferOut: OutStream;
    begin
        Helper.CopyZip(ZipFile, Buffer, BufferOut);
        if Helper.BufferEntries(Buffer) <> 1 then
            Error('entries %1', Helper.BufferEntries(Buffer));
    end;
}
