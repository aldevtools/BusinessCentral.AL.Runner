//! Triggers of the root text element beside a table element and a container (xmlports/shapes-properties/Y83XpMiscTests.al
//! ContainerTriggers_Nested, ContainerTriggers_AttributeOnly): the root's OnBeforePassVariable runs first on export and its
//! OnAfterAssignVariable last on import; an empty element (`<x />`) leaves the variable as it was, a leaf runs its
//! OnAfterAssignVariable twice, a container or element with attributes once.

use al_testkit::text;

const SRC: &str = r#"
table 70 "RC Item" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
table 71 "RC Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }

xmlport 72 "RC Nested"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Rw; "RC Item")
            {
                XmlName = 'Item';
                fieldelement(Id; Rw.Id) { }
                fieldelement(Name; Rw.Name) { }
                trigger OnAfterGetRecord() begin Log += 'rw-agr(' + Format(Rw.Id) + ');'; end;
            }
            textelement(Hdr)
            {
                textattribute(HAttr)
                {
                    trigger OnBeforePassVariable() begin Log += 'hattr-bpv;'; end;
                    trigger OnAfterAssignVariable() begin Log += 'hattr-aav(' + HAttr + ');'; end;
                }
                textelement(HCode)
                {
                    trigger OnBeforePassVariable() begin Log += 'hcode-bpv;'; end;
                    trigger OnAfterAssignVariable() begin Log += 'hcode-aav(' + HCode + ');'; end;
                }
                textelement(Inner)
                {
                    textelement(InnerVal)
                    {
                        trigger OnBeforePassVariable() begin Log += 'innerval-bpv;'; end;
                        trigger OnAfterAssignVariable() begin Log += 'innerval-aav(' + InnerVal + ');'; end;
                    }
                    trigger OnBeforePassVariable() begin Log += 'inner-bpv;'; end;
                    trigger OnAfterAssignVariable() begin Log += 'inner-aav;'; end;
                }
                trigger OnBeforePassVariable() begin Log += 'hdr-bpv;'; end;
                trigger OnAfterAssignVariable() begin Log += 'hdr-aav;'; end;
            }
            trigger OnBeforePassVariable() begin Log += 'root-bpv;'; end;
            trigger OnAfterAssignVariable() begin Log += 'root-aav;'; end;
        }
    }
    var Log: Text;
    trigger OnPreXmlPort() begin HAttr := 'ha'; HCode := 'hc'; InnerVal := 'iv'; end;
    procedure GetLog(): Text begin exit(Log); end;
}

xmlport 73 "RC Attr Only"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            textelement(Endpoint)
            {
                textattribute(SchemeId)
                {
                    trigger OnAfterAssignVariable() begin Log += 'scheme-aav(' + SchemeId + ');'; end;
                }
                trigger OnAfterAssignVariable() begin Log += 'ep-aav(' + Endpoint + ');'; end;
            }
        }
    }
    var Log: Text;
    trigger OnPreXmlPort() begin Endpoint := '0088'; SchemeId := 'GLN'; end;
    procedure GetLog(): Text begin exit(Log); end;
}

codeunit 74 C
{
    local procedure Read(var B: Record "RC Blob" temporary): Text
    var InS: InStream; L: Text; R: Text;
    begin
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(L); R += L + '|'; end;
        exit(R);
    end;

    procedure Export(): Text
    var XP: XmlPort "RC Nested"; I: Record "RC Item"; B: Record "RC Blob" temporary; OutS: OutStream;
    begin
        I.Id := 1; I.Name := 'one'; I.Insert();
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(OutS);
        XP.SetDestination(OutS); XP.Export();
        exit(XP.GetLog() + '#' + Read(B));
    end;

    local procedure Import(Doc: Text): Text
    var XP: XmlPort "RC Nested"; B: Record "RC Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(OutS, TextEncoding::UTF8); OutS.WriteText(Doc);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        XP.SetSource(InS); XP.Import();
        exit(XP.GetLog());
    end;

    procedure ImportFull(): Text
    begin
        exit(Import('<Root><Item><Id>7</Id><Name>seven</Name></Item><Hdr HAttr="ha2"><HCode>hc2</HCode><Inner><InnerVal>iv2</InnerVal></Inner></Hdr></Root>'));
    end;

    procedure ImportEmptyLeaf(): Text
    begin
        exit(Import('<Root><Item><Id>8</Id><Name>eight</Name></Item><Hdr HAttr=""><HCode /><Inner><InnerVal /></Inner></Hdr></Root>'));
    end;

    procedure ImportNoScheme(): Text
    var XP: XmlPort "RC Attr Only"; B: Record "RC Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<Root><Endpoint>0100</Endpoint></Root>');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        XP.SetSource(InS);
        asserterror XP.Import();
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ImportEmptyEndpoint(): Text
    var XP: XmlPort "RC Attr Only"; B: Record "RC Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<Root><Endpoint SchemeId="X" /></Root>');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        XP.SetSource(InS); XP.Import();
        exit(XP.GetLog());
    end;
}
"#;

#[test]
fn the_root_before_pass_runs_first_on_export() {
    // ContainerTriggers_Nested 'export'.
    assert_eq!(
        text(al_testkit::run(SRC, "Export")),
        "root-bpv;rw-agr(1);hdr-bpv;hattr-bpv;hcode-bpv;inner-bpv;innerval-bpv;#<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root>|  <Item>|    <Id>1</Id>|    <Name>one</Name>|  </Item>|  <Hdr HAttr=\"ha\">|    <HCode>hc</HCode>|    <Inner>|      <InnerVal>iv</InnerVal>|    </Inner>|  </Hdr>|</Root>|"
    );
}

#[test]
fn the_root_after_assign_runs_last_on_import() {
    // ContainerTriggers_Nested 'import full'.
    assert_eq!(
        text(al_testkit::run(SRC, "ImportFull")),
        "hattr-aav(ha2);hcode-aav(hc2);innerval-aav(iv2);inner-aav;hdr-aav;root-aav;"
    );
}

#[test]
fn an_empty_leaf_keeps_its_value_and_runs_twice() {
    // ContainerTriggers_Nested 'import empty leaf'.
    assert_eq!(
        text(al_testkit::run(SRC, "ImportEmptyLeaf")),
        "hattr-aav();hcode-aav(hc);hcode-aav(hc);innerval-aav(iv);innerval-aav(iv);inner-aav;hdr-aav;root-aav;"
    );
}

#[test]
fn an_empty_element_with_an_attribute_keeps_its_value() {
    // ContainerTriggers_AttributeOnly 'import empty endpoint'.
    assert_eq!(
        text(al_testkit::run(SRC, "ImportEmptyEndpoint")),
        "scheme-aav(X);ep-aav(0088);"
    );
}

#[test]
fn a_text_attribute_of_an_attributed_element_below_the_root_is_required() {
    // ContainerTriggers_AttributeOnly 'import no scheme'.
    assert_eq!(
        text(al_testkit::run(SRC, "ImportNoScheme")),
        "Imported XML cannot validate with the schema: The required attribute 'SchemeId' is missing.|XmlPortInvalidXml"
    );
}
