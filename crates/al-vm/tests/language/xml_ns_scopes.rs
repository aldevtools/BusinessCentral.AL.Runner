//! XmlNamespaceManager scopes, RemoveNamespace and LookupPrefix (corpus xml/lt-xml-remaining-node-methods/P104NsManager.al).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69799 C
{
    local procedure Lk(var M: XmlNamespaceManager; U: Text): Text
    var R: Text; F: Boolean;
    begin
        R := 'unset';
        F := M.LookupPrefix(U, R);
        exit(Format(F) + ':[' + R + '];');
    end;

    local procedure Has(var M: XmlNamespaceManager; P: Text): Text
    var U: Text; F: Boolean;
    begin
        F := M.LookupNamespace(P, U);
        exit(Format(M.HasNamespace(P)) + ':' + Format(F) + ':[' + U + '];');
    end;

    procedure Prefixes(): Text
    var M: XmlNamespaceManager; T: Text;
    begin
        T += Lk(M, 'urn:p');
        M.AddNamespace('p', 'urn:p');
        T += Lk(M, 'urn:p') + Lk(M, 'URN:P') + Lk(M, '') + Lk(M, 'http://www.w3.org/2000/xmlns/');
        M.AddNamespace('q', 'urn:p');
        T += Lk(M, 'urn:p');
        M.AddNamespace('r', '');
        T += Lk(M, '');
        M.AddNamespace('p', 'urn:other');
        T += Lk(M, 'urn:p') + Lk(M, 'urn:other');
        exit(T);
    end;

    procedure Scopes(): Text
    var M: XmlNamespaceManager; T: Text;
    begin
        M.PopScope();
        M.AddNamespace('p', 'urn:outer');
        M.PushScope();
        T += Has(M, 'p');
        M.AddNamespace('q', 'urn:inner');
        M.AddNamespace('p', 'urn:shadow');
        T += Has(M, 'p') + Lk(M, 'urn:outer') + Lk(M, 'urn:shadow');
        M.RemoveNamespace('q', 'urn:inner');
        T += Has(M, 'q');
        M.PopScope();
        T += Has(M, 'q') + Has(M, 'p') + Lk(M, 'urn:outer');
        M.RemoveNamespace('p', 'urn:wrong');
        M.RemoveNamespace('xml', 'http://www.w3.org/XML/1998/namespace');
        T += Has(M, 'p') + Has(M, 'xml');
        M.RemoveNamespace('p', 'urn:outer');
        T += Has(M, 'p') + Lk(M, 'urn:outer');
        exit(T);
    end;
}
"#;

#[test]
fn lookup_prefix_gives_the_newest_prefix_still_standing_for_the_namespace() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Prefixes")),
        "No:[unset];Yes:[p];No:[unset];Yes:[];Yes:[xmlns];Yes:[q];Yes:[r];Yes:[q];Yes:[p];"
    );
}

#[test]
fn scopes_shadow_and_pop_and_remove_works_on_the_current_scope() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Scopes")),
        "No:Yes:[urn:outer];Yes:Yes:[urn:shadow];No:[unset];Yes:[p];No:No:[];No:No:[];Yes:Yes:[urn:outer];Yes:[p];\
         Yes:Yes:[urn:outer];No:Yes:[http://www.w3.org/XML/1998/namespace];No:No:[];No:[unset];"
    );
}
