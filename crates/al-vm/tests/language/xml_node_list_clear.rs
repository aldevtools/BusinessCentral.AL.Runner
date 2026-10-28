//! Clear of an XmlNodeList (corpus branch al-runtime-rs-staging-pub-tp, xml/XmlNodes.al TP_NodeList_Clear): the list is empty
//! after it, a copy taken before keeps its nodes, and the variable can be filled again by SelectNodes.

use al_testkit::text;

fn run(body: &str) -> String {
    let src = format!("codeunit 50100 C {{ {body} }}");
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

#[test]
fn a_cleared_node_list_is_empty_and_a_copy_keeps_its_nodes() {
    let r = run(
        "procedure F(): Text var Doc: XmlDocument; L: XmlNodeList; Copy: XmlNodeList; Fresh: XmlNodeList; N: XmlNode; R: Text;
        begin
            XmlDocument.ReadFrom('<a><b>1</b><b>2</b><c>3</c></a>', Doc);
            Clear(Fresh);
            R := Format(Fresh.Count()) + ';';
            Doc.SelectNodes('//b', L);
            Copy := L;
            Clear(L);
            R += Format(L.Count()) + ';' + Format(Copy.Count()) + ';' + Format(L.Get(1, N)) + ';';
            foreach N in L do
                R += 'x';
            Doc.SelectNodes('//c', L);
            L.Get(1, N);
            R += Format(L.Count()) + ';' + N.AsXmlElement().InnerText();
            exit(R);
        end;",
    );
    assert_eq!(r, "0;0;2;No;1;3");
}
