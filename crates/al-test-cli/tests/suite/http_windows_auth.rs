//! `HttpClient.UseDefaultNetworkWindowsAuthentication` and `UseWindowsAuthentication` (Text and SecretText): each call answers
//! Yes, also twice and with blank texts, and a request after it that a handler answers is answered as before
//! (al-language-onprem/network/HttpClientAuthConfig.al:19-71, :109-136). A request no handler answers is refused (not pinned).

use al_testkit::outcomes_in;

const SRC: &str = r#"
codeunit 50150 T
{
    Subtype = Test;
    TestHttpRequestPolicy = AllowOutboundFromHandler;

    var
        Calls: Integer;

    [HttpClientHandler]
    procedure Handler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Calls += 1;
        Response.HttpStatusCode := 200;
        Response.Content.WriteFrom('reply ' + Format(Calls));
        exit(false);
    end;

    local procedure Sec(Value: Text) Result: SecretText
    begin
        Result := Value;
    end;

    local procedure Request(var Client: HttpClient): Text
    var
        Resp: HttpResponseMessage;
        Body: Text;
    begin
        Client.Get('https://example.invalid/after-config', Resp);
        Resp.Content.ReadAs(Body);
        exit(Format(Resp.HttpStatusCode) + ' ' + Body);
    end;

    [Test]
    procedure Results()
    var
        Client: HttpClient;
        R: Text;
    begin
        R := Format(Client.UseDefaultNetworkWindowsAuthentication()) + Format(Client.UseDefaultNetworkWindowsAuthentication());
        R += Format(Client.UseWindowsAuthentication('user', 'password', 'DOMAIN')) + Format(Client.UseWindowsAuthentication('', ''));
        R += Format(Client.UseWindowsAuthentication(Sec('user'), Sec('password'), Sec('D'))) + Format(Client.UseWindowsAuthentication(Sec(''), Sec('')));
        if R <> 'YesYesYesYesYesYes' then
            Error(R);
    end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure RequestAfterIt()
    var
        Client: HttpClient;
        R: Text;
    begin
        Calls := 0;
        Client.UseDefaultNetworkWindowsAuthentication();
        R := Request(Client);
        Client.UseWindowsAuthentication('user', 'password', 'DOMAIN');
        R += '|' + Request(Client);
        Client.UseWindowsAuthentication(Sec('user'), Sec('password'));
        R += '|' + Request(Client);
        if R <> '200 reply 1|200 reply 2|200 reply 3' then
            Error(R);
    end;

    [Test]
    procedure RequestNoHandlerAnswers()
    var
        Client: HttpClient;
    begin
        Client.UseDefaultNetworkWindowsAuthentication();
        Request(Client);
    end;
}
"#;

#[test]
fn the_calls_answer_yes_and_a_handler_answers_the_request_after_them() {
    let o = outcomes_in(SRC, "T");
    let get = |n: &str| format!("{:?}", o.iter().find(|(t, _)| t == n).unwrap().1);
    assert!(get("Results").starts_with("Pass"), "{o:?}");
    assert!(get("RequestAfterIt").starts_with("Pass"), "{o:?}");
    assert!(get("RequestNoHandlerAnswers").contains("not pinned"), "{o:?}");
}
