dotnet
{
    assembly("netstandard")
    {
        type("System.IO.Compression.ZipArchive"; NbZipArchive) { }
        type("System.IO.Compression.ZipArchiveMode"; NbZipMode) { }
        type("System.IO.Compression.ZipArchiveEntry"; NbZipEntry) { }
        type("System.Text.Encoding"; NbEncoding) { }
    }
}

table 50360 "NB Zip File"
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

// Neighbors of al-runner issue 8360: a ZipArchive opened in Update mode over an OutStream that holds a zip, disposed.
codeunit 50361 "NB Zip Update Dispose"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure MakeZip(var ZipFile: Record "NB Zip File" temporary)
    var
        ZipArchive: DotNet NbZipArchive;
        Mode: DotNet NbZipMode;
        ZipEntry: DotNet NbZipEntry;
        EntryOut: OutStream;
        ZipOut: OutStream;
    begin
        ZipFile.Content.CreateOutStream(ZipOut);
        ZipArchive := ZipArchive.ZipArchive(ZipOut, Mode.Create);
        ZipEntry := ZipArchive.CreateEntry('a.txt');
        EntryOut := ZipEntry.Open();
        EntryOut.WriteText('hello');
        ZipArchive.Dispose();
    end;

    // The Blob holds a zip; its OutStream stands at the end (the seed of the bridge).
    local procedure CopyZip(var ZipFile: Record "NB Zip File" temporary; var Buffer: Record "NB Zip File" temporary; var BufferOut: OutStream)
    var
        ZipIn: InStream;
    begin
        MakeZip(ZipFile);
        ZipFile.Content.CreateInStream(ZipIn);
        Buffer.Content.CreateOutStream(BufferOut);
        CopyStream(BufferOut, ZipIn);
    end;

    local procedure AddEntry(ZipArchive: DotNet NbZipArchive; Name: Text)
    var
        ZipEntry: DotNet NbZipEntry;
        EntryOut: OutStream;
    begin
        ZipEntry := ZipArchive.CreateEntry(Name);
        EntryOut := ZipEntry.Open();
        EntryOut.WriteText('world');
    end;

    local procedure EntryCount(var Buffer: Record "NB Zip File" temporary): Integer
    var
        ZipArchive: DotNet NbZipArchive;
        Mode: DotNet NbZipMode;
        BufferIn: InStream;
    begin
        Buffer.Content.CreateInStream(BufferIn);
        ZipArchive := ZipArchive.ZipArchive(BufferIn, Mode.Read);
        exit(ZipArchive.Entries.Count);
    end;

    [Test]
    procedure OwnCodeunit_Update4Arg_AddEntry_Dispose()
    var
        ZipFile: Record "NB Zip File" temporary;
        Buffer: Record "NB Zip File" temporary;
        ZipArchive: DotNet NbZipArchive;
        Mode: DotNet NbZipMode;
        Enc: DotNet NbEncoding;
        BufferOut: OutStream;
    begin
        CopyZip(ZipFile, Buffer, BufferOut);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        if EntryCount(Buffer) <> 2 then
            Error('entries %1', EntryCount(Buffer));
    end;

    [Test]
    procedure OwnCodeunit_Update4Arg_NoChange_Dispose()
    var
        ZipFile: Record "NB Zip File" temporary;
        Buffer: Record "NB Zip File" temporary;
        ZipArchive: DotNet NbZipArchive;
        Mode: DotNet NbZipMode;
        Enc: DotNet NbEncoding;
        BufferOut: OutStream;
    begin
        CopyZip(ZipFile, Buffer, BufferOut);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        ZipArchive.Dispose();
        if EntryCount(Buffer) <> 1 then
            Error('entries %1', EntryCount(Buffer));
    end;

    [Test]
    procedure Update3Arg_AddEntry_Dispose()
    var
        ZipFile: Record "NB Zip File" temporary;
        Buffer: Record "NB Zip File" temporary;
        ZipArchive: DotNet NbZipArchive;
        Mode: DotNet NbZipMode;
        BufferOut: OutStream;
    begin
        CopyZip(ZipFile, Buffer, BufferOut);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false);
        AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        if EntryCount(Buffer) <> 2 then
            Error('entries %1', EntryCount(Buffer));
    end;

    [Test]
    procedure Update2Arg_AddEntry_Dispose()
    var
        ZipFile: Record "NB Zip File" temporary;
        Buffer: Record "NB Zip File" temporary;
        ZipArchive: DotNet NbZipArchive;
        Mode: DotNet NbZipMode;
        BufferOut: OutStream;
    begin
        CopyZip(ZipFile, Buffer, BufferOut);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update);
        AddEntry(ZipArchive, 'b.txt');
        ZipArchive.Dispose();
        if EntryCount(Buffer) <> 2 then
            Error('entries %1', EntryCount(Buffer));
    end;

    [Test]
    procedure Update4Arg_AlWriteBeforeDispose()
    var
        ZipFile: Record "NB Zip File" temporary;
        Buffer: Record "NB Zip File" temporary;
        ZipArchive: DotNet NbZipArchive;
        Mode: DotNet NbZipMode;
        Enc: DotNet NbEncoding;
        BufferOut: OutStream;
    begin
        CopyZip(ZipFile, Buffer, BufferOut);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Update, false, Enc.UTF8());
        AddEntry(ZipArchive, 'b.txt');
        BufferOut.WriteText('tail');
        ZipArchive.Dispose();
        if EntryCount(Buffer) <> 2 then
            Error('entries %1', EntryCount(Buffer));
    end;

    [Test]
    procedure Read_Dispose_KeepsTheZip()
    var
        ZipFile: Record "NB Zip File" temporary;
        Buffer: Record "NB Zip File" temporary;
        ZipArchive: DotNet NbZipArchive;
        Mode: DotNet NbZipMode;
        BufferOut: OutStream;
    begin
        CopyZip(ZipFile, Buffer, BufferOut);
        ZipArchive := ZipArchive.ZipArchive(BufferOut, Mode.Read);
        ZipArchive.Dispose();
        if EntryCount(Buffer) <> 1 then
            Error('entries %1', EntryCount(Buffer));
    end;
}
