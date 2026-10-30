//! HttpHeaders.GetSecretValues (corpus xml/lt-xml-attr-doc-http-members/P97HdrSecretValues.al) and
//! HttpRequestMessage.SetSecretRequestUri / GetSecretRequestUri (P97ReqSecretUri.al).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69798 C
{
    local procedure S(V: Text): SecretText
    var X: SecretText;
    begin
        X := SecretText.SecretStrSubstNo(V);
        exit(X);
    end;

    local procedure Arr(var H: HttpHeaders; K: Text): Text
    var V: array[3] of SecretText; F: Boolean;
    begin
        F := H.GetSecretValues(K, V);
        exit(Format(F) + '|' + Format(V[1].IsEmpty()) + '|' + Format(V[2].IsEmpty()) + '|' + Format(V[3].IsEmpty()) + ';');
    end;

    local procedure Lst(var H: HttpHeaders; K: Text; var L: List of [SecretText]): Text
    var F: Boolean; V: SecretText; T: Text;
    begin
        F := H.GetSecretValues(K, L);
        foreach V in L do
            T += Format(V.IsEmpty()) + ',';
        exit(Format(F) + '|' + Format(L.Count()) + '|' + T + ';');
    end;

    procedure Values(): Text
    var Req: HttpRequestMessage; H: HttpHeaders; U: HttpHeaders; L: List of [SecretText]; L2: List of [SecretText]; T: Text;
    begin
        Req.GetHeaders(H);
        H.Add('X-Sec', S('s1'));
        H.Add('X-Sec2', S('a'));
        H.Add('X-Sec2', S('b'));
        H.Add('X-Sec4', S('a'));
        H.Add('X-Sec4', S('b'));
        H.Add('X-Sec4', S('c'));
        H.Add('X-Sec4', S('d'));
        H.Add('X-Plain', 'p1');
        H.Add('X-Empty', S(''));
        T += Arr(H, 'X-Sec') + Arr(H, 'x-sec2') + Arr(H, 'X-Sec4') + Arr(H, 'X-Plain') + Arr(H, 'X-Missing') + Arr(H, '');
        T += Lst(H, 'X-Empty', L);
        L2.Add(S('pre'));
        T += Lst(H, 'X-Sec', L2) + Lst(H, 'X-Missing', L2);
        T += Arr(U, 'X-Sec');
        exit(T);
    end;

    local procedure TrySecret(var Req: HttpRequestMessage; V: Text): Text
    var Ok: Boolean;
    begin
        Ok := Req.SetSecretRequestUri(S(V));
        exit(Format(Ok) + '|' + Format(Req.GetSecretRequestUri().IsEmpty()) + '|' + Req.GetRequestUri() + ';');
    end;

    procedure Uri(): Text
    var Req: HttpRequestMessage; Req2: HttpRequestMessage; T: Text;
    begin
        T += Format(Req.GetSecretRequestUri().IsEmpty()) + ';';
        T += TrySecret(Req, '') + TrySecret(Req, 'not a uri') + TrySecret(Req, '/relative/path');
        T += TrySecret(Req, 'https://example.invalid/a?token=1') + TrySecret(Req, 'ftp://example.invalid/file');
        Req2.Method('POST');
        Req2.SetRequestUri('https://example.invalid/b');
        T += Format(Req2.GetSecretRequestUri().IsEmpty()) + ';';
        T += TrySecret(Req2, 'https://example.invalid/c');
        Req2.SetRequestUri('https://example.invalid/d');
        T += Req2.GetRequestUri() + ';' + Req2.Method();
        exit(T);
    end;
}
"#;

#[test]
fn get_secret_values_fills_an_array_or_appends_to_a_list() {
    // P97HdrSecretValues.al:GetSecretValues_Array, _List, _ListIsClearedOrAppended, _UninitialisedHeaders.
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Values")),
        "Yes|No|Yes|Yes;Yes|No|No|Yes;No|Yes|Yes|Yes;No|Yes|Yes|Yes;No|Yes|Yes|Yes;No|Yes|Yes|Yes;\
         Yes|1|Yes,;Yes|2|No,No,;No|2|No,No,;No|Yes|Yes|Yes;"
    );
}

#[test]
fn a_secret_request_uri_hides_the_plain_one() {
    // P97ReqSecretUri.al:SecretUri_RoundTrip, PlainUri_ThenSecretGetter, SecretUri_InvalidValues, SecretUri_Method_Unchanged.
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Uri")),
        "Yes;No|Yes|;No|Yes|;No|Yes|;Yes|No|;Yes|No|;No;Yes|No|;https://example.invalid/d;POST"
    );
}
