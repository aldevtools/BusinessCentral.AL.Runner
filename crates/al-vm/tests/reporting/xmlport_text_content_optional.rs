//! A text element with MinOccurs = Zero and MaxOccurs = Once right below a temporary table element, holding field elements and
//! leaf text elements (corpus branch al-runtime-rs-staging-pub-tq, xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al
//! TQ_Response_*, objects in TqXpResponseObjects.al).

use al_testkit::text;

const SRC: &str = r#"
table 50 "TR Member"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[50]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

table 51 "TR Blob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 52 "TR Response"
{
    Format = Xml;
    FormatEvaluate = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(response)
        {
            MaxOccurs = Once;
            tableelement(member; "TR Member")
            {
                MinOccurs = Zero;
                MaxOccurs = Unbounded;
                UseTemporary = true;
                fieldattribute(no; member."No.") { }
                textelement(membership)
                {
                    MinOccurs = Zero;
                    MaxOccurs = Once;
                    fieldelement(name; member.Name)
                    {
                        MinOccurs = Zero;
                    }
                    textelement(tier)
                    {
                        MaxOccurs = Once;
                        trigger OnBeforePassVariable()
                        begin
                            Log += 'pass tier;';
                        end;
                        trigger OnAfterAssignVariable()
                        begin
                            Log += 'tier=' + tier + ';';
                        end;
                    }
                    trigger OnBeforePassVariable()
                    begin
                        Log += 'pass membership;';
                    end;
                }
                trigger OnAfterGetRecord()
                begin
                    Log += 'get ' + member."No." + ';';
                end;
            }
        }
    }
    var
        Log: Text;

    procedure AddMember(No: Code[20]; NewName: Text[50])
    begin
        member."No." := No;
        member.Name := NewName;
        member.Insert();
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
        P: XmlPort "TR Response";
        B: Record "TR Blob" temporary;
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Xml: Text;
    begin
        P.AddMember('M1', 'Ann');
        P.AddMember('M2', '');
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
        P: XmlPort "TR Response";
        B: Record "TR Blob" temporary;
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
        exit(P.GetLog());
    end;

    procedure ImportSet(): Text
    begin
        exit(ImportRows('<response><member no="M1"><membership><name>Ann</name><tier>GOLD</tier></membership></member></response>'));
    end;

    procedure ImportEmptyLeaf(): Text
    begin
        exit(ImportRows('<response><member no="M1"><membership><name>Ann</name><tier /></membership></member></response>'));
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

// TQ_Response_Export_Rows, TQ_Response_Export_EmptyValues: `membership` is written for every row, also when all it holds is
// empty; an empty field element with MinOccurs = Zero and an empty leaf text element are written as `<name />`, `<tier />`. The
// row's OnAfterGetRecord, then the container's OnBeforePassVariable, then its leaf's.
#[test]
fn an_optional_text_element_below_a_table_element_is_written_for_every_row() {
    assert_eq!(
        run("ExportRows"),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|<response>|  <member no=\"M1\">|    <membership>|      <name>Ann</name>|      <tier />|    </membership>|  </member>|  <member no=\"M2\">|    <membership>|      <name />|      <tier />|    </membership>|  </member>|</response>|get M1;pass membership;pass tier;get M2;pass membership;pass tier;"
    );
}

// TQ_Response_Import `full`: a leaf text element with a value runs its OnAfterAssignVariable once.
#[test]
fn a_leaf_text_element_with_a_value_in_an_optional_text_element_runs_its_trigger_once() {
    assert_eq!(run("ImportSet"), "tier=GOLD;");
}

// TQ_Response_Import_Shapes `optional leaves absent`: real BC runs the OnAfterAssignVariable of an empty `<tier />` twice
// (`tier=;tier=;`; TR_EmptyLeaf_Import: `<tier></tier>` once, xmlport_empty_leaf_import.rs).
#[test]
fn an_empty_leaf_text_element_with_an_assign_trigger_in_a_text_element_below_a_table_element_runs_it_twice() {
    assert_eq!(run("ImportEmptyLeaf"), "tier=;tier=;");
}
