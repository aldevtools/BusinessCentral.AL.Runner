// Review round 1. A binary file whose bytes hold the rendered-text mark followed by a byte that is no render number
// (files/binary-mark-bytes.png): no rendered text, main's answer on every read. A placeholder opened in write mode and
// closed with nothing written: its bytes are as before, so a copy, a rename or a delete answers as on main. The new
// Media.ImportFile refusal.
table 51010 "UO Media Buffer"
{
    TableType = Temporary;
    fields
    {
        field(1; PK; Integer) { }
        field(2; Pic; Media) { }
    }
    keys { key(PK; PK) { } }
}

codeunit 51011 "UO Bin_BlobImport"
{
    Subtype = Test;

    [Test]
    procedure Bin_BlobImport()
    var
        Buffer: Record "UO Buffer";
    begin
        Buffer.Content.Import('binary-mark-bytes.png');
        Error('length %1', Buffer.Content.Length);
    end;
}

codeunit 51012 "UO Bin_MediaImportFile"
{
    Subtype = Test;

    [Test]
    procedure Bin_MediaImportFile()
    var
        Buffer: Record "UO Media Buffer";
    begin
        Buffer.Pic.ImportFile('binary-mark-bytes.png', 'bin');
        Error('has %1', Buffer.Pic.HasValue());
    end;
}

codeunit 51013 "UO Bin_CopyStream_Length"
{
    Subtype = Test;

    [Test]
    procedure Bin_CopyStream_Length()
    var
        Buffer: Record "UO Buffer";
        F: File;
        InS: InStream;
        OutS: OutStream;
    begin
        F.Open('binary-mark-bytes.png');
        F.CreateInStream(InS);
        Buffer.Content.CreateOutStream(OutS);
        CopyStream(OutS, InS);
        Error('length %1', Buffer.Content.Length);
    end;
}

codeunit 51014 "UO Word_MediaImportFile"
{
    Subtype = Test;

    // New refusal: Media.ImportFile reads the content of a placeholder Word document another session wrote.
    [Test]
    procedure Word_MediaImportFile()
    var
        Buffer: Record "UO Media Buffer";
    begin
        Buffer.Pic.ImportFile('placeholder.docx', 'doc');
        Error('has %1', Buffer.Pic.HasValue());
    end;
}

codeunit 51015 "UO WriteClose"
{
    procedure OpenClose(Name: Text)
    var
        F: File;
    begin
        F.WriteMode(true);
        F.Open(Name);
        F.Close();
    end;
}

codeunit 51016 "UO WriteClose_Copy"
{
    Subtype = Test;

    [Test]
    procedure WriteClose_Copy()
    var
        W: Codeunit "UO WriteClose";
    begin
        W.OpenClose('placeholder-wc-copy.html');
        File.Copy('placeholder-wc-copy.html', 'wc-copied.html');
        Error('exists %1', File.Exists('wc-copied.html'));
    end;
}

codeunit 51017 "UO WriteClose_Rename"
{
    Subtype = Test;

    [Test]
    procedure WriteClose_Rename()
    var
        W: Codeunit "UO WriteClose";
    begin
        W.OpenClose('placeholder-wc-rename.html');
        File.Rename('placeholder-wc-rename.html', 'wc-renamed.html');
        Error('exists %1 %2', File.Exists('wc-renamed.html'), File.Exists('placeholder-wc-rename.html'));
    end;
}

dotnet
{
    assembly("mscorlib")
    {
        type("System.IO.File"; UOIOFile) { }
    }
}

codeunit 51018 "UO WriteClose_Delete"
{
    Subtype = Test;

    // The DotNet delete File Management.DeleteServerFile makes. Outside the company build's file system the policy
    // refuses it on main and head alike; after the fix the close stores no mark the delete could refuse on.
    [Test]
    procedure WriteClose_Delete()
    var
        W: Codeunit "UO WriteClose";
        IOFile: DotNet UOIOFile;
    begin
        W.OpenClose('placeholder-wc-delete.html');
        IOFile.Delete('placeholder-wc-delete.html');
        Error('exists %1', File.Exists('placeholder-wc-delete.html'));
    end;
}

codeunit 51019 "UO WriteText_Close_Read"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // AL wrote into the open placeholder: the file keeps the mark, its read is refused.
    [Test]
    procedure WriteText_Close_Read()
    var
        F: File;
        OutS: OutStream;
    begin
        F.WriteMode(true);
        F.Open('placeholder-wc-write.html');
        F.CreateOutStream(OutS);
        OutS.WriteText('x');
        F.Close();
        Error('len %1', StrLen(R.ReadNoCount('placeholder-wc-write.html', TextEncoding::UTF8)));
    end;
}
