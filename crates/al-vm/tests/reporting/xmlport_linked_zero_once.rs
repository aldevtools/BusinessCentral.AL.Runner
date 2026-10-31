//! A table element linked with LinkTable and LinkFields, MinOccurs = Zero and MaxOccurs = Once, on export: a parent without a
//! child row writes no child element, of two child rows only the lowest key is written, and OnAfterGetRecord runs for that row
//! only (misc/lt-y14-triage/Y14XmlPortLink.al:136 LinkedZeroOnce_ZeroOneTwoChildren, corpus master).

use al_testkit::{run, text};

const SRC: &str = r#"
table 69980 "LZ Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69981 "LZ Parent" { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
table 69982 "LZ Child" { fields { field(1; ParentNo; Integer) { } field(2; Seq; Integer) { } } keys { key(PK; ParentNo, Seq) { } } }
xmlport 69983 "LZ Port"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Par; "LZ Parent")
            {
                XmlName = 'Parent';
                fieldattribute(No; Par.No) { }
                tableelement(Chi; "LZ Child")
                {
                    XmlName = 'Child'; LinkTable = Par; LinkFields = ParentNo = field(No); MinOccurs = Zero; MaxOccurs = Once;
                    fieldelement(Seq; Chi.Seq) { }
                    trigger OnPreXmlItem() begin Log += 'pre(' + Format(Par.No) + ');'; end;
                    trigger OnAfterGetRecord() begin Log += 'agr(' + Format(Chi.ParentNo) + '/' + Format(Chi.Seq) + ');'; end;
                }
            }
        }
    }
    var Log: Text;
    procedure GetLog(): Text begin exit(Log); end;
}
codeunit 69984 C
{
    procedure Export(): Text
    var B: Record "LZ Blob" temporary; P: Record "LZ Parent"; R: Record "LZ Child"; X: XmlPort "LZ Port"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        P.No := 3; P.Insert(); P.No := 1; P.Insert(); P.No := 2; P.Insert();
        R.ParentNo := 2; R.Seq := 10; R.Insert();
        R.ParentNo := 3; R.Seq := 20; R.Insert();
        R.ParentNo := 3; R.Seq := 10; R.Insert();
        B.Data.CreateOutStream(OutS);
        X.SetDestination(OutS);
        X.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line.Trim(); end;
        exit(T + '|' + X.GetLog());
    end;
}
"#;

#[test]
fn a_linked_zero_once_child_writes_the_lowest_key_and_nothing_without_rows() {
    let out = text(run(SRC, "Export"));
    assert!(
        out.ends_with(
            "<Root><Parent No=\"1\" /><Parent No=\"2\"><Child><Seq>10</Seq></Child></Parent><Parent No=\"3\"><Child><Seq>10</Seq></Child></Parent></Root>|pre(1);pre(2);agr(2/10);pre(3);agr(3/10);"
        ),
        "{out}"
    );
}
