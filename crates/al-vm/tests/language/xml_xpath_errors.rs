//! The errors of SelectNodes and SelectSingleNode with a path that is no XPath, statement form (corpus
//! xml/XmlXPathNamespaces.al, xml/lt-xml-cdata-comment-pi-text/P90*Read.al).

use al_testkit::text;

fn errors(paths: &[&str]) -> String {
    let mut body = String::new();
    for p in paths {
        body.push_str(&format!(
            "asserterror Doc.SelectNodes('{}', L); S += GetLastErrorText() + '/' + GetLastErrorCode() + '|';\n",
            p.replace('\'', "''")
        ));
    }
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var Doc: XmlDocument; L: XmlNodeList; S: Text; begin
         XmlDocument.ReadFrom('<a><b/></a>', Doc); {body} exit(S); end; }}"
    );
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

const NODE_SET: &str = "Expression must evaluate to a node-set./Xml:XPath|";
const NS: &str =
    "Namespace Manager or XsltContext needed. This query has a prefix, variable, or user-defined function./Xml:XPath|";

#[test]
fn a_missing_step_or_expression_raises_the_node_set_error() {
    // xml/XmlXPathNamespaces.al:150 SelectNodes_Invalid_XPath `bracket`, `empty`, `slashes`, `open_paren`.
    assert_eq!(errors(&["/a/[", "", "///", "foo("]), NODE_SET.repeat(4));
}

#[test]
fn trailing_tokens_and_a_bad_character_raise_the_invalid_token_error() {
    // xml/XmlXPathNamespaces.al:150 `two_names`, :640 SelectNodes_Backslash_Path.
    assert_eq!(
        errors(&["a b", "\\\\contract"]),
        "'a b' has an invalid token./Xml:XPath|'\\\\contract' has an invalid token./Xml:XPath|"
    );
}

#[test]
fn a_prefix_or_a_variable_without_a_manager_raises_the_namespace_manager_error() {
    // xml/XmlXPathNamespaces.al:314 `prefix_without_manager`; P90CDataRead.al:1239 `$v`.
    assert_eq!(errors(&["//p:b", "$v"]), NS.repeat(2));
}

#[test]
fn a_prefix_star_selects_every_element_of_its_namespace_and_xml_is_a_known_prefix() {
    // xml/XmlXPathNamespaces.al:344 `x_star`, :408 `lookup_xml`, `lookup_empty`.
    let src = "codeunit 50100 C { procedure F(): Text var Doc: XmlDocument; M: XmlNamespaceManager; L: XmlNodeList; U: Text; S: Text; begin
        XmlDocument.ReadFrom('<a xmlns=\"urn:d\" xmlns:p=\"urn:p\"><b/><p:b/><p:c/></a>', Doc); M.AddNamespace('x', 'urn:p');
        Doc.SelectNodes('//x:*', M, L); S := Format(L.Count()) + '|' + Format(M.LookupNamespace('xml', U)) + U;
        S += '|' + Format(M.LookupNamespace('', U)) + '[' + U + ']|' + Format(M.HasNamespace('xml')); exit(S); end; }";
    let r = text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v));
    assert_eq!(r, "2|Yeshttp://www.w3.org/XML/1998/namespace|Yes[]|No");
}

#[test]
fn a_processing_instruction_gets_a_new_target_and_data() {
    // xml/xml/P90PIRead.al:2215 PI_GetSetTargetAndData.
    let src = "codeunit 50100 C { procedure F(): Text var P: XmlProcessingInstruction; S: Text; T: Text; begin
        P := XmlProcessingInstruction.Create('t', 'data1'); P.SetTarget('t2');
        asserterror P.SetTarget(''); S := GetLastErrorCode(); asserterror P.SetTarget('a b'); S += '|' + GetLastErrorCode();
        P.SetData(''); P.WriteTo(T); exit(S + '|' + T); end; }";
    let r = text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v));
    assert_eq!(r, "Argument|Xml:Xml|<?t2?>");
}
