//! A report dataset column on a Blob or Media field of a global record variable (al-runner issue 7399): only the shapes
//! reports/dataset/DatasetBlobColumnOfGlobalRecord.al pins run (`al-vm reports/blob_column.rs`, `Session::media_field`); the
//! others keep the base behavior.

use al_testrun::Outcome;

const SRC: &str = r#"
table 69960 "GB Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
        field(4; Photo; Media) { }
        field(6; Pics; MediaSet) { }
    }
    keys { key(PK; Id) { } }
}
table 69968 "GB Line" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { } } }
codeunit 69961 "GB Mode"
{
    SingleInstance = true;
    var
        M: Integer;
    procedure SetMode(N: Integer) begin M := N; end;
    procedure Mode(): Integer begin exit(M); end;
}
report 69962 "GB Report"
{
    dataset
    {
        dataitem(One; "GB Line")
        {
            column(C; G.Data) { }
            column(T; TempRow.Data) { }
            column(P; G.Pics) { }
        }
    }
    trigger OnPreReport()
    var
        Mode: Codeunit "GB Mode";
        OutS: OutStream;
    begin
        case Mode.Mode() of
            1:
                begin
                    G.Init();
                    G.Data.CreateOutStream(OutS, TextEncoding::UTF8);
                    OutS.WriteText('New');
                end;
            2:
                begin
                    G.Get(1);
                    G.Data.CreateOutStream(OutS, TextEncoding::UTF8);
                    OutS.WriteText('New');
                end;
            3:
                begin
                    G.Get(2);
                    Clear(G.Data);
                    G.Data.CreateOutStream(OutS, TextEncoding::UTF8);
                    OutS.WriteText('New');
                end;
            4:
                begin
                    TempRow.Init();
                    TempRow.Data.CreateOutStream(OutS, TextEncoding::UTF8);
                    OutS.WriteText('New');
                end;
        end;
    end;
    var
        G: Record "GB Row";
        TempRow: Record "GB Row" temporary;
}
report 69963 "GB Ext Base"
{
    dataset { dataitem(One; "GB Line") { column(N; One.Number) { } } }
}
reportextension 69964 "GB Ext" extends "GB Ext Base"
{
    dataset
    {
        add(One)
        {
            column(EC; E.Data) { }
            column(EM; E.Photo) { }
        }
    }
    trigger OnPreReport() begin E.Get(2); end;
    var
        E: Record "GB Row";
}
report 69965 "GB Ext Media Base"
{
    dataset { dataitem(One; "GB Line") { column(N; One.Number) { } } }
}
reportextension 69966 "GB Ext Media" extends "GB Ext Media Base"
{
    dataset { add(One) { column(EM; E.Photo) { } } }
    var
        E: Record "GB Row";
}
codeunit 69967 "GB Tests"
{
    Subtype = Test;

    local procedure Seed()
    var
        Row: Record "GB Row";
        Line: Record "GB Line";
        OutS: OutStream;
    begin
        Line.DeleteAll();
        Line.Number := 1;
        Line.Insert();
        Row.DeleteAll();
        Row.Init();
        Row.Id := 1;
        Row.Insert();
        Row.Init();
        Row.Id := 2;
        Row.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('Hello BC');
        Row.Insert();
    end;

    local procedure Save(Id: Integer; Mode: Integer): Text
    var
        Buf: Record "GB Row" temporary;
        M: Codeunit "GB Mode";
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        All: Text;
    begin
        Seed();
        M.SetMode(Mode);
        Buf.Data.CreateOutStream(OutS);
        Report.SaveAs(Id, '', ReportFormat::Xml, OutS);
        Buf.Data.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            All += Line.Trim();
        end;
        exit(All);
    end;

    local procedure Expect(Dataset: Text; Part: Text)
    begin
        if Dataset.IndexOf(Part) = 0 then
            Error('%1 not in %2', Part, Dataset);
    end;

    [Test]
    procedure InitWrite()
    begin
        Expect(Save(Report::"GB Report", 1), '<Column name="C">TmV3</Column>');
    end;

    [Test]
    procedure EmptyGetThenWrite() begin Save(Report::"GB Report", 2); end;

    [Test]
    procedure ClearWrite() begin Save(Report::"GB Report", 3); end;

    [Test]
    procedure TempWriteNoInsert() begin Save(Report::"GB Report", 4); end;

    [Test]
    procedure EmptyMediaSetOfGlobalRecord()
    begin
        Expect(Save(Report::"GB Report", 1), '<Column name="P">00000000-0000-0000-0000-000000000000</Column>');
    end;

    [Test]
    procedure ExtensionGlobalBlob() begin Save(Report::"GB Ext Base", 0); end;

    [Test]
    procedure ExtensionGlobalMedia()
    begin
        Expect(Save(Report::"GB Ext Media Base", 0), '<Column name="EM">00000000-0000-0000-0000-000000000000</Column>');
    end;
}
"#;

fn outcome(name: &str) -> Outcome {
    al_testkit::outcomes_in(SRC, "GB Tests")
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, o)| o)
        .expect("the test ran")
}

fn refused(name: &str, needle: &str) {
    match outcome(name) {
        Outcome::NotRun(why) => assert!(why.contains(needle), "{name}: {why}"),
        other => panic!("{name}: expected a refusal, got {other:?}"),
    }
}

/// The pinned write: Init, one stream, one write into a Blob that was never stored (DatasetBlobColumnOfGlobalRecord.al:84-86).
#[test]
fn a_blob_written_in_memory_after_init_shows_the_content() {
    assert_eq!(outcome("InitWrite"), Outcome::Pass);
}

/// A write after a Get (of an empty row, or after Clear of the field) and a write into a temporary record are not pinned.
#[test]
fn other_writes_into_a_global_blob_are_refused() {
    for name in ["EmptyGetThenWrite", "ClearWrite", "TempWriteNoInsert"] {
        refused(name, "holds a Blob a stream wrote after it was read");
    }
}

/// Only an empty Media field of the report's own global record is pinned as an empty element; an empty MediaSet keeps the
/// zero Guid of the base.
#[test]
fn an_empty_media_set_of_a_global_record_keeps_the_zero_guid() {
    assert_eq!(outcome("EmptyMediaSetOfGlobalRecord"), Outcome::Pass);
}

/// A column a reportextension adds on a field of its own global record is not pinned: the Blob stays refused, the Media keeps the
/// zero Guid of the base.
#[test]
fn a_report_extension_global_record_column_keeps_the_base() {
    refused(
        "ExtensionGlobalBlob",
        "not calculated and is not a field of the data item",
    );
    assert_eq!(outcome("ExtensionGlobalMedia"), Outcome::Pass);
}
