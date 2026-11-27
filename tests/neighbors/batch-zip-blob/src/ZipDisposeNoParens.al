dotnet
{
    assembly("netstandard")
    {
        type("System.IO.Compression.ZipArchive"; NbZbZipArchive) { }
        type("System.IO.Compression.ZipArchiveMode"; NbZbZipMode) { }
        type("System.IO.Compression.ZipArchiveEntry"; NbZbZipEntry) { }
        type("System.Text.Encoding"; NbZbEncoding) { }
    }
}

table 59291 "NB ZB Zip File"
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

// Neighbors of al-runner issue 9291: `ZipArchive.Dispose;` without parentheses (AL lowers it to a property read) takes the
// path of `Dispose()`. One test per codeunit: a refusal mid-run stops the codeunit.
codeunit 59292 "NB ZB Helper"
{
    procedure SeedUpdateArchive(var Buffer: Record "NB ZB Zip File" temporary; var BufferOut: OutStream; var ZipArchive: DotNet NbZbZipArchive)
    var
        ZipFile: Record "NB ZB Zip File" temporary;
        Mode: DotNet NbZbZipMode;
        Enc: DotNet NbZbEncoding;
        ZipEntry: DotNet NbZbZipEntry;
        Maker: DotNet NbZbZipArchive;
        ZipOut: OutStream;
        ZipIn: InStream;
    begin
        ZipFile.Content.CreateOutStream(ZipOut);
        Maker := Maker.ZipArchive(ZipOut, Mode.Create);
        ZipEntry := Maker.CreateEntry('a.txt');
        Maker.Dispose();
        ZipFile.Content.CreateInStream(ZipIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, ZipIn);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        ZipEntry := ZipArchive.CreateEntry('b.txt');
    end;

    procedure BufferEntries(var Buffer: Record "NB ZB Zip File" temporary): Integer
    var
        ZipArchive: DotNet NbZbZipArchive;
        Mode: DotNet NbZbZipMode;
        BufferIn: InStream;
    begin
        Buffer.Content.CreateInStream(BufferIn);
        ZipArchive := ZipArchive.ZipArchive(BufferIn, Mode.Read);
        exit(ZipArchive.Entries.Count);
    end;
}

codeunit 59293 "NB ZB Dispose Parens"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // The control: Dispose() with parentheses keeps both entries (main and head).
    [Test]
    procedure Update4Arg_DisposeWithParens()
    var
        Helper: Codeunit "NB ZB Helper";
        Buffer: Record "NB ZB Zip File" temporary;
        ZipArchive: DotNet NbZbZipArchive;
        BufferOut: OutStream;
    begin
        Helper.SeedUpdateArchive(Buffer, BufferOut, ZipArchive);
        ZipArchive.Dispose();
        if Helper.BufferEntries(Buffer) <> 2 then
            Error('entries %1', Helper.BufferEntries(Buffer));
    end;
}

codeunit 59294 "NB ZB Dispose No Parens"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Update4Arg_DisposeWithoutParens()
    var
        Helper: Codeunit "NB ZB Helper";
        Buffer: Record "NB ZB Zip File" temporary;
        ZipArchive: DotNet NbZbZipArchive;
        BufferOut: OutStream;
    begin
        Helper.SeedUpdateArchive(Buffer, BufferOut, ZipArchive);
        ZipArchive.Dispose;
        if Helper.BufferEntries(Buffer) <> 2 then
            Error('entries %1', Helper.BufferEntries(Buffer));
    end;
}

codeunit 59295 "NB ZB No Parens After Write"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // AL wrote into the Blob after the Update constructor: the rewrite stays refused, with or without parentheses.
    [Test]
    procedure Update4Arg_AlWrite_DisposeWithoutParens()
    var
        Helper: Codeunit "NB ZB Helper";
        Buffer: Record "NB ZB Zip File" temporary;
        ZipArchive: DotNet NbZbZipArchive;
        BufferOut: OutStream;
    begin
        Helper.SeedUpdateArchive(Buffer, BufferOut, ZipArchive);
        BufferOut.WriteText('x');
        ZipArchive.Dispose;
        if Helper.BufferEntries(Buffer) <> 2 then
            Error('entries %1', Helper.BufferEntries(Buffer));
    end;
}
