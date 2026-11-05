//! A table element whose LinkTable names the table element before it under the same text element (a preceding sibling, not an
//! element above it): `Root > Hdr, Line` with `LinkTable = Hdr` (xmlports/shapes-properties/R6924eLinkSibling.al, al-runner
//! issue 6924). The export writes only the lines of the header element's record as its loop left it (the last header), and no
//! line without a header row; the import leaves the line's link field blank, the header reads blank in the line's triggers, a
//! non-blank link field in the document conflicts, and a header after a line fails the schema check.

use al_testkit::{error_message, run, text};

const SRC: &str = r#"
table 69990 "LS Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69991 "LS Hdr" { fields { field(1; "No."; Code[20]) { } field(2; Name; Text[30]) { } } keys { key(PK; "No.") { } } }
table 69992 "LS Line" { fields { field(1; "Doc No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Item; Text[30]) { } } keys { key(PK; "Doc No.", "Line No.") { } } }
xmlport 69993 "LS Port"
{
    Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(TransportHeader; "LS Hdr")
            {
                MinOccurs = Zero;
                fieldattribute(No; TransportHeader."No.") { }
                fieldattribute(Name; TransportHeader.Name) { Occurrence = Optional; }
                trigger OnAfterGetRecord() begin Log += 'agr hdr ' + TransportHeader."No." + ';'; end;
                trigger OnBeforeInsertRecord() begin Log += 'ins hdr ' + TransportHeader."No." + ';'; end;
            }
            tableelement(TransportLine; "LS Line")
            {
                MinOccurs = Zero;
                LinkTable = TransportHeader;
                LinkFields = "Doc No." = field("No.");
                fieldattribute(Doc; TransportLine."Doc No.") { Occurrence = Optional; }
                fieldattribute(LineNo; TransportLine."Line No.") { }
                fieldattribute(Item; TransportLine.Item) { Occurrence = Optional; }
                trigger OnAfterGetRecord() begin Log += 'agr line ' + TransportLine."Doc No." + '-' + Format(TransportLine."Line No.") + '|' + TransportHeader."No." + ';'; end;
                trigger OnBeforeInsertRecord() begin Log += 'ins line ' + TransportLine."Doc No." + '-' + Format(TransportLine."Line No.") + '|' + TransportHeader."No." + ';'; end;
            }
        }
    }
    var Log: Text;
    procedure GetLog(): Text begin exit(Log); end;
}
codeunit 69994 C
{
    local procedure AddHdr(No: Code[20])
    var H: Record "LS Hdr";
    begin
        H."No." := No; H.Name := 'n' + No; H.Insert();
    end;

    local procedure AddLine(DocNo: Code[20]; LineNo: Integer)
    var L: Record "LS Line";
    begin
        L."Doc No." := DocNo; L."Line No." := LineNo; L.Item := 'i' + DocNo + Format(LineNo); L.Insert();
    end;

    local procedure DoExport(): Text
    var B: Record "LS Blob" temporary; X: XmlPort "LS Port"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        B.Data.CreateOutStream(OutS);
        X.SetDestination(OutS);
        X.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line.Trim() + '|'; end;
        exit(T + X.GetLog());
    end;

    local procedure DoImport(Xml: Text): Text
    var B: Record "LS Blob" temporary; X: XmlPort "LS Port"; H: Record "LS Hdr"; L: Record "LS Line"; OutS: OutStream; InS: InStream; Rows: Text;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Xml);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        X.SetSource(InS);
        X.Import();
        if H.FindSet() then repeat Rows += 'H ' + H."No." + ';'; until H.Next() = 0;
        if L.FindSet() then repeat Rows += 'L ' + L."Doc No." + '-' + Format(L."Line No.") + ' ' + L.Item + ';'; until L.Next() = 0;
        exit(Rows + '#' + X.GetLog());
    end;

    procedure ExportTwoHeaders(): Text
    begin
        AddHdr('H1'); AddHdr('H2');
        AddLine('H1', 1); AddLine('H1', 2); AddLine('H2', 1); AddLine('X9', 1);
        exit(DoExport());
    end;

    procedure ExportNoHeader(): Text
    begin
        AddLine('H1', 1);
        exit(DoExport());
    end;

    procedure ExportNoHeaderBlankLine(): Text
    begin
        AddLine('', 1);
        exit(DoExport());
    end;

    procedure ImportHeadersFirst(): Text
    begin
        exit(DoImport('<Root><TransportHeader No="H1" /><TransportHeader No="H2" /><TransportLine LineNo="1" Item="a" /><TransportLine LineNo="2" Item="b" /></Root>'));
    end;

    procedure ImportLineWithDoc(): Text
    begin
        exit(DoImport('<Root><TransportHeader No="H1" /><TransportLine Doc="Z1" LineNo="1" Item="a" /></Root>'));
    end;

    procedure ImportInterleaved(): Text
    begin
        exit(DoImport('<Root><TransportHeader No="H1" /><TransportLine LineNo="1" Item="a" /><TransportHeader No="H2" /><TransportLine LineNo="2" Item="b" /></Root>'));
    end;
}
"#;

// R6924eLinkSibling.al:224 LinkSibling_Export_TwoHeaders.
#[test]
fn export_writes_only_the_lines_of_the_last_header() {
    let out = text(run(SRC, "ExportTwoHeaders"));
    assert_eq!(
        out,
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|<Root>|<TransportHeader No=\"H1\" Name=\"nH1\" />|<TransportHeader No=\"H2\" Name=\"nH2\" />|<TransportLine Doc=\"H2\" LineNo=\"1\" Item=\"iH21\" />|</Root>|agr hdr H1;agr hdr H2;agr line H2-1|H2;"
    );
}

// R6924eLinkSibling.al:240 LinkSibling_Export_NoHeader.
#[test]
fn export_without_a_header_row_writes_no_line() {
    let out = text(run(SRC, "ExportNoHeader"));
    assert_eq!(
        out,
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|<Root />|"
    );
}

// Without a header row, a line whose link field is blank is not pinned (BC reads no line; whether a blank link value would match
// it is not observed): refused.
#[test]
fn export_without_a_header_row_and_a_line_with_a_blank_link_is_refused() {
    let r = run(SRC, "ExportNoHeaderBlankLine");
    assert!(format!("{r:?}").contains("not pinned"), "{r:?}");
}

// R6924eLinkSibling.al:251 LinkSibling_Import_HeadersFirst.
#[test]
fn import_leaves_the_link_field_blank() {
    let out = text(run(SRC, "ImportHeadersFirst"));
    assert_eq!(
        out,
        "H H1;H H2;L -1 a;L -2 b;#ins hdr H1;ins hdr H2;ins line -1|;ins line -2|;"
    );
}

// R6924eLinkSibling.al:261 LinkSibling_Import_LineWithDoc.
#[test]
fn import_of_a_line_with_its_own_link_value_conflicts() {
    let msg = error_message(run(SRC, "ImportLineWithDoc"));
    assert_eq!(
        msg,
        "The data conflicts with the LinkTable and LinkFields setup of Table TransportLine."
    );
}

// R6924eLinkSibling.al:271 LinkSibling_Import_Interleaved.
#[test]
fn import_of_a_header_after_a_line_fails_the_schema_check() {
    let msg = error_message(run(SRC, "ImportInterleaved"));
    assert_eq!(
        msg,
        "Imported XML cannot validate with the schema: The element 'Root' has invalid child element 'TransportHeader'. List of possible elements expected: 'TransportLine'."
    );
}
