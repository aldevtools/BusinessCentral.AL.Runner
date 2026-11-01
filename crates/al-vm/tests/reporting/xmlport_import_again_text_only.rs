//! A second Import on the same XmlPort instance whose schema has no table element reads the new source again and keeps the
//! instance's globals (al-runner issue 6657): real BC values from corpus writer Y66
//! (xmlport/lt-y66-xmlport-var-members/Y66XmlPortVarMembers.al:235-277 `got = a;a;`, not on corpus master yet). With a table
//! element the second Import reads nothing (xmlport_export_again.rs, QrXpContainerOccurs.al TT_ImportTwice_*).

use al_testkit::text;

const SRC: &str = r#"
table 50100 Buf { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
xmlport 50101 Imp
{
    Format = Xml;
    Direction = Import;
    UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            textelement(N)
            {
                MinOccurs = Zero;
                MaxOccurs = Unbounded;
                trigger OnAfterAssignVariable() begin Got += N + ';'; end;
            }
        }
    }
    var Got: Text;
    procedure GetGot(): Text begin exit(Got); end;
}
codeunit 50102 C
{
    procedure Twice(): Text
    var B: Record Buf temporary; P: XmlPort Imp; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Root><N>a</N></Root>');
        B.Data.CreateInStream(InS);
        P.SetSource(InS);
        P.Import();
        B.Data.CreateInStream(InS);
        P.SetSource(InS);
        P.Import();
        exit(P.GetGot());
    end;
}
"#;

#[test]
fn a_second_import_without_a_table_element_reads_again_and_keeps_globals() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Twice")), "a;a;");
}
