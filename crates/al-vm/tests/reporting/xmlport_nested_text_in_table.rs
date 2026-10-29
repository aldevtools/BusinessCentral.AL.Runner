//! A text element right below a table element with field and text attributes and one text element of only text attributes
//! inside it (assumption `xmlport-nested-text-in-table`; corpus branch al-runtime-rs-staging-pub-tp,
//! xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TP_Nested_*).

use al_testkit::text;

const SRC: &str = r#"
table 50 "TN Row"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Status; Text[30]) { }
        field(3; Adm; Text[30]) { }
        field(4; AdmCode; Text[30]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 51 "TN Blob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 52 "TN Nested"
{
    Direction = Both;
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(root)
        {
            tableelement(row; "TN Row")
            {
                UseTemporary = true;
                MinOccurs = Zero;
                MaxOccurs = Unbounded;
                textelement(entry)
                {
                    fieldattribute(no; row."No.") { }
                    textattribute(status)
                    {
                        trigger OnBeforePassVariable()
                        begin
                            Log += 'pass status;';
                            status := row.Status;
                        end;
                        trigger OnAfterAssignVariable()
                        begin
                            Log += 'status=' + status + ';';
                        end;
                    }
                    textelement(admission)
                    {
                        textattribute(admcode)
                        {
                            XmlName = 'option_value';
                            Occurrence = Required;
                            trigger OnBeforePassVariable()
                            begin
                                Log += 'pass admcode;';
                                admcode := row.AdmCode;
                            end;
                        }
                        trigger OnBeforePassVariable()
                        begin
                            Log += 'pass admission;';
                            admission := row.Adm;
                        end;
                        trigger OnAfterAssignVariable()
                        begin
                            Log += 'admission=' + admission + ' admcode=' + admcode + ';';
                        end;
                    }
                    trigger OnBeforePassVariable()
                    begin
                        Log += 'pass entry;';
                    end;
                    trigger OnAfterAssignVariable()
                    begin
                        Log += 'entry=' + entry + ';';
                    end;
                }
                trigger OnBeforeInsertRecord()
                begin
                    row.Status := status;
                    row.Adm := admission;
                    row.AdmCode := admcode;
                end;
            }
        }
    }
    var
        Log: Text;

    procedure AddRow(No: Code[10]; NewStatus: Text[30]; NewAdm: Text[30]; NewAdmCode: Text[30])
    begin
        row."No." := No;
        row.Status := NewStatus;
        row.Adm := NewAdm;
        row.AdmCode := NewAdmCode;
        row.Insert();
    end;

    procedure Rows() Result: Text
    begin
        if row.FindSet() then
            repeat
                Result += row."No." + ':' + row.Status + '/' + row.Adm + '/' + row.AdmCode + ';';
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
        P: XmlPort "TN Nested";
        B: Record "TN Blob" temporary;
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Xml: Text;
    begin
        P.AddRow('R1', 'open', 'A1', '1');
        P.AddRow('R2', '', '', '2');
        P.AddRow('R3', 'closed', 'X', '');
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
        P: XmlPort "TN Nested";
        B: Record "TN Blob" temporary;
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
        exit(ImportRows('<root><row><entry no="R1" status="open"><admission option_value="1">A1</admission></entry></row><row><entry no="R2" status="closed">T<admission option_value="2"/></entry></row></root>'));
    end;

    procedure ImportNoAdmission(): Text
    begin
        asserterror ImportRows('<root><row><entry no="R1" status="open"/></row></root>');
        exit(GetLastErrorText());
    end;

    procedure ImportNoOptionValue(): Text
    begin
        asserterror ImportRows('<root><row><entry no="R1" status="open"><admission>A1</admission></entry></row></root>');
        exit(GetLastErrorText());
    end;

    procedure ImportNoStatus(): Text
    begin
        asserterror ImportRows('<root><row><entry no="R1"><admission option_value="1">A1</admission></entry></row></root>');
        exit(GetLastErrorText());
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

// QrXpContainerOccurs.al TP_Nested_Export: the outer element's trigger, its attributes, then the inner element's trigger before
// its attribute's; an empty inner text leaves the element empty, an empty Required attribute is written as `option_value=""`.
#[test]
fn a_text_element_inside_a_text_element_below_a_table_element_exports_after_the_outer_attributes() {
    assert_eq!(
        run("ExportRows"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|<root>|  <row>|    <entry no=\"R1\" status=\"open\">|      <admission option_value=\"1\">A1</admission>|    </entry>|  </row>|  <row>|    <entry no=\"R2\" status=\"\">|      <admission option_value=\"2\" />|    </entry>|  </row>|  <row>|    <entry no=\"R3\" status=\"closed\">|      <admission option_value=\"\">X</admission>|    </entry>|  </row>|</root>|pass entry;pass status;pass admission;pass admcode;pass entry;pass status;pass admission;pass admcode;pass entry;pass status;pass admission;pass admcode;"
    );
}

// TP_Nested_Import `two rows`, TP_Nested_Import_Shapes `text in entry`: the outer attributes, the inner element, then the outer
// element's own text.
#[test]
fn a_text_element_inside_a_text_element_below_a_table_element_imports_inner_before_outer_text() {
    assert_eq!(
        run("ImportTwo"),
        "status=open;admission=A1 admcode=1;entry=;status=closed;admission= admcode=2;entry=T; R1:open/A1/1;R2:closed//2;"
    );
}

// TP_Nested_Import_Shapes: a missing inner element, a missing Required inner attribute, a missing outer text attribute.
#[test]
fn a_text_element_inside_a_text_element_below_a_table_element_fails_the_schema_check() {
    assert_eq!(
        run("ImportNoAdmission"),
        "Imported XML cannot validate with the schema: The element 'entry' has incomplete content. List of possible elements expected: 'admission'."
    );
    assert_eq!(
        run("ImportNoOptionValue"),
        "Imported XML cannot validate with the schema: The required attribute 'option_value' is missing."
    );
    assert_eq!(
        run("ImportNoStatus"),
        "Imported XML cannot validate with the schema: The required attribute 'status' is missing."
    );
}
