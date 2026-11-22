// Neighbors of fix-blob-form (issue 8588): the install baseline's bytes form now carries every Blob buffer field.
// The install code of the dependency stores one row per mark a stored row hands on (utf16_binary, render_pending,
// inserted). A look the mark refuses is refused on the thread that captured the baseline, and now also on every copy
// read from its bytes (a worker thread, the cached baseline). Four lane codeunits run the same three mark tests, so that
// some of them run on a worker thread; the neighbor codeunit holds the control rows and the shapes without a mark.
codeunit 61100 "BF Blob Reader"
{
    procedure ReadRow(RowId: Integer; Encoding: TextEncoding): Text
    var
        Row: Record "BF Blob Row";
        InS: InStream;
        Value: Text;
    begin
        Row.Get(RowId);
        Row.CalcFields(Data);
        Row.Data.CreateInStream(InS, Encoding);
        InS.Read(Value);
        exit(Value);
    end;

    procedure ImportOwnBlob(RowId: Integer): Boolean
    var
        Row: Record "BF Blob Row";
        InS: InStream;
    begin
        Row.Get(RowId);
        Row.CalcFields(Data);
        Row.Data.CreateInStream(InS);
        Row.Pic.ImportStream(InS, 'pic');
        exit(Row.Pic.HasValue());
    end;
}

codeunit 61101 "BF Marks Lane 1"
{
    Subtype = Test;

    // utf16_binary: Byte values written through a UTF-16 OutStream, read as Text.
    [Test]
    procedure Utf16Binary_ReadAsText()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('read=%1', Reader.ReadRow(1, TextEncoding::UTF16));
    end;

    // render_pending: a report saved to the Blob in a rendered format, read as Text.
    [Test]
    procedure RenderPending_ReadAsText()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('read=%1', CopyStr(Reader.ReadRow(5, TextEncoding::UTF8), 1, 5));
    end;

    // inserted: Media.ImportStream from a stream on the row's own Blob, which holds what the Insert stored.
    [Test]
    procedure Inserted_MediaImportOwnBlob()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('hasValue=%1', Reader.ImportOwnBlob(4));
    end;
}

codeunit 61102 "BF Marks Lane 2"
{
    Subtype = Test;

    // utf16_binary: Byte values written through a UTF-16 OutStream, read as Text.
    [Test]
    procedure Utf16Binary_ReadAsText()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('read=%1', Reader.ReadRow(1, TextEncoding::UTF16));
    end;

    // render_pending: a report saved to the Blob in a rendered format, read as Text.
    [Test]
    procedure RenderPending_ReadAsText()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('read=%1', CopyStr(Reader.ReadRow(5, TextEncoding::UTF8), 1, 5));
    end;

    // inserted: Media.ImportStream from a stream on the row's own Blob, which holds what the Insert stored.
    [Test]
    procedure Inserted_MediaImportOwnBlob()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('hasValue=%1', Reader.ImportOwnBlob(4));
    end;
}

codeunit 61103 "BF Marks Lane 3"
{
    Subtype = Test;

    // utf16_binary: Byte values written through a UTF-16 OutStream, read as Text.
    [Test]
    procedure Utf16Binary_ReadAsText()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('read=%1', Reader.ReadRow(1, TextEncoding::UTF16));
    end;

    // render_pending: a report saved to the Blob in a rendered format, read as Text.
    [Test]
    procedure RenderPending_ReadAsText()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('read=%1', CopyStr(Reader.ReadRow(5, TextEncoding::UTF8), 1, 5));
    end;

    // inserted: Media.ImportStream from a stream on the row's own Blob, which holds what the Insert stored.
    [Test]
    procedure Inserted_MediaImportOwnBlob()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('hasValue=%1', Reader.ImportOwnBlob(4));
    end;
}

codeunit 61104 "BF Marks Lane 4"
{
    Subtype = Test;

    // utf16_binary: Byte values written through a UTF-16 OutStream, read as Text.
    [Test]
    procedure Utf16Binary_ReadAsText()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('read=%1', Reader.ReadRow(1, TextEncoding::UTF16));
    end;

    // render_pending: a report saved to the Blob in a rendered format, read as Text.
    [Test]
    procedure RenderPending_ReadAsText()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('read=%1', CopyStr(Reader.ReadRow(5, TextEncoding::UTF8), 1, 5));
    end;

    // inserted: Media.ImportStream from a stream on the row's own Blob, which holds what the Insert stored.
    [Test]
    procedure Inserted_MediaImportOwnBlob()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        Error('hasValue=%1', Reader.ImportOwnBlob(4));
    end;
}

codeunit 61105 "BF Blob Neighbors"
{
    Subtype = Test;

    // The control: a Text written through a UTF-8 OutStream reads back.
    [Test]
    procedure Control_Utf8TextRow_ReadAsText()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        if Reader.ReadRow(2, TextEncoding::UTF8) <> 'Plain' then
            Error('read=%1', Reader.ReadRow(2, TextEncoding::UTF8));
    end;

    // A Text, not a Byte, written through a UTF-16 OutStream: no mark, it reads back on every copy.
    [Test]
    procedure Control_Utf16TextRow_ReadAsText()
    var
        Reader: Codeunit "BF Blob Reader";
    begin
        if Reader.ReadRow(3, TextEncoding::UTF16) <> 'Wide' then
            Error('read=%1', Reader.ReadRow(3, TextEncoding::UTF16));
    end;

    // The utf16_binary row, looked at without reading its bytes as Text: HasValue does not read the mark.
    [Test]
    procedure Utf16Binary_HasValue()
    var
        Row: Record "BF Blob Row";
    begin
        Row.Get(1);
        Row.CalcFields(Data);
        if not Row.Data.HasValue() then
            Error('no value');
    end;

    // The utf16_binary shape written by the test itself, not by the install code: no baseline copy is involved.
    [Test]
    procedure Utf16Binary_WrittenInTest_ReadAsText()
    var
        Row: Record "BF Blob Row";
        Reader: Codeunit "BF Blob Reader";
        Out: OutStream;
        B: Byte;
    begin
        Row.Id := 10;
        Row.Data.CreateOutStream(Out, TextEncoding::UTF16);
        B := 65;
        Out.Write(B);
        B := 0;
        Out.Write(B);
        Row.Insert();
        Error('read=%1', Reader.ReadRow(10, TextEncoding::UTF16));
    end;
}
