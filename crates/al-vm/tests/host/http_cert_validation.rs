//! `HttpClient.UseServerCertificateValidation`. Pinned by network/lt-cz-http-leads/CzHttpNoNetwork.al (codeunit 85600,
//! BC 27, 28, 29): `false` detaches an HttpHeaders taken earlier from DefaultRequestHeaders, `true` keeps the link.
//! What is not pinned is refused: its result, a detach of headers that are not empty or of a client with a base address or
//! a timeout, and a request of such a client that no handler answers.

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
fn an_unpinned_detach_is_refused() {
    assert!(refused(run(SRC, "DetachFilled")).contains("UseServerCertificateValidation"));
    assert!(refused(run(SRC, "DetachWithBase")).contains("UseServerCertificateValidation"));
}

#[test]
fn the_result_is_refused_unless_an_empty_if_drops_it() {
    assert!(refused(run(SRC, "ResultUsed")).contains("result"));
    assert_eq!(text(run(SRC, "ResultInEmptyIf")), "No");
}

#[test]
fn a_request_no_handler_answers_is_refused_after_the_call() {
    assert!(refused(run(SRC, "SendWithoutHandler")).contains("UseServerCertificateValidation"));
}
