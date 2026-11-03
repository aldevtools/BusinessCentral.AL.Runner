//! A parent row modified again after its child rows (no AutoReplace) imports when the table element has an OnAfterInitRecord
//! trigger: the values that trigger assigned equal the stored ones, so which side wins does not matter (corpus
//! xmlport/lt-xmlport-trigger-flow/XmlPortTriggerFlow.al:452 NoAutoReplace_NewParentTwoChildren, :512
//! NoAutoReplace_ExistingChildUnderNewParent log in OnAfterInitRecord only).

use al_testkit::{run, text};

const SRC: &str = r#"
table 69995 "XAI Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69996 "XAI Par" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Extra; Text[30]) { } } keys { key(PK; Id) { } } }
table 69997 "XAI Chi" { fields { field(1; Id; Integer) { } field(2; ParentId; Integer) { } } keys { key(PK; Id) { } } }
xmlport 69998 "XAI Port"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Par; "XAI Par")
            {
                fieldelement(Id; Par.Id) { }
                fieldelement(Name; Par.Name) { }
                trigger OnAfterInitRecord() begin Par.Extra := 'init'; end;
                tableelement(Chi; "XAI Chi")
                {
                    LinkTable = Par; LinkFields = ParentId = field(Id); MinOccurs = Zero;
                    fieldelement(CId; Chi.Id) { }
                }
            }
        }
    }
}
codeunit 69999 C
{
    procedure F(): Text
    var B: Record "XAI Blob" temporary; P: Record "XAI Par"; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Root><Par><Id>1</Id><Name>one</Name><Chi><CId>10</CId></Chi></Par></Root>');
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XAI Port", InS);
        P.Get(1);
        exit(P.Name + '/' + P.Extra);
    end;
}
"#;

#[test]
fn a_modify_after_the_children_keeps_what_the_init_trigger_assigned() {
    assert_eq!(text(run(SRC, "F")), "one/init");
}
