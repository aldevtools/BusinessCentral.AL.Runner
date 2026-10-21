//! An XML report dataset with a Blob column (AL Runner issue 5582), pinned by report/lt-bc-xml-blob/BcXmlBlob.al: an empty Blob is
//! an empty element, content is its Base64 text, with or without CalcFields; an empty Media or MediaSet is an empty element.

const SRC: &str = r#"
table 50430 "BX Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
        field(3; Pic; Media) { }
        field(4; Pics; MediaSet) { }
        field(5; Note; Text[30]) { }
    }
    keys { key(PK; Id) { } }
}
table 50431 "BX Out"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { } }
}
report 50432 "BX Columns"
{
    dataset
    {
        dataitem(Row; "BX Row")
        {
            column(Id; Row.Id) { }
            column(BlobCol; Row.Data) { }
            column(MediaCol; Row.Pic) { }
            column(MediaSetCol; Row.Pics) { }
            column(NoteCol; Row.Note) { }
        }
    }
}
report 50433 "BX CalcFields"
{
    dataset
    {
        dataitem(Row; "BX Row")
        {
            column(Id; Row.Id) { }
            column(BlobCol; Row.Data) { }
            trigger OnAfterGetRecord()
            begin
                Row.CalcFields(Data);
            end;
        }
    }
}
codeunit 50434 C
{
    local procedure Columns(ReportId: Integer): Text
    var
        O: Record "BX Out" temporary;
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        R: Text;
    begin
        O.Data.CreateOutStream(OutS);
        if not Report.SaveAs(ReportId, '', ReportFormat::Xml, OutS) then
            exit('not saved');
        O.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            if Line.Contains('<Column ') then
                R += Line.Trim() + '\';
        end;
        exit(R);
    end;

    local procedure Setup()
    var
        Row: Record "BX Row";
        OutS: OutStream;
    begin
        Row.DeleteAll();
        Row.Init(); Row.Id := 1; Row.Note := 'empty'; Row.Insert();
        Row.Init(); Row.Id := 2; Row.Note := 'text';
        Row.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('Hello BC');
        Row.Insert();
    end;

    procedure EmptyAndText(): Text
    begin
        Setup();
        exit(Columns(Report::"BX Columns"));
    end;

    procedure CalcFieldsInTrigger(): Text
    begin
        Setup();
        exit(Columns(Report::"BX CalcFields"));
    end;

    procedure BinaryBytes(): Text
    var
        Row: Record "BX Row";
        OutS: OutStream;
        B: Byte;
    begin
        Row.DeleteAll();
        Row.Init(); Row.Id := 3; Row.Note := 'bytes';
        Row.Data.CreateOutStream(OutS);
        B := 0; OutS.Write(B);
        B := 255; OutS.Write(B);
        B := 65; OutS.Write(B);
        Row.Insert();
        exit(Columns(Report::"BX CalcFields"));
    end;
}
"#;

fn run(routine: &str) -> Vec<String> {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
        .split('\\')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// BcXmlBlob.al:93-107 (Xml_BlobMediaColumns_EmptyAndText): the report does not calculate the Blob, and the content shows.
#[test]
fn blob_and_empty_media_columns() {
    assert_eq!(
        run("EmptyAndText"),
        [
            r#"<Column name="Id">1</Column>"#,
            r#"<Column name="BlobCol"></Column>"#,
            r#"<Column name="MediaCol"></Column>"#,
            r#"<Column name="MediaSetCol"></Column>"#,
            r#"<Column name="NoteCol">empty</Column>"#,
            r#"<Column name="Id">2</Column>"#,
            r#"<Column name="BlobCol">SGVsbG8gQkM=</Column>"#,
            r#"<Column name="MediaCol"></Column>"#,
            r#"<Column name="MediaSetCol"></Column>"#,
            r#"<Column name="NoteCol">text</Column>"#,
        ]
    );
}

/// BcXmlBlob.al:110-124 (Xml_BlobColumn_CalcFieldsInTrigger).
#[test]
fn blob_column_after_calcfields() {
    assert_eq!(
        run("CalcFieldsInTrigger"),
        [
            r#"<Column name="Id">1</Column>"#,
            r#"<Column name="BlobCol"></Column>"#,
            r#"<Column name="Id">2</Column>"#,
            r#"<Column name="BlobCol">SGVsbG8gQkM=</Column>"#,
        ]
    );
}

/// BcXmlBlob.al:127-154 (Xml_BlobColumn_BinaryBytes).
#[test]
fn blob_column_of_binary_bytes() {
    assert_eq!(
        run("BinaryBytes"),
        [
            r#"<Column name="Id">3</Column>"#,
            r#"<Column name="BlobCol">AP9B</Column>"#
        ]
    );
}
