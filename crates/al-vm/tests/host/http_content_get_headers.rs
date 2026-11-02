//! `HttpContent.GetHeaders` on the content of a request that never got content, with the result used and as a statement.
//! Every literal is what real BC answered in the corpus: network/lt-y62-content-headers/Y62ContentHeaders.al (codeunit 93264,
//! writer Y62).

use al_testkit::text;

const SRC: &str = r#"
codeunit 50191 C
{
    local procedure NewRequest(var Req: HttpRequestMessage)
    begin
        Req.SetRequestUri('https://example.invalid/x');
        Req.Method('GET');
    end;

    local procedure HeadersAsExit(var Req: HttpRequestMessage; var H: HttpHeaders): Boolean
    begin
        exit(Req.Content.GetHeaders(H));
    end;

    local procedure HeadersAsIf(var Req: HttpRequestMessage; var H: HttpHeaders): Text
    begin
        if Req.Content.GetHeaders(H) then
            exit('true');
        exit('false');
    end;

    [TryFunction]
    local procedure HeadersAsStatement(var Req: HttpRequestMessage; var H: HttpHeaders)
    begin
        Req.Content.GetHeaders(H);
    end;

    local procedure State(var H: HttpHeaders): Text
    begin
        exit('|keys=' + Format(H.Keys().Count()) + '|seed=' + Format(H.Contains('X-Seed')));
    end;

    local procedure Seeded(var H: HttpHeaders)
    begin
        Clear(H);
        H.Add('X-Seed', '1');
    end;

    local procedure Statement(var Req: HttpRequestMessage; var H: HttpHeaders): Text
    begin
        Seeded(H);
        if HeadersAsStatement(Req, H) then
            exit('ok' + State(H));
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode() + State(H));
    end;

    procedure ResultUsed(): Text
    var
        Req1: HttpRequestMessage;
        Req2: HttpRequestMessage;
        H: HttpHeaders;
        Txt: Text;
        Out: Text;
    begin
        NewRequest(Req1);
        NewRequest(Req2);
        Seeded(H);
        Out := Format(HeadersAsExit(Req1, H)) + State(H);
        Seeded(H);
        Out += ';' + HeadersAsIf(Req2, H) + State(H);
        Out += ';' + Statement(Req1, H);
        Txt := 'preset';
        Out += ';' + Format(Req1.Content.ReadAs(Txt)) + '|' + Txt;
        exit(Out);
    end;
}
"#;

/// Y62ContentHeaders.al:GetHeaders_ResultUsed_FreshRequest and GetHeaders_ResultUsed_ThenStatementAndReadAs: with the result used
/// the call answers No and leaves H as it was; the content stays uninitialized, so a later statement call still raises.
#[test]
fn get_headers_with_the_result_used_answers_false_on_a_request_without_content() {
    assert_eq!(
        text(al_testkit::run(SRC, "ResultUsed")),
        "No|keys=1|seed=Yes;false|keys=1|seed=Yes;\
         error:Microsoft.Dynamics.Nav.Runtime.NavHttpContent variable not initialized.|NotInitialized|keys=1|seed=Yes;No|preset"
    );
}
