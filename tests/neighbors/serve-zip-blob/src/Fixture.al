dotnet
{
    assembly("netstandard")
    {
        type("System.IO.Compression.ZipArchive"; NbSzZipArchive) { }
        type("System.IO.Compression.ZipArchiveMode"; NbSzZipMode) { }
        type("System.IO.Compression.ZipArchiveEntry"; NbSzZipEntry) { }
        type("System.Text.Encoding"; NbSzEncoding) { }
        type("System.IO.StreamWriter"; NbSzStreamWriter) { }
        type("System.IO.Stream"; NbSzStream) { }
    }
}

table 59400 "NB SZ Zip File"
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

table 59401 "NB SZ Stored Zip File"
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

codeunit 59402 "NB SZ Zip Helper"
{
    procedure MakeZip(var ZipFile: Record "NB SZ Zip File" temporary; Name: Text)
    var
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        ZipEntry: DotNet NbSzZipEntry;
        ZipOut: OutStream;
    begin
        ZipFile.Content.CreateOutStream(ZipOut);
        ZipArchive := ZipArchive.ZipArchive(ZipOut, Mode.Create);
        ZipEntry := ZipArchive.CreateEntry(Name);
        ZipArchive.Dispose();
    end;

    procedure SeedIn(var ZipFile: Record "NB SZ Zip File" temporary; Name: Text; var ZipIn: InStream)
    begin
        MakeZip(ZipFile, Name);
        ZipFile.Content.CreateInStream(ZipIn);
    end;

    procedure AddEntry(ZipArchive: DotNet NbSzZipArchive; Name: Text)
    var
        ZipEntry: DotNet NbSzZipEntry;
    begin
        ZipEntry := ZipArchive.CreateEntry(Name);
    end;

    // "<count>:<name>;<name>;" of the zip read from InS in Read mode.
    procedure Entries(var InS: InStream): Text
    var
        ZipArchive: DotNet NbSzZipArchive;
        Mode: DotNet NbSzZipMode;
        ZipEntry: DotNet NbSzZipEntry;
        Names: Text;
    begin
        ZipArchive := ZipArchive.ZipArchive(InS, Mode.Read);
        foreach ZipEntry in ZipArchive.Entries do
            Names += ZipEntry.FullName + ';';
        exit(Format(ZipArchive.Entries.Count) + ':' + Names);
    end;

    procedure BufferEntries(var ZipFile: Record "NB SZ Zip File" temporary): Text
    var
        InS: InStream;
    begin
        ZipFile.Content.CreateInStream(InS);
        exit(Entries(InS));
    end;

    procedure StoredEntries(var ZipFile: Record "NB SZ Stored Zip File"): Text
    var
        InS: InStream;
    begin
        ZipFile.Content.CreateInStream(InS);
        exit(Entries(InS));
    end;

    // The last three bytes of the Blob as decimal numbers, "b1,b2,b3".
    procedure LastBytes(var ZipFile: Record "NB SZ Zip File" temporary): Text
    var
        InS: InStream;
        B: Byte;
        B1: Integer;
        B2: Integer;
        B3: Integer;
    begin
        ZipFile.Content.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.Read(B);
            B1 := B2;
            B2 := B3;
            B3 := B;
        end;
        exit(StrSubstNo('%1,%2,%3', B1, B2, B3));
    end;

    procedure BlobText(var ZipFile: Record "NB SZ Zip File" temporary): Text
    var
        InS: InStream;
        T: Text;
    begin
        ZipFile.Content.CreateInStream(InS);
        InS.ReadText(T);
        exit(T);
    end;

    procedure StoredText(var ZipFile: Record "NB SZ Stored Zip File"): Text
    var
        InS: InStream;
        T: Text;
    begin
        ZipFile.Content.CreateInStream(InS);
        InS.ReadText(T);
        exit(T);
    end;
}
codeunit 59403 "NB SZ Observer"
{
    var
        Seen: Dictionary of [Text, Text];
        Wanted: Dictionary of [Text, Text];

    procedure Observe(Name: Text; Value: Text)
    begin
        Seen.Set(Name, Value);
    end;

    procedure Expect(Name: Text; Value: Text)
    begin
        Wanted.Set(Name, Value);
    end;

    procedure Verify()
    var
        Name: Text;
        Got: Text;
    begin
        foreach Name in Wanted.Keys() do begin
            if not Seen.Get(Name, Got) then
                Error('%1: not observed', Name);
            if Got <> Wanted.Get(Name) then
                Error('%1: expected %2, got %3', Name, Wanted.Get(Name), Got);
        end;
    end;
}
