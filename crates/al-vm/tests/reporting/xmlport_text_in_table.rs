//! A text element right below a table element: only text attributes below it, or attributes and leaf elements, with its
//! triggers and text (assumption `xmlport-text-in-table`; corpus branch al-runtime-rs-staging-pub-to,
//! xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TO_AttrItem_*, TO_TextContent_*).

use al_testkit::text;

const SRC: &str = r#"
table 50 "TT Row"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Name; Text[30]) { }
        field(3; Opt; Text[30]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 51 "TT Blob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 52 "TT Attr Item"
{
    Direction = Both;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(root)
        {
            tableelement(row; "TT Row")
            {
                UseTemporary = true;
                MinOccurs = Zero;
                MaxOccurs = Unbounded;
                fieldattribute(no; row."No.") { }
                textelement(item)
                {
                    textattribute(optattr)
                    {
                        XmlName = 'option_value';
                        trigger OnBeforePassVariable()
                        begin
                            Log += 'pass attr;';
                            optattr := row.Opt;
                        end;
                        trigger OnAfterAssignVariable()
                        begin
                            Log += 'attr=' + optattr + ';';
                        end;
                    }
                    trigger OnBeforePassVariable()
                    begin
                        Log += 'pass item;';
                        item := row.Name;
                    end;
                    trigger OnAfterAssignVariable()
                    begin
                        Log += 'item=' + item + ';';
                    end;
                }
                trigger OnBeforeInsertRecord()
                begin
                    row.Name := item;
                    row.Opt := optattr;
                end;
            }
        }
    }
    var
        Log: Text;

    procedure AddRow(No: Code[10]; NewName: Text[30]; NewOpt: Text[30])
    begin
        row."No." := No;
        row.Name := NewName;
        row.Opt := NewOpt;
        row.Insert();
    end;

    procedure Rows() Result: Text
    begin
        if row.FindSet() then
            repeat
                Result += row."No." + ':' + row.Name + '/' + row.Opt + ';';
            until row.Next() = 0;
    end;

    procedure GetLog(): Text
    begin
        exit(Log);
    end;
}

codeunit 53 C
{
    procedure ExportRows(): Text
    var
        P: XmlPort "TT Attr Item";
        B: Record "TT Blob" temporary;
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Xml: Text;
    begin
        P.AddRow('R1', 'REQUIRED', '1');
        P.AddRow('R2', '', '2');
        B.Id := 1;
        B.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        P.SetDestination(OutS);
        P.Export();
        B.Modify();
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            Xml += Line + '|';
        end;
        exit(Xml + P.GetLog());
    end;

    procedure ImportRows(Body: Text): Text
    var
        P: XmlPort "TT Attr Item";
        B: Record "TT Blob" temporary;
        OutS: OutStream;
        InS: InStream;
    begin
        B.Id := 1;
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<?xml version="1.0" encoding="UTF-8"?>' + Body);
        B.Insert();
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        P.SetSource(InS);
        P.Import();
        exit(P.GetLog() + ' ' + P.Rows());
    end;

    procedure ImportTwo(): Text
    begin
        exit(ImportRows('<root><row no="R1"><item option_value="1">REQ</item></row><row no="R2"><item option_value="2">X</item></row></root>'));
    end;

    procedure ImportNoAttribute(): Text
    begin
        asserterror ImportRows('<root><row no="R1"><item>REQ</item></row></root>');
        exit(GetLastErrorText());
    end;

    procedure ImportChild(): Text
    begin
        asserterror ImportRows('<root><row no="R1"><item option_value="1">A<x/></item></row></root>');
        exit(GetLastErrorText());
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

// QrXpContainerOccurs.al TO_AttrItem_Trigger_Export: the element's trigger before its attribute's, its text after the
// attributes, an empty text leaves the element empty.
#[test]
fn a_text_element_with_text_attributes_below_a_table_element_exports_its_text_after_them() {
    assert_eq!(
        run("ExportRows"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|<root>|  <row no=\"R1\">|    <item option_value=\"1\">REQUIRED</item>|  </row>|  <row no=\"R2\">|    <item option_value=\"2\" />|  </row>|</root>|pass item;pass attr;pass item;pass attr;"
    );
}

// TO_AttrItem_Trigger_Import: the attribute, then the element's own text, each with its OnAfterAssignVariable.
#[test]
fn a_text_element_with_text_attributes_below_a_table_element_imports_attributes_then_its_text() {
    assert_eq!(run("ImportTwo"), "attr=1;item=REQ;attr=2;item=X; R1:REQ/1;R2:X/2;");
}

// TO_AttrItem_Trigger_Import `attribute missing`, TO_AttrItem_Trigger_Import_Missing `child element`.
#[test]
fn a_text_element_with_text_attributes_below_a_table_element_fails_the_schema_check() {
    assert_eq!(
        run("ImportNoAttribute"),
        "Imported XML cannot validate with the schema: The required attribute 'option_value' is missing."
    );
    assert_eq!(
        run("ImportChild"),
        "Imported XML cannot validate with the schema: The element 'item' cannot contain child element 'x' because the parent element's content model is text only."
    );
}
