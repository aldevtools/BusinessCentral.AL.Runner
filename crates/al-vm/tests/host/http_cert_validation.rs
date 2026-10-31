//! `HttpClient.UseServerCertificateValidation`. Pinned by network/lt-cz-http-leads/CzHttpNoNetwork.al (codeunit 85600,
//! BC 27, 28, 29): `false` detaches an HttpHeaders taken earlier from DefaultRequestHeaders, `true` keeps the link.
//! network/lt-httpclient-property-assign/Y6HttpSettingChanges.al (codeunit 91311, corpus master): the first call that leaves
//! the defaults (validation `false`, cookies `true`) rebuilds the client: default timeout, no base address, new default headers;
//! a call that keeps a default and every later call change nothing; the call answers its argument.
//! What is not pinned is refused: a setting call on a client cleared after its rebuild, and a request of such a client that
//! no handler answers outside BlockOutboundRequests.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{run, text};

fn refused(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(m)) => m,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

const SRC: &str = r#"
codeunit 50191 C
{
    // CzHttpNoNetwork.al:DefaultRequestHeaders_ReferenceLost_AfterValidationOff
    procedure ReferenceLost(): Text
    var
        Client: HttpClient;
        Headers: HttpHeaders;
        Other: HttpHeaders;
        R: Text;
    begin
        Headers := Client.DefaultRequestHeaders();
        Client.UseServerCertificateValidation(false);
        Headers.Add('X-Test', '1');
        R := Format(Client.DefaultRequestHeaders().Contains('X-Test')) + ' ' + Format(Headers.Contains('X-Test'));
        Other := Client.DefaultRequestHeaders();
        Client.UseServerCertificateValidation(true);
        Other.Add('X-Two', '2');
        R += ' ' + Format(Client.DefaultRequestHeaders().Contains('X-Two')) + ' ' + Format(Client.DefaultRequestHeaders().Contains('X-Test'));
        exit(R + ' ' + Format(Client.DefaultRequestHeaders().Keys().Count()));
    end;

    // CzHttpNoNetwork.al:DefaultRequestHeaders_Reference_ValidationTrueOnFreshClient, first part
    procedure TrueKeepsLink(): Text
    var
        Client: HttpClient;
        Headers: HttpHeaders;
    begin
        Headers := Client.DefaultRequestHeaders();
        Client.UseServerCertificateValidation(true);
        Headers.Add('X-Test', '1');
        exit(Format(Client.DefaultRequestHeaders().Contains('X-Test')));
    end;

    procedure DetachFilled()
    var
        Client: HttpClient;
    begin
        Client.DefaultRequestHeaders().Add('X-Test', '1');
        Client.UseServerCertificateValidation(false);
    end;

    procedure DetachWithBase()
    var
        Client: HttpClient;
    begin
        Client.SetBaseAddress('https://example.com/');
        Client.UseServerCertificateValidation(false);
    end;

    procedure ResultUsed(): Boolean
    var
        Client: HttpClient;
    begin
        exit(Client.UseServerCertificateValidation(false));
    end;

    procedure ResultInEmptyIf(): Text
    var
        Client: HttpClient;
        Headers: HttpHeaders;
    begin
        Headers := Client.DefaultRequestHeaders();
        if Client.UseServerCertificateValidation(false) then;
        Headers.Add('X-Test', '1');
        exit(Format(Client.DefaultRequestHeaders().Contains('X-Test')));
    end;

    // network/lt-httpclient-property-assign/HttpClientPropertyAssign.al:24-32, :51-55: the property-assign form; the call
    // form answers its argument when that is the setting already made.
    procedure PropertyAssign(): Text
    var
        Client: HttpClient;
        Flag: Boolean;
        R: Text;
    begin
        Client.UseServerCertificateValidation := false;
        R := Format(Client.UseServerCertificateValidation(false));
        Client.UseServerCertificateValidation := true;
        R += ' ' + Format(Client.UseServerCertificateValidation(true));
        Client.UseServerCertificateValidation := not Flag;
        Client.UseResponseCookies := false;
        Client.UseResponseCookies := true;
        exit(R);
    end;

    procedure ResultOfAChange(): Boolean
    var
        Client: HttpClient;
    begin
        Client.UseServerCertificateValidation := false;
        exit(Client.UseServerCertificateValidation(true));
    end;

    // HttpClientPropertyAssign.al:73-77: `false` on a client with a timeout gives the default timeout back.
    procedure TimeoutReset(): Text
    var
        Client: HttpClient;
    begin
        Client.Timeout := 5000;
        Client.UseServerCertificateValidation := false;
        exit(Format(Client.Timeout()));
    end;

    // Y6HttpSettingChanges.al:30-132, :165-207 (corpus master).
    local procedure Prepare(var Client: HttpClient)
    begin
        Client.Timeout := 5000;
        Client.SetBaseAddress('https://example.invalid/base/');
        Client.DefaultRequestHeaders().Add('X-Before', '1');
    end;

    local procedure State(var Client: HttpClient): Text
    begin
        exit(Format(Client.Timeout()) + '|' + Client.GetBaseAddress() + '|' + Format(Client.DefaultRequestHeaders().Contains('X-Before')));
    end;

    procedure Rebuilds(): Text
    var
        A: HttpClient;
        B: HttpClient;
        C: HttpClient;
        R: Text;
    begin
        Prepare(A);
        R := Format(A.UseServerCertificateValidation(true)) + ' ' + State(A);
        Prepare(A);
        R += ' / ' + Format(A.UseServerCertificateValidation(false)) + ' ' + State(A);
        Prepare(A);
        R += ' / ' + Format(A.UseServerCertificateValidation(true)) + ' ' + State(A);
        Prepare(B);
        B.UseResponseCookies(false);
        R += ' / ' + State(B);
        B.UseResponseCookies := true;
        R += ' / ' + State(B);
        Prepare(B);
        B.UseServerCertificateValidation := false;
        R += ' / ' + State(B);
        C.UseServerCertificateValidation(false);
        C.Timeout := 2000;
        C.UseResponseCookies(true);
        exit(R + ' / ' + Format(C.Timeout()));
    end;

    procedure AfterClear()
    var
        Client: HttpClient;
    begin
        Client.UseResponseCookies(true);
        Client.Clear();
        Client.UseServerCertificateValidation(false);
    end;

    procedure SendWithoutHandler()
    var
        Client: HttpClient;
        Response: HttpResponseMessage;
    begin
        Client.UseServerCertificateValidation(false);
        Client.Get('https://lt.invalid/', Response);
    end;
}
"#;

#[test]
fn false_detaches_an_earlier_headers_reference_and_true_keeps_the_link() {
    assert_eq!(text(run(SRC, "ReferenceLost")), "No Yes Yes No 1");
    assert_eq!(text(run(SRC, "TrueKeepsLink")), "Yes");
}

#[test]
fn a_rebuild_drops_filled_headers_and_the_base_address() {
    assert!(run(SRC, "DetachFilled").is_ok());
    assert!(run(SRC, "DetachWithBase").is_ok());
}

#[test]
fn the_result_is_the_argument() {
    assert!(!al_testkit::boolean(run(SRC, "ResultUsed")));
    assert_eq!(text(run(SRC, "ResultInEmptyIf")), "No");
}

#[test]
fn the_first_call_that_leaves_the_defaults_rebuilds_the_client_once() {
    assert_eq!(
        text(run(SRC, "Rebuilds")),
        "Yes 5 seconds|https://example.invalid/base/|Yes / No 1 minute 40 seconds||No / Yes 5 seconds|https://example.invalid/base/|Yes \
         / 5 seconds|https://example.invalid/base/|Yes / 1 minute 40 seconds||No / 5 seconds|https://example.invalid/base/|Yes / 2 seconds"
    );
    assert!(refused(run(SRC, "AfterClear")).contains("cleared"));
}

#[test]
fn a_request_no_handler_answers_is_refused_after_the_call() {
    assert!(refused(run(SRC, "SendWithoutHandler")).contains("UseServerCertificateValidation"));
}

#[test]
fn the_property_assign_form_sets_the_value_and_the_call_answers_the_setting_it_repeats() {
    assert_eq!(text(run(SRC, "PropertyAssign")), "No Yes");
    assert!(al_testkit::boolean(run(SRC, "ResultOfAChange")));
}

#[test]
fn false_gives_a_client_with_a_timeout_the_default_timeout() {
    assert_eq!(text(run(SRC, "TimeoutReset")), "1 minute 40 seconds");
}
