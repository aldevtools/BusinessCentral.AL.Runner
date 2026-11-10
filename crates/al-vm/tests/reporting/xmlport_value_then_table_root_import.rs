//! XmlPort import of a root table element (MaxOccurs = Once) that holds one leaf text element and one table element: the
//! corpus pins the export of this shape only (xmlports/shapes-properties/XpTextValueNestedTable.al:1535
//! XTT_RootTableValue_Export, :1559 XTT_RowDirect_Export, :1571 XTT_RootTableAfterOnly_Export), so the import is refused.
//! The nearest pinned import (xmlports/shapes-properties/XpTableRootNestedText.al:534 XNT_TextTbl_Import) has the text
//! and the table inside a text element, not right below the root.

use al_builtins::native::Signal;

const SRC: &str = r#"
table 60 "RW Comm" { fields { field(1; Id; Code[10]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 61 "RW Order"
{
    fields { field(1; "No."; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 62 "RW Blob"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 63 "RW BeforePass Only"
{
    Format = Xml; UseRequestPage = false;
    schema
    {
        tableelement(comm; "RW Comm")
        {
            MaxOccurs = Once;
            XmlName = 'Envelope';
            textelement(T1)
            {
                trigger OnBeforePassVariable()
                begin
                    T1 := 'x';
                end;
            }
            tableelement(orders; "RW Order")
            {
                fieldelement(No; orders."No.") { }
            }
        }
    }
}

xmlport 64 "RW No Trigger"
{
    Format = Xml; UseRequestPage = false;
    schema
    {
        tableelement(comm; "RW Comm")
        {
            MaxOccurs = Once;
            XmlName = 'Envelope';
            textelement(T1) { }
            tableelement(orders; "RW Order")
            {
                fieldelement(No; orders."No.") { }
            }
        }
    }
}

codeunit 65 C
{
    local procedure ImportText(Id: Integer; Body: Text)
    var
        B: Record "RW Blob" temporary;
        OutS: OutStream;
        InS: InStream;
    begin
        B.Id := 1;
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Body);
        B.Insert();
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Id, InS);
    end;

    local procedure Stored(): Text
    var
        O: Record "RW Order";
        C: Record "RW Comm";
        R: Text;
    begin
        R := 'comm=' + Format(C.Count()) + ' orders=';
        if O.FindSet() then repeat R += O."No." + ','; until O.Next() = 0;
        exit(R);
    end;

    procedure BeforePassOnly(): Text
    begin
        ImportText(63, '<?xml version="1.0" encoding="UTF-8"?><Envelope><T1>q</T1><orders><No>A1</No></orders></Envelope>');
        exit(Stored());
    end;

    procedure NoTrigger(): Text
    begin
        ImportText(64, '<?xml version="1.0" encoding="UTF-8"?><Envelope><T1>q</T1><orders><No>A1</No></orders></Envelope>');
        exit(Stored());
    end;

    procedure NoTriggerTwoOrders(): Text
    begin
        ImportText(64, '<?xml version="1.0" encoding="UTF-8"?><Envelope><T1>q</T1><orders><No>A1</No></orders><orders><No>A2</No></orders></Envelope>');
        exit(Stored());
    end;
}
"#;

fn refused(routine: &str) {
    match al_testkit::run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(why)) => assert!(
            why.contains("a leaf text element and a table element") && why.contains("not pinned"),
            "{routine}: {why}"
        ),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn the_import_of_a_root_table_element_with_a_valued_leaf_and_a_table_element_is_refused() {
    refused("BeforePassOnly");
}

#[test]
fn the_import_of_a_root_table_element_with_a_plain_leaf_and_a_table_element_is_refused() {
    refused("NoTrigger");
    refused("NoTriggerTwoOrders");
}
