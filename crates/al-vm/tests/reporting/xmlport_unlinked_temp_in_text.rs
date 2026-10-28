//! A text element (MinOccurs = Zero, MaxOccurs = Once) right below a temporary table element, holding a temporary table element
//! without LinkTable (assumption `xmlport-unlinked-temp-in-text`; corpus branch al-runtime-rs-staging-pub-tp,
//! xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TP_Unlinked_*).

use al_testkit::text;

const SRC: &str = r#"
table 50 "TU Row"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Status; Text[30]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 51 "TU Attr"
{
    fields
    {
        field(1; Parent; Code[10]) { }
        field(2; "Code"; Code[20]) { }
        field(3; Value; Text[30]) { }
    }
    keys { key(PK; Parent, "Code") { Clustered = true; } }
}

table 52 "TU Blob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 53 "TU Unlinked"
{
    Direction = Both;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(root)
        {
            tableelement(row; "TU Row")
            {
                UseTemporary = true;
                MinOccurs = Zero;
                MaxOccurs = Unbounded;
                fieldattribute(no; row."No.") { }
                textelement(attributes)
                {
                    MinOccurs = Zero;
                    MaxOccurs = Once;
                    tableelement(attr; "TU Attr")
                    {
                        MinOccurs = Zero;
                        XmlName = 'attribute';
                        UseTemporary = true;
                        fieldattribute(code; attr."Code") { }
                        fieldattribute(value; attr.Value) { }
                        trigger OnAfterGetRecord()
                        begin
                            Log += 'get attr ' + attr."Code" + ';';
                        end;
                        trigger OnBeforeInsertRecord()
                        begin
                            Log += 'before insert attr ' + attr."Code" + ' row=' + row."No." + ';';
                            attr.Parent := row."No.";
                        end;
                    }
                }
                fieldelement(status; row.Status)
                {
                    MinOccurs = Zero;
                }
                trigger OnBeforeInsertRecord()
                begin
                    Log += 'before insert ' + row."No." + ';';
                end;
            }
        }
    }
    var
        Log: Text;

    procedure AddRow(No: Code[10]; NewStatus: Text[30])
    begin
        row."No." := No;
        row.Status := NewStatus;
        row.Insert();
    end;

    procedure AddAttr(NewParent: Code[10]; NewCode: Code[20]; NewValue: Text[30])
    begin
        attr.Parent := NewParent;
        attr."Code" := NewCode;
        attr.Value := NewValue;
        attr.Insert();
    end;

    procedure Rows() Result: Text
    begin
        if row.FindSet() then
            repeat
                Result += row."No." + ':' + row.Status + ';';
            until row.Next() = 0;
        Result += ' attrs=';
        if attr.FindSet() then
            repeat
                Result += attr.Parent + '/' + attr."Code" + '=' + attr.Value + ';';
            until attr.Next() = 0;
    end;

    procedure GetLog(): Text
    begin
        exit(Log);
    end;
}

codeunit 54 C
{
    procedure ExportRows(): Text
    var
        P: XmlPort "TU Unlinked";
        B: Record "TU Blob" temporary;
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Xml: Text;
    begin
        P.AddRow('R1', 's1');
        P.AddRow('R2', 's2');
        P.AddAttr('R1', 'A', '1');
        P.AddAttr('X', 'C', '3');
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
        P: XmlPort "TU Unlinked";
        B: Record "TU Blob" temporary;
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
        exit(ImportRows('<root><row no="R1"><attributes><attribute code="A" value="1"/><attribute code="B" value="2"/></attributes><status>s1</status></row><row no="R2"><status>s2</status></row></root>'));
    end;

    procedure ImportSameCode(): Text
    begin
        asserterror ImportRows('<root><row no="R1"><attributes><attribute code="A" value="1"/><attribute code="A" value="2"/></attributes></row></root>');
        exit(GetLastErrorText());
    end;

    procedure ImportTwice(): Text
    begin
        asserterror ImportRows('<root><row no="R1"><attributes/><attributes/></row></root>');
        exit(GetLastErrorText());
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

// QrXpContainerOccurs.al TP_Unlinked_Export: every outer row writes all the inner records.
#[test]
fn an_unlinked_temporary_table_element_in_a_text_element_exports_all_its_rows_under_every_row() {
    assert_eq!(
        run("ExportRows"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|<root>|  <row no=\"R1\">|    <attributes>|      <attribute code=\"A\" value=\"1\" />|      <attribute code=\"C\" value=\"3\" />|    </attributes>|    <status>s1</status>|  </row>|  <row no=\"R2\">|    <attributes>|      <attribute code=\"A\" value=\"1\" />|      <attribute code=\"C\" value=\"3\" />|    </attributes>|    <status>s2</status>|  </row>|</root>|get attr A;get attr C;get attr A;get attr C;"
    );
}

// TP_Unlinked_Import `two rows`, `attributes absent`: the inner rows are stored before the outer row's OnBeforeInsertRecord.
#[test]
fn an_unlinked_temporary_table_element_in_a_text_element_imports_its_rows_before_the_outer_row() {
    assert_eq!(
        run("ImportTwo"),
        "before insert attr A row=R1;before insert attr B row=R1;before insert R1;before insert R2; R1:s1;R2:s2; attrs=R1/A=1;R1/B=2;"
    );
}

// TP_Unlinked_Import_Shapes `same code twice`, `attributes twice`.
#[test]
fn an_unlinked_temporary_table_element_in_a_text_element_fails_on_a_duplicate_or_a_second_container() {
    assert_eq!(
        run("ImportSameCode"),
        "The record in table TU Attr already exists. Identification fields and values: Parent='R1',Code='A'"
    );
    assert_eq!(
        run("ImportTwice"),
        "Imported XML cannot validate with the schema: The element 'row' has invalid child element 'attributes'. List of possible elements expected: 'status'."
    );
}
