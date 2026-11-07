//! E-invoice export shapes (streams/Y72XpEInvoiceShapes.al): an attributed leaf with Skip two containers
//! below a table element (EndpointID), a container right below a table element that holds an inner table element (TaxTotal), and
//! a blank attribute imported into an Enum field with OnValidate (Y72XpTextTypeBlankEnum.al).

use al_testkit::{run, text};

const SRC: &str = r#"
table 69960 "XE Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
enum 69961 "XE Kind" { value(0; "None") { Caption = ' '; } value(1; One) { Caption = 'One'; } }
table 69967 "XE Num" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { } } }
table 69962 "XE Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Res; Enum "XE Kind") { trigger OnValidate() var L: Codeunit "XE Log"; begin L.Add('validate ' + Format(Rec.Res.AsInteger())); end; }
    }
    keys { key(PK; Id) { } }
}
codeunit 69963 "XE Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
xmlport 69964 "XE Party"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(Doc)
        {
        tableelement(hdr; "XE Num")
        {
            MaxOccurs = Once; SourceTableView = sorting(Number) where(Number = filter(1 ..)); XmlName = 'Invoice';
            textelement(Supplier)
            {
                textelement(Party)
                {
                    textelement(endpointid)
                    {
                        XmlName = 'EndpointID';
                        textattribute(schemeid) { XmlName = 'schemeID'; }
                        trigger OnBeforePassVariable() var L: Codeunit "XE Log"; begin L.Add('endpoint'); EndpointId := Ep; SchemeId := '0088'; if EndpointId = '' then currXMLport.Skip(); end;
                    }
                    textelement(Name) { trigger OnBeforePassVariable() begin Name := 'N'; end; }
                }
            }
            textelement(TaxTotal)
            {
                textelement(TaxAmount) { }
                tableelement(sub; "XE Num")
                {
                    XmlName = 'TaxSubtotal'; SourceTableView = sorting(Number) where(Number = filter(1 ..));
                    textelement(Amt) { }
                    trigger OnPreXmlItem() var L: Codeunit "XE Log"; begin L.Add('pre sub'); end;
                    trigger OnAfterGetRecord() var L: Codeunit "XE Log"; begin L.Add('agr sub ' + Format(sub.Number)); if sub.Number > 2 then currXMLport.Break(); Amt := Format(sub.Number); end;
                }
                trigger OnBeforePassVariable() var L: Codeunit "XE Log"; begin L.Add('taxtotal'); TaxAmount := '30'; end;
            }
            trigger OnAfterGetRecord() begin if hdr.Number > 1 then currXMLport.Break(); end;
        }
        }
    }
    var Ep: Text;
    procedure SetEp(E: Text) begin Ep := E; end;
}
xmlport 69965 "XE Enum"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(e; "XE Row") { fieldattribute(Id; e.Id) { } fieldattribute(Res; e.Res) { } } } }
}
codeunit 69966 C
{
    local procedure Export(Ep: Text): Text
    var B: Record "XE Blob" temporary; X: Xmlport "XE Party"; OutS: OutStream; InS: InStream; T: Text; Line: Text; L: Codeunit "XE Log"; N: Record "XE Num"; i: Integer;
    begin
        for i := 1 to 3 do begin N.Number := i; if N.Insert() then; end;
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        X.SetEp(Ep);
        X.SetDestination(OutS);
        X.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line.Trim(); end;
        exit(T + '|' + L.Take());
    end;
    procedure Filled(): Text begin exit(Export('EP1')); end;
    procedure Empty(): Text begin exit(Export('')); end;
    procedure BlankEnum(): Text
    var B: Record "XE Blob" temporary; R: Record "XE Row"; OutS: OutStream; InS: InStream; L: Codeunit "XE Log";
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<Root><e Id="5" Res="" /><e Id="6" Res=" " /><e Id="7" Res="One" /></Root>');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"XE Enum", InS);
        R.FindSet();
        repeat L.Add(Format(R.Id) + '=' + Format(R.Res.AsInteger())); until R.Next() = 0;
        exit(L.Take());
    end;
}
"#;

#[test]
fn an_attributed_leaf_below_two_containers_and_a_container_with_an_inner_table_export() {
    let out = text(run(SRC, "Filled"));
    assert!(
        out.contains(r#"<Party><EndpointID schemeID="0088">EP1</EndpointID><Name>N</Name></Party>"#),
        "{out}"
    );
    assert!(
        out.contains("<TaxTotal><TaxAmount>30</TaxAmount><TaxSubtotal><Amt>1</Amt></TaxSubtotal><TaxSubtotal><Amt>2</Amt></TaxSubtotal></TaxTotal>"),
        "{out}"
    );
    assert!(
        out.ends_with("|endpoint;taxtotal;pre sub;agr sub 1;agr sub 2;agr sub 3;"),
        "{out}"
    );
    let out = text(run(SRC, "Empty"));
    assert!(out.contains("<Party><Name>N</Name></Party>"), "{out}");
}

#[test]
fn a_blank_attribute_into_an_enum_field_with_onvalidate_is_not_validated() {
    // Y72XpTextTypeBlankEnum.al:847 BlankAttribute_Import_Literal.
    assert_eq!(text(run(SRC, "BlankEnum")), "validate 1;5=0;6=0;7=1;");
}
