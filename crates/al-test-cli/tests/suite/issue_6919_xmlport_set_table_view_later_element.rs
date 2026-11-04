//! `XmlPort.SetTableView` with a record of a table element that is not the first one (al-runner issue 6919, the reporter's
//! repro): the first element is a temporary table of another table; BC runs the export.

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50101 "Repro Receiver"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50102 "Repro Transport"
{
    fields
    {
        field(1; "Entry no."; Code[10]) { }
        field(2; Site; Code[10]) { }
    }
    keys { key(PK; "Entry no.") { Clustered = true; } }
}

table 50103 "Repro Blob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 50100 "Repro Export"
{
    Direction = Export;
    Format = Xml;

    schema
    {
        textelement(Root)
        {
            tableelement(receivers; "Repro Receiver")
            {
                UseTemporary = true;
                fieldelement(Id; receivers.Id) { }
            }
            tableelement(transportinbound; "Repro Transport")
            {
                fieldelement(EntryNo; transportinbound."Entry no.") { }
                fieldelement(Site; transportinbound.Site) { }
            }
        }
    }
}

codeunit 50101 "Repro SetTableView"
{
    Subtype = Test;

    [Test]
    procedure SetTableView_WithTheRecordOfALaterTableElement_ExportsIt()
    var
        Transport: Record "Repro Transport";
        Export: XmlPort "Repro Export";
        TempBlob: Record "Repro Blob" temporary;
        OutStr: OutStream;
        InStr: InStream;
        Result: Text;
        Line: Text;
    begin
        Transport."Entry no." := 'T1';
        Transport.Site := 'S1';
        Transport.Insert();
        Transport.SetRecFilter();

        TempBlob.Data.CreateOutStream(OutStr);
        Export.SetTableView(Transport);
        Export.SetDestination(OutStr);
        Export.Export();

        TempBlob.Data.CreateInStream(InStr, TextEncoding::UTF16);
        while not InStr.EOS() do begin
            InStr.ReadText(Line);
            Result += Line;
        end;
        if StrPos(Result, 'T1') = 0 then
            Error('The export does not hold T1: %1', Result);
    end;
}
"#;

#[test]
fn set_table_view_with_the_record_of_a_later_table_element_exports() {
    let outcomes = outcomes_in(SRC, "Repro SetTableView");
    assert_eq!(
        outcomes,
        vec![(
            "SetTableView_WithTheRecordOfALaterTableElement_ExportsIt".to_string(),
            al_testrun::Outcome::Pass
        )]
    );
}

/// The ports of xmlports/set-table-view/XmlPortSetTableViewElement.al:128-266 (corpus PR 1121), with the
/// exports BC 29.0 gave.
const PROBE: &str = r#"
table 50120 "S14" { fields { field(1; Code; Code[10]) { } field(2; Name; Text[20]) { } } keys { key(PK; Code) { } } }
table 50121 "S15" { fields { field(1; Id; Integer) { } field(2; Grp; Code[10]) { } } keys { key(PK; Id) { } } }
table 50122 "S00" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 50123 "S01" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
table 50124 "XB" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
xmlport 50120 "XP STV Siblings"
{
    Direction = Export; Format = Xml;
    schema { textelement(Root) {
        tableelement(receivers; "S01") { UseTemporary = true; fieldelement(Id; receivers.Id) { } }
        tableelement(transport; "S14") { fieldelement(Code; transport.Code) { } fieldelement(Name; transport.Name) { } } } }
}
xmlport 50121 "XP STV Nested"
{
    Direction = Export; Format = Xml;
    schema { textelement(Root) {
        tableelement(header; "S14") { fieldattribute(Code; header.Code) { }
            tableelement(line; "S15") { LinkTable = header; LinkFields = Grp = field(Code); fieldattribute(Id; line.Id) { } } } } }
}
xmlport 50122 "XP STV Same Table"
{
    Direction = Export; Format = Xml;
    schema { textelement(Root) {
        tableelement(first; "S14") { XmlName = 'First'; fieldattribute(Code; first.Code) { } }
        tableelement(second; "S14") { XmlName = 'Second'; fieldattribute(Code; second.Code) { } } } }
}
codeunit 50125 C
{
    local procedure Seed()
    var T: Record "S14"; L: Record "S15";
    begin
        T.DeleteAll(); L.DeleteAll();
        T.Code := 'T1'; T.Name := 'S1'; T.Insert(); T.Code := 'T2'; T.Name := 'S2'; T.Insert();
        L.Id := 1; L.Grp := 'T1'; L.Insert(); L.Id := 2; L.Grp := 'T2'; L.Insert(); L.Id := 3; L.Grp := 'T1'; L.Insert();
    end;
    local procedure ReadAll(var B: Record "XB" temporary): Text
    var InS: InStream; Line: Text; R: Text;
    begin
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(Line); R += Line + '|'; end;
        exit(R);
    end;
    procedure Nested(): Text
    var L: Record "S15"; P: XmlPort "XP STV Nested"; B: Record "XB" temporary; O: OutStream;
    begin
        Seed(); L.SetFilter(Id, '<>3'); B.Data.CreateOutStream(O); P.SetTableView(L); P.SetDestination(O); P.Export(); exit(ReadAll(B));
    end;
    procedure SameTable(): Text
    var T: Record "S14"; P: XmlPort "XP STV Same Table"; B: Record "XB" temporary; O: OutStream;
    begin
        Seed(); T.SetRange(Code, 'T2'); B.Data.CreateOutStream(O); P.SetTableView(T); P.SetDestination(O); P.Export(); exit(ReadAll(B));
    end;
    procedure Sibling(): Text
    var T: Record "S14"; P: XmlPort "XP STV Siblings"; B: Record "XB" temporary; O: OutStream;
    begin
        Seed(); T.SetRange(Name, 'S2'); B.Data.CreateOutStream(O); P.SetTableView(T); P.SetDestination(O); P.Export(); exit(ReadAll(B));
    end;
    procedure NoElement(): Text
    var U: Record "S00"; P: XmlPort "XP STV Siblings";
    begin
        U.SetRange(Id, 1);
        asserterror P.SetTableView(U);
        exit(GetLastErrorCode() + '|' + GetLastErrorText());
    end;
}
"#;

const HEAD: &str = r#"<?xml version="1.0" encoding="UTF-16" standalone="no"?>|<Root>|"#;

/// SetTableView_RecordOfLaterSiblingElement_RangeFilter `export`.
#[test]
fn the_view_filters_the_later_sibling_element_of_its_table() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PROBE, "C", "Sibling")),
        format!(
            "{HEAD}  <receivers>|    <Id>0</Id>|  </receivers>|  <transport>|    <Code>T2</Code>|    <Name>S2</Name>|  </transport>|</Root>|"
        )
    );
}

/// SetTableView_RecordOfNestedChildElement_Export `export`.
#[test]
fn the_view_filters_a_nested_child_element_of_its_table() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PROBE, "C", "Nested")),
        format!(
            "{HEAD}  <header Code=\"T1\">|    <line Id=\"1\" />|  </header>|  <header Code=\"T2\">|    <line Id=\"2\" />|  </header>|</Root>|"
        )
    );
}

/// SetTableView_TwoElementsOfTheSameTable_Export `export`: only the first element of the table gets the view.
#[test]
fn the_view_goes_to_the_first_element_of_its_table_only() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PROBE, "C", "SameTable")),
        format!("{HEAD}  <First Code=\"T2\" />|  <Second Code=\"T1\" />|  <Second Code=\"T2\" />|</Root>|")
    );
}

/// SetTableView_RecordOfTableNoElementUses `settableview`.
#[test]
fn a_record_of_a_table_no_element_uses_raises_xmlport_node_not_found() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(PROBE, "C", "NoElement")),
        "XmlPortNodeNotFound|The XP STV Siblings XmlPort does not have a XmlItem that uses the table (Table 50122 S00) specified in the function SetTableView."
    );
}
