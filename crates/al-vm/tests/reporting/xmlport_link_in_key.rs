//! Xml import of a nested linked table element whose link field is part of its primary key, with two rows under one
//! parent (Microsoft's DemoTool: Base App xmlport 1225 "Imp / Exp Data Exch Def & Map", Data Exch. Column Def rows). Only
//! the first row gets the link (TestXmlPortTriggersReplaceViewContracts.al:414, a link field outside the key); a later row
//! keeps the fields of the primary key, as Init does (TestXmlPortTriggersReplaceViewContracts.al:648).

use al_testkit::run;

const SRC: &str = r#"
table 69990 "LK Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69991 "LK Head" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 69992 "LK Line"
{
    fields { field(1; HeadCode; Code[10]) { } field(2; LineNo; Integer) { } field(3; Note; Text[10]) { } }
    keys { key(PK; HeadCode, LineNo) { } }
    trigger OnInsert() begin TestField(HeadCode); end;
}
xmlport 69993 "LK Port"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Head; "LK Head")
            {
                fieldattribute(Code; Head.Code) { }
                tableelement(Line; "LK Line")
                {
                    LinkTable = Head; LinkFields = HeadCode = field(Code); MinOccurs = Zero;
                    fieldattribute(LineNo; Line.LineNo) { }
                    fieldattribute(Note; Line.Note) { Occurrence = Optional; }
                }
            }
        }
    }
}
codeunit 69994 C
{
    procedure F(): Text
    var B: Record "LK Blob" temporary; L: Record "LK Line"; OutS: OutStream; InS: InStream; T: Text;
    begin
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Root><Head Code="H1"><Line LineNo="10" Note="a"/><Line LineNo="20"/></Head><Head Code="H2"><Line LineNo="10"/></Head></Root>');
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"LK Port", InS);
        if L.FindSet() then repeat T += L.HeadCode + '/' + Format(L.LineNo) + '/' + L.Note + ','; until L.Next() = 0;
        exit(T);
    end;
}
"#;

#[test]
fn a_later_row_keeps_a_link_field_of_its_primary_key() {
    assert_eq!(al_testkit::text(run(SRC, "F")), "H1/10/a,H1/20/,H2/10/,");
}
