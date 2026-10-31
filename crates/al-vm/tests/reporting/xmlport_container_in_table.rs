//! XmlPort import of a text element that only holds field elements (a container) inside a table element, with the container's
//! MinOccurs at its default (Once) and at Zero (misc/lt-y13-triage/Y13XmlPortContainer.al:115-163, corpus master): complete rows
//! import; a container that comes twice in one row gives one row with the values of the last; an empty row is a schema error
//! with Once and one row of default values with Zero.

use al_testkit::text;

const SRC: &str = r#"
table 69920 "XC Row"
{
    fields { field(1; Id; Integer) { AutoIncrement = true; } field(2; Name; Text[30]) { } field(3; Qty; Integer) { } field(4; Xml; Blob) { } }
    keys { key(PK; Id) { } }
}
xmlport 69921 "XC Once"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "XC Row")
            {
                XmlName = 'Row';
                textelement(Info) { fieldelement(Name; Row.Name) { } fieldelement(Qty; Row.Qty) { } }
            }
        }
    }
}
xmlport 69922 "XC Zero"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "XC Row")
            {
                XmlName = 'Row';
                textelement(Info) { MinOccurs = Zero; fieldelement(Name; Row.Name) { } fieldelement(Qty; Row.Qty) { } }
            }
        }
    }
}
codeunit 69923 C
{
    [TryFunction]
    local procedure TryImport(PortId: Integer; Body: Text)
    var Holder: Record "XC Row" temporary; InS: InStream; OutS: OutStream;
    begin
        Holder.Xml.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<?xml version="1.0" encoding="utf-8"?><Root>' + Body + '</Root>');
        Holder.Xml.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(PortId, InS);
    end;
    local procedure Outcome(PortId: Integer; Body: Text): Text
    var Row: Record "XC Row"; Out: Text;
    begin
        Row.DeleteAll();
        if TryImport(PortId, Body) then Out := 'ok' else Out := 'error ' + GetLastErrorText() + ' (' + GetLastErrorCode() + ')';
        Out += ' /';
        if Row.FindSet() then
            repeat Out += ' [' + Row.Name + '|' + Format(Row.Qty) + ']'; until Row.Next() = 0;
        exit(Out + ';');
    end;
    procedure F(): Text
    var Out: Text;
    begin
        Out := Outcome(69921, '<Row><Info><Name>A</Name><Qty>1</Qty></Info><Info><Name>B</Name><Qty>2</Qty></Info></Row>');
        Out += Outcome(69922, '<Row><Info><Name>A</Name><Qty>1</Qty></Info><Info><Name>B</Name><Qty>2</Qty></Info></Row>');
        Out += Outcome(69921, '<Row></Row>');
        Out += Outcome(69922, '<Row><Info><Name>A</Name><Qty>1</Qty></Info></Row><Row></Row>');
        Out += Outcome(69921, '<Row><Name>A</Name><Qty>1</Qty></Row>');
        exit(Out);
    end;
}
"#;

#[test]
fn a_container_text_element_in_a_table_element_imports_as_pinned() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "F")),
        "ok / [B|2];\
         ok / [B|2];\
         error Imported XML cannot validate with the schema: The element 'Row' has incomplete content. List of possible elements expected: 'Info'. (XmlPortInvalidXml) /;\
         ok / [A|1] [|0];\
         error Imported XML cannot validate with the schema: The element 'Row' has invalid child element 'Name'. List of possible elements expected: 'Info'. (XmlPortInvalidXml) /;"
    );
}
