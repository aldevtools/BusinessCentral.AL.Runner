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

// The assignment form of the other members with one form that takes one value is the call too (corpus writer Y29 on
// the SaaS sandbox, record/ProbePropertyAssignmentForms.al on branch al-runtime-rs-staging-y29: Y29_TextBuilder_AssignForms,
// Y29_Collections_Text_Sleep_AssignForms, Y29_XmlNodes_AssignForms, Y29_HttpClient_AssignForms, Y29_HttpRequestMessage_AssignForms).
const OTHERS: &str = r#"
codeunit 69864 T
{
    procedure F(): Text
    var
        B: TextBuilder;
        L: List of [Integer];
        D: Dictionary of [Integer, Text];
        E: XmlElement;
        C: HttpClient;
        Txt: Text;
        R: HttpRequestMessage;
        Dlg: Dialog;
    begin
        R.SetRequestUri := 'https://example.com/y29?b=1';
        Dlg.HideSubsequentDialogs := true;
        B.Append := 'abc';
        B.Append := 'def';
        L.Add := 5;
        L.Add := 6;
        D.Add(1, 'x');
        D.Remove := 1;
        E := XmlElement.Create('e');
        E.Add := 'txt-y29';
        C.SetBaseAddress := 'https://example.com/y29/';
        Txt := 'abc';
        Txt.Contains := 'b';
        exit(B.ToText() + '|' + Format(L.Count) + '|' + Format(L.Get(2)) + '|' + Format(D.ContainsKey(1)) + '|' + E.InnerText + '|' + C.GetBaseAddress() + '|' + Txt + '|' + R.GetRequestUri());
    end;
}
"#;

#[test]
fn assigning_a_one_value_method_calls_it() {
    assert_eq!(
        text(al_testkit::run_in(OTHERS, "T", "F")),
        "abcdef|2|6|No|txt-y29|https://example.com/y29/|abc|https://example.com/y29?b=1"
    );
}
