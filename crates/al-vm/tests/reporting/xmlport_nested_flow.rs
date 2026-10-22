//! A table element nested in another, linked by LinkTable, with MinOccurs = Zero: the trigger order BC runs
//! (corpus xmlport/lt-xmlport-trigger-flow/XmlPortTriggerFlow.al, tests :293-399).

use al_testkit::run;

const SRC: &str = r#"
table 69960 "NF Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69961 "NF Par" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
table 69962 "NF Chi" { fields { field(1; Id; Integer) { } field(2; ParentId; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69963 "NF Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(T: Text) begin Entries += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
xmlport 69964 "NF Export"
{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NF Par")
            {
                fieldelement(Id; Par.Id) { }
                tableelement(Chi; "NF Chi")
                {
                    LinkTable = Par; LinkFields = ParentId = field(Id); MinOccurs = Zero;
                    fieldelement(CId; Chi.Id) { trigger OnBeforePassField() begin L.Add('Chi.BeforePassField=' + Format(Chi.Id)); end; }
                    trigger OnPreXmlItem() begin L.Add('Chi.Pre count=' + Format(Chi.Count())); end;
                }
            }
        }
    }
    var L: Codeunit "NF Log";
}
xmlport 69965 "NF Import"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NF Par")
            {
                AutoReplace = true;
                fieldelement(Id; Par.Id) { }
                fieldelement(Name; Par.Name) { }
                tableelement(Chi; "NF Chi")
                {
                    LinkTable = Par; LinkFields = ParentId = field(Id); MinOccurs = Zero; AutoReplace = true;
                    fieldelement(CId; Chi.Id) { }
                    trigger OnBeforeInsertRecord() begin L.Add('Chi.Ins=' + Format(Chi.Id)); end;
                }
                trigger OnBeforeInsertRecord() begin L.Add('Par.Ins=' + Format(Par.Id) + '/' + Par.Name); end;
                trigger OnBeforeModifyRecord() begin L.Add('Par.Mod=' + Format(Par.Id) + '/' + Par.Name); end;
                trigger OnAfterModifyRecord() begin L.Add('Par.AfterMod=' + Format(Par.Id)); end;
            }
        }
    }
    var L: Codeunit "NF Log";
}
codeunit 69966 C
{
    procedure Seed()
    var P: Record "NF Par"; Ch: Record "NF Chi";
    begin
        P.Id := 1; P.Insert(); P.Id := 2; P.Insert();
        Ch.Id := 10; Ch.ParentId := 1; Ch.Insert();
        Ch.Id := 11; Ch.ParentId := 1; Ch.Insert();
        Ch.Id := 20; Ch.ParentId := 2; Ch.Insert();
    end;

    procedure ExportRows(): Text
    begin
        Seed();
        exit(ExportEmpty());
    end;

    procedure ExportEmpty(): Text
    var B: Record "NF Blob" temporary; L: Codeunit "NF Log"; OutS: OutStream;
    begin
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"NF Export", OutS);
        exit(L.Take());
    end;

    procedure Import(): Text
    var B: Record "NF Blob" temporary; L: Codeunit "NF Log"; OutS: OutStream; InS: InStream;
    begin
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Root><Par><Id>1</Id><Name>one</Name><Chi><CId>10</CId></Chi><Chi><CId>11</CId></Chi></Par><Par><Id>2</Id><Name>two</Name></Par></Root>');
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"NF Import", InS);
        exit(L.Take());
    end;
}
"#;

#[test]
fn the_link_filter_is_set_before_the_child_on_pre_xml_item() {
    // Export_TriggerOrder_ParentWithNestedChild (:325): `Chi.PreXmlItem count=2` for parent 1, `count=1` for parent 2.
    assert_eq!(
        al_testkit::text(run(SRC, "ExportRows")),
        "Chi.Pre count=2;Chi.BeforePassField=10;Chi.BeforePassField=11;Chi.Pre count=1;Chi.BeforePassField=20;"
    );
}

#[test]
fn an_empty_optional_child_passes_no_field() {
    // Export_TriggerOrder_EmptyTables (:353): `Chi.PreXmlItem count=0` and no `Chi.Id.BeforePassField`.
    assert_eq!(al_testkit::text(run(SRC, "ExportEmpty")), "Chi.Pre count=0;");
}

#[test]
fn an_auto_replace_parent_is_modified_again_after_its_child_rows() {
    // Import_TriggerOrder_NewRows (:374): parent 1 is inserted before its child rows and modified after them; parent 2, without
    // child rows, is only inserted.
    assert_eq!(
        al_testkit::text(run(SRC, "Import")),
        "Par.Ins=1/one;Chi.Ins=10;Chi.Ins=11;Par.Mod=1/one;Par.AfterMod=1;Par.Ins=2/two;"
    );
}
