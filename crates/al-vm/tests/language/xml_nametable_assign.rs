//! `NsMgr.NameTable := Doc.NameTable` is the call `NsMgr.NameTable(Doc.NameTable())` (al-runner issue 6526, the
//! reporter's repro; alc accepts it). Until the corpus pin lands the path marks assumption `nametable-assign-is-call`.

use al_testkit::text;

const SRC: &str = r#"
codeunit 69863 T
{
    procedure F(): Text
    var
        Doc: XmlDocument;
        NsMgr: XmlNamespaceManager;
        Node: XmlNode;
    begin
        XmlDocument.ReadFrom('<r xmlns="urn:repro"><docNo>PSI-1001</docNo></r>', Doc);
        NsMgr.NameTable := Doc.NameTable;
        NsMgr.AddNamespace('urn', 'urn:repro');
        if Doc.SelectSingleNode('//urn:docNo', NsMgr, Node) then
            exit(Node.AsXmlElement().InnerText);
        exit('not found');
    end;
}
"#;

#[test]
fn assigning_the_name_table_of_a_namespace_manager_calls_its_setter() {
    al_builtins::assumptions::clear_hits();
    assert_eq!(text(al_testkit::run_in(SRC, "T", "F")), "PSI-1001");
    let hits: Vec<_> = al_builtins::assumptions::take_hits()
        .iter()
        .map(|a| a.info().id)
        .collect();
    assert!(hits.contains(&"nametable-assign-is-call"), "{hits:?}");
}
