// Files a report save wrote in another session or process (the fixtures under files/), read here: this session holds no
// mark for them. One test per read path and per neighbor of the new refusal in Session::fs_read.
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

codeunit 50982 "UO Tests"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // al-runner issue 8703: the placeholder Html, read as text.
    [Test]
    procedure Html_Read_StrLen()
    var
        T: Text;
    begin
        T := R.ReadNoCount('placeholder.html', TextEncoding::UTF8);
        Error('len %1: %2', StrLen(T), T);
    end;

    [Test]
    procedure Html_StrPos_EmptyBody()
    begin
        if StrPos(R.ReadNoCount('placeholder.html', TextEncoding::UTF8), '<body></body>') = 0 then
            Error('body has content');
    end;

    [Test]
    procedure Html_TrailingByte_Read()
    var
        T: Text;
    begin
        T := R.ReadNoCount('placeholder-plus.html', TextEncoding::UTF8);
        Error('len %1', StrLen(T));
    end;

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

    [Test]
    procedure Pdf_FirstByte()
    begin
        Error('byte %1', R.FirstByte('placeholder.pdf'));
    end;

    [Test]
    procedure Html_BlobImport()
    var
        Buffer: Record "UO Buffer";
    begin
        Buffer.Content.Import('placeholder.html');
        Error('length %1', Buffer.Content.Length);
    end;

    [Test]
    procedure Html_FileCopy()
    begin
        File.Copy('placeholder.html', 'copy-of-placeholder.html');
        File.Erase('copy-of-placeholder.html');
        Error('copied');
    end;

    // al-runner issue 8702: the rendered-content mark's bytes in a file.
    [Test]
    procedure Mark_ReadCount()
    var
        N: Integer;
    begin
        R.ReadAll('marked-utf8.txt', TextEncoding::UTF8, N);
        Error('count %1', N);
    end;

    [Test]
    procedure Mark_NotEmpty()
    begin
        if R.ReadNoCount('marked-utf8.txt', TextEncoding::UTF8) = '' then
            Error('empty');
    end;

    [Test]
    procedure Mark_Windows_StrLen()
    begin
        Error('len %1', StrLen(R.ReadNoCount('marked-utf8.txt', TextEncoding::Windows)));
    end;

    [Test]
    procedure Mark_Utf16_StrLen()
    begin
        Error('len %1', StrLen(R.ReadNoCount('marked-utf16.txt', TextEncoding::UTF16)));
    end;

    [Test]
    procedure Mark_FirstByte()
    begin
        Error('byte %1', R.FirstByte('marked-utf8.txt'));
    end;

}

codeunit 50983 "UO Word Tests"
{
    Subtype = Test;

    [Test]
    procedure Word_FirstByte()
    var
        R: Codeunit "UO Read";
    begin
        Error('byte %1', R.FirstByte('placeholder.docx'));
    end;
}

// The neighbors that keep main's answer, apart from the refused tests (a refusal ends the codeunit).
codeunit 50984 "UO Keep Tests"
{
    Subtype = Test;

    var
        R: Codeunit "UO Read";

    // The start of the placeholder only: not its whole bytes, so main's read stays.
    [Test]
    procedure HtmlHead_Read()
    var
        T: Text;
    begin
        T := R.ReadNoCount('placeholder-head.html', TextEncoding::UTF8);
        Error('len %1', StrLen(T));
    end;

    [Test]
    procedure Html_Exists()
    begin
        if not File.Exists('placeholder.html') then
            Error('missing');
    end;

    // Another text mark's bytes (U+FDD0): no render behind it, main's read stays.
    [Test]
    procedure OtherMark_StrLen()
    begin
        Error('len %1', StrLen(R.ReadNoCount('other-mark.txt', TextEncoding::UTF8)));
    end;

    [Test]
    procedure Plain_Read()
    begin
        if R.ReadNoCount('plain.txt', TextEncoding::UTF8) <> 'plain text' then
            Error('wrong text');
    end;
}
