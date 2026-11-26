// Files a report save wrote in another session or process (the fixtures under files/), read here: this session holds no
// mark for them. One test per read path, per neighbor of the new mark (Session::outside_output_mark)
// and per new refusal; one codeunit per test, since a refusal ends the rest of its codeunit.
table 50980 "UO Buffer"
{
    TableType = Temporary;
    fields
    {
        field(1; PK; Integer) { }
        field(2; Content; Blob) { }
    }
    keys { key(PK; PK) { } }
}

codeunit 50981 "UO Read"
{
    procedure ReadAll(Name: Text; Enc: TextEncoding; var Count: Integer) T: Text
    var
        F: File;
        InS: InStream;
    begin
        if not F.Open(Name, Enc) then
            Error('no file %1', Name);
        F.CreateInStream(InS);
        Count := InS.Read(T);
        F.Close();
    end;

    procedure ReadNoCount(Name: Text; Enc: TextEncoding) T: Text
    var
        F: File;
        InS: InStream;
    begin
        if not F.Open(Name, Enc) then
            Error('no file %1', Name);
        F.CreateInStream(InS);
        InS.Read(T);
        F.Close();
    end;

    procedure FirstByte(Name: Text) B: Byte
    var
        F: File;
        InS: InStream;
    begin
        F.Open(Name);
        F.CreateInStream(InS);
        InS.Read(B);
        F.Close();
    end;
}

codeunit 50982 "UO Html_Read_StrLen"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // 8703: the placeholder Html, read as text.
    [Test]
    procedure Html_Read_StrLen()
    var
        T: Text;
    begin
        T := R.ReadNoCount('placeholder.html', TextEncoding::UTF8);
        Error('len %1: %2', StrLen(T), T);
    end;
}

codeunit 50983 "UO Html_StrPos_EmptyBody"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Html_StrPos_EmptyBody()
    begin
        if StrPos(R.ReadNoCount('placeholder.html', TextEncoding::UTF8), '<body></body>') = 0 then
            Error('body has content');
    end;
}

codeunit 50984 "UO Html_TrailingByte_Read"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // The whole placeholder with a byte after it.
    [Test]
    procedure Html_TrailingByte_Read()
    var
        T: Text;
    begin
        T := R.ReadNoCount('placeholder-plus.html', TextEncoding::UTF8);
        Error('len %1', StrLen(T));
    end;
}

codeunit 50985 "UO Html_Header_Read"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // A partial copy: the placeholder's header, then other bytes.
    [Test]
    procedure Html_Header_Read()
    var
        T: Text;
    begin
        T := R.ReadNoCount('placeholder-header.html', TextEncoding::UTF8);
        Error('len %1', StrLen(T));
    end;
}

codeunit 50986 "UO HtmlHead_Read"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // Shorter than the header: not told apart from other HTML, main's read stays.
    [Test]
    procedure HtmlHead_Read()
    var
        T: Text;
    begin
        T := R.ReadNoCount('placeholder-head.html', TextEncoding::UTF8);
        Error('len %1', StrLen(T));
    end;
}

codeunit 50987 "UO Html_OpenOnly"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // Open and close without a read: no look at the content.
    [Test]
    procedure Html_OpenOnly()
    var
        F: File;
    begin
        F.Open('placeholder.html');
        F.Close();
    end;
}

codeunit 50988 "UO Html_Len_NotZero"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // The size compared with 0 answers as a placeholder's.
    [Test]
    procedure Html_Len_NotZero()
    var
        F: File;
    begin
        F.Open('placeholder.html');
        if F.Len <= 0 then
            Error('empty');
        F.Close();
    end;
}

codeunit 50989 "UO Html_Len_Value"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // The size itself is not BC's.
    [Test]
    procedure Html_Len_Value()
    var
        F: File;
    begin
        F.Open('placeholder.html');
        Error('len %1', F.Len);
    end;
}

codeunit 50990 "UO Html_Exists"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Html_Exists()
    begin
        if not File.Exists('placeholder.html') then
            Error('missing');
    end;
}

codeunit 50991 "UO Html_FileCopy"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // A byte copy looks at no byte.
    [Test]
    procedure Html_FileCopy()
    begin
        File.Copy('placeholder.html', 'copy-of-placeholder.html');
        File.Erase('copy-of-placeholder.html');
    end;
}

codeunit 50992 "UO Html_CopyThenReadCopy"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // The copy keeps the bytes, so its read is refused as well.
    [Test]
    procedure Html_CopyThenReadCopy()
    var
        T: Text;
    begin
        File.Copy('placeholder.html', 'copy-to-read.html');
        T := R.ReadNoCount('copy-to-read.html', TextEncoding::UTF8);
        Error('len %1', StrLen(T));
    end;
}

codeunit 50993 "UO Html_RenameThenRead"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Html_RenameThenRead()
    var
        T: Text;
    begin
        File.Rename('placeholder-rename.html', 'renamed.html');
        T := R.ReadNoCount('renamed.html', TextEncoding::UTF8);
        Error('len %1', StrLen(T));
    end;
}

codeunit 50994 "UO Html_BlobImport"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Html_BlobImport()
    var
        Buffer: Record "UO Buffer";
    begin
        Buffer.Content.Import('placeholder.html');
        Error('length %1', Buffer.Content.Length);
    end;
}

codeunit 50995 "UO Html_CopyStream_Length"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // A stream copy into a blob carries the mark; the blob's size is a placeholder's.
    [Test]
    procedure Html_CopyStream_Length()
    var
        Buffer: Record "UO Buffer";
        F: File;
        InS: InStream;
        OutS: OutStream;
    begin
        F.Open('placeholder.html');
        F.CreateInStream(InS);
        Buffer.Content.CreateOutStream(OutS);
        CopyStream(OutS, InS);
        Error('length %1', Buffer.Content.Length);
    end;
}

codeunit 50996 "UO Pdf_ReadText5"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // The pinned start of BC's PDF answers.
    [Test]
    procedure Pdf_ReadText5()
    var
        F: File;
        InS: InStream;
        T: Text;
    begin
        F.Open('placeholder.pdf', TextEncoding::Windows);
        F.CreateInStream(InS);
        InS.ReadText(T, 5);
        F.Close();
        Error('head %1', T);
    end;
}

codeunit 50997 "UO Pdf_ReadText8"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // Past the pinned start: the version is the placeholder's.
    [Test]
    procedure Pdf_ReadText8()
    var
        F: File;
        InS: InStream;
        T: Text;
    begin
        F.Open('placeholder.pdf', TextEncoding::Windows);
        F.CreateInStream(InS);
        InS.ReadText(T, 8);
        Error('head %1', T);
    end;
}

codeunit 50998 "UO Pdf_FirstByte"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Pdf_FirstByte()
    begin
        Error('byte %1', R.FirstByte('placeholder.pdf'));
    end;
}

codeunit 50999 "UO Word_FirstByte"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Word_FirstByte()
    begin
        Error('byte %1', R.FirstByte('placeholder.docx'));
    end;
}

codeunit 51000 "UO Mark_ReadCount"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // 8702: the rendered-content mark's bytes in a file.
    [Test]
    procedure Mark_ReadCount()
    var
        N: Integer;
    begin
        R.ReadAll('marked-utf8.txt', TextEncoding::UTF8, N);
        Error('count %1', N);
    end;
}

codeunit 51001 "UO Mark_NotEmpty"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Mark_NotEmpty()
    begin
        if R.ReadNoCount('marked-utf8.txt', TextEncoding::UTF8) = '' then
            Error('empty');
    end;
}

codeunit 51002 "UO Mark_Windows_StrLen"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Mark_Windows_StrLen()
    begin
        Error('len %1', StrLen(R.ReadNoCount('marked-utf8.txt', TextEncoding::Windows)));
    end;
}

codeunit 51003 "UO Mark_Utf16_StrLen"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Mark_Utf16_StrLen()
    begin
        Error('len %1', StrLen(R.ReadNoCount('marked-utf16.txt', TextEncoding::UTF16)));
    end;
}

codeunit 51004 "UO Mark_FirstByte"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Mark_FirstByte()
    begin
        Error('byte %1', R.FirstByte('marked-utf8.txt'));
    end;
}

codeunit 51005 "UO Mark_OpenOnly"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Mark_OpenOnly()
    var
        F: File;
    begin
        F.Open('marked-utf8.txt');
        F.Close();
    end;
}

codeunit 51006 "UO OtherMark_StrLen"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // Another text mark's bytes (U+FDD0): no render behind it, main's read stays.
    [Test]
    procedure OtherMark_StrLen()
    begin
        Error('len %1', StrLen(R.ReadNoCount('other-mark.txt', TextEncoding::UTF8)));
    end;
}

codeunit 51007 "UO Plain_Read"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    [Test]
    procedure Plain_Read()
    begin
        if R.ReadNoCount('plain.txt', TextEncoding::UTF8) <> 'plain text' then
            Error('wrong text');
    end;
}
