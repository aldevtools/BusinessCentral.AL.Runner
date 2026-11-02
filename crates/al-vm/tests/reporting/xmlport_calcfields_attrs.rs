//! XmlPort table element `CalcFields`, `LinkTableForceInsert`, and Occurrence of the attributes of a field element
//! (xmlport/lt-y68-xmlport-shapes/Y68XmlPortShapes.al, corpus master 0d01b0cc).

const SRC: &str = r#"
table 91610 "XC Parent"
{
    fields
    {
        field(1; No; Code[10]) { }
        field(2; Lang; Text[10]) { }
        field(3; Req; Text[10]) { }
        field(4; Name; Text[30]) { }
        field(5; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum("XC Child".Amount where(ParentNo = field(No))); }
        field(6; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("XC Child" where(ParentNo = field(No))); }
    }
    keys { key(PK; No) { } }
}
table 91611 "XC Child"
{
    fields { field(1; ParentNo; Code[10]) { } field(2; LineNo; Integer) { } field(3; Amount; Decimal) { } }
    keys { key(PK; ParentNo, LineNo) { } }
}
table 91612 "XC Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
xmlport 91613 "XC Calc"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(P; "XC Parent")
            {
                CalcFields = Total;
                fieldelement(No; P.No) { }
                fieldelement(Total; P.Total) { }
                trigger OnAfterGetRecord() begin Seen += P.No + ' total=' + Format(P.Total) + ' cnt=' + Format(P.Cnt) + ';'; end;
                trigger OnBeforeInsertRecord() begin Seen += 'bir total=' + Format(P.Total) + ';'; end;
            }
        }
    }
    var Seen: Text;
    procedure GetSeen(): Text begin exit(Seen); end;
}
xmlport 91614 "XC Attr"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(P; "XC Parent")
            {
                fieldelement(No; P.No) { }
                fieldelement(Name; P.Name)
                {
                    fieldattribute(Lang; P.Lang) { Occurrence = Optional; }
                    fieldattribute(Req; P.Req) { }
                }
            }
        }
    }
}
xmlport 91615 "XC Force"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(P; "XC Parent")
            {
                fieldelement(No; P.No) { }
                tableelement(C; "XC Child")
                {
                    MinOccurs = Zero;
                    LinkTable = P;
                    LinkFields = ParentNo = field(No);
                    LinkTableForceInsert = true;
                    fieldelement(Line; C.LineNo) { }
                }
            }
        }
    }
}
codeunit 91616 C
{
    local procedure Seed()
    var
        P: Record "XC Parent";
        Ch: Record "XC Child";
    begin
        P.No := 'P1'; P.Insert();
        P.No := 'P2'; P.Insert();
        Ch.ParentNo := 'P1'; Ch.LineNo := 1; Ch.Amount := 1.5; Ch.Insert();
        Ch.ParentNo := 'P1'; Ch.LineNo := 2; Ch.Amount := 2; Ch.Insert();
    end;
    procedure CalcExport(): Text
    var
        B: Record "XC Blob" temporary;
        X: XmlPort "XC Calc";
        OutS: OutStream;
    begin
        Seed();
        B.Data.CreateOutStream(OutS);
        X.SetDestination(OutS);
        X.Export();
        exit(X.GetSeen());
    end;
    local procedure Import(Id: Integer; Content: Text; var X: XmlPort "XC Calc")
    var
        B: Record "XC Blob" temporary;
        OutS: OutStream;
        InS: InStream;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText(Content);
        B.Data.CreateInStream(InS);
        X.SetSource(InS);
        X.Import();
    end;
    procedure CalcImport(): Text
    var
        Ch: Record "XC Child";
        X: XmlPort "XC Calc";
    begin
        Ch.ParentNo := 'P1'; Ch.LineNo := 1; Ch.Amount := 4; Ch.Insert();
        Import(0, '<Root><P><No>P1</No><Total>9</Total></P></Root>', X);
        exit(X.GetSeen());
    end;
    local procedure Stream(Content: Text; var InS: InStream; var B: Record "XC Blob" temporary)
    var
        OutS: OutStream;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText(Content);
        B.Data.CreateInStream(InS);
    end;
    procedure AttrImport(Content: Text): Text
    var
        B: Record "XC Blob" temporary;
        P: Record "XC Parent";
        InS: InStream;
    begin
        Stream(Content, InS, B);
        Xmlport.Import(Xmlport::"XC Attr", InS);
        P.FindFirst();
        exit(P.Name + '|' + P.Lang + '|' + P.Req);
    end;
    procedure LangMissing(): Text begin exit(AttrImport('<Root><P><No>A</No><Name Req="r">x</Name></P></Root>')); end;
    procedure ReqMissing(): Text begin exit(AttrImport('<Root><P><No>A</No><Name Lang="en">x</Name></P></Root>')); end;
    procedure EmptyElement(): Text begin exit(AttrImport('<Root><P><No>A</No><Name Lang="de" Req="b" /></P></Root>')); end;
    procedure AttrExport(): Text
    var
        B: Record "XC Blob" temporary;
        P: Record "XC Parent";
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Result: Text;
    begin
        P.No := 'A'; P.Name := 'filled'; P.Lang := 'en'; P.Req := 'r'; P.Insert();
        P.No := 'B'; P.Name := 'blank'; P.Lang := ''; P.Req := ''; P.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XC Attr", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            Result += Line.Trim() + '|';
        end;
        exit(Result);
    end;
    procedure ForceImport(): Text
    var
        B: Record "XC Blob" temporary;
        P: Record "XC Parent";
        Ch: Record "XC Child";
        InS: InStream;
    begin
        Stream('<Root><P><No>P1</No><C><Line>1</Line></C><C><Line>2</Line></C></P><P><No>P2</No></P></Root>', InS, B);
        Xmlport.Import(Xmlport::"XC Force", InS);
        exit(Format(P.Count()) + '/' + Format(Ch.Count()));
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn calcfields_calculates_the_listed_flowfields_before_onaftergetrecord() {
    // Y68XmlPortShapes.al:567-582 Q4_CalcFields_Export_RealTable `total log`.
    assert_eq!(run("CalcExport"), "P1 total=3.5 cnt=0;P2 total=0 cnt=0;");
}

#[test]
fn calcfields_changes_nothing_on_import() {
    // Y68XmlPortShapes.al:623-640 Q4_CalcFields_Import.
    assert_eq!(run("CalcImport"), "bir total=0;");
}

#[test]
fn an_optional_field_attribute_of_a_field_element_may_be_missing() {
    // Y68XmlPortShapes.al:642-692 `lang missing`, `empty element`.
    assert_eq!(run("LangMissing"), "x||r");
    assert_eq!(run("EmptyElement"), "|de|b");
}

#[test]
fn a_required_field_attribute_of_a_field_element_must_be_present() {
    // Y68XmlPortShapes.al:642-692 `req missing`.
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(SRC, "C", "ReqMissing")),
        "Imported XML cannot validate with the schema: The required attribute 'Req' is missing."
    );
}

#[test]
fn a_blank_optional_attribute_is_left_out_of_the_export() {
    // Y68XmlPortShapes.al:694-713 Q6_FieldElement_Attributes_Export.
    assert_eq!(
        run("AttrExport"),
        "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root>|<P>|<No>A</No>|<Name Lang=\"en\" Req=\"r\">filled</Name>|</P>|<P>|<No>B</No>|<Name Req=\"\">blank</Name>|</P>|</Root>|"
    );
}

#[test]
fn link_table_force_insert_imports_as_without_it() {
    // Y68XmlPortShapes.al:715-757 Q6_LinkTableForceInsert_Import.
    assert_eq!(run("ForceImport"), "2/2");
}
