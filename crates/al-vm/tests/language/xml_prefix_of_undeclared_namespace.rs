//! XmlElement.GetPrefixOfNamespace of a namespace no element in scope declares answers Yes with an empty prefix, and the
//! reserved namespaces answer `xml` and `xmlns` (xml/lt-tf-xml-ns/XmlNsPrefixes.al:55-108 Prefix_OfUndeclaredNamespace).

use al_testkit::text;

#[test]
fn the_prefix_of_an_undeclared_namespace_is_empty_and_found() {
    let src = "codeunit 50100 C {
        local procedure P(E: XmlElement; Ns: Text): Text var Pf: Text; R: Boolean;
        begin Pf := 'unset'; R := E.GetPrefixOfNamespace(Ns, Pf); exit(Format(R) + '[' + Pf + ']'); end;
        procedure F(): Text var Doc: XmlDocument; Root: XmlElement; Child: XmlElement; N: XmlNode; R: Text;
        begin
            XmlDocument.ReadFrom('<a xmlns:p=\"urn:p\"><b/></a>', Doc);
            Doc.GetRoot(Root);
            Root.SelectSingleNode('b', N);
            Child := N.AsXmlElement();
            R := P(Root, 'urn:p') + P(Root, 'urn:zz') + P(Root, '') + P(Root, 'http://www.w3.org/XML/1998/namespace')
                + P(Root, 'http://www.w3.org/2000/xmlns/') + P(Child, 'urn:p') + P(Child, 'urn:zz');
            Root := XmlElement.Create('r');
            exit(R + P(Root, 'urn:zz'));
        end; }";
    let r = text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v));
    assert_eq!(r, "Yes[p]Yes[]Yes[]Yes[xml]Yes[xmlns]Yes[p]Yes[]Yes[]");
}

/// GetNamespaceOfPrefix of the reserved prefixes `xml` and `xmlns` answers Yes with their namespaces; an unknown or the empty
/// prefix answers No and leaves the variable (XmlNsPrefixes.al:76-81, :98-107).
#[test]
fn the_reserved_prefixes_have_their_namespaces() {
    let src = "codeunit 50100 C {
        local procedure N(E: XmlElement; P: Text): Text var Ns: Text; R: Boolean;
        begin Ns := 'unset'; R := E.GetNamespaceOfPrefix(P, Ns); exit(Format(R) + '[' + Ns + ']'); end;
        procedure F(): Text var Doc: XmlDocument; Root: XmlElement;
        begin
            XmlDocument.ReadFrom('<a xmlns:p=\"urn:p\"><b/></a>', Doc);
            Doc.GetRoot(Root);
            exit(N(Root, 'p') + N(Root, 'zz') + N(Root, '') + N(Root, 'xml') + N(Root, 'xmlns'));
        end; }";
    let r = text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v));
    assert_eq!(
        r,
        "Yes[urn:p]No[unset]No[unset]Yes[http://www.w3.org/XML/1998/namespace]Yes[http://www.w3.org/2000/xmlns/]"
    );
}
