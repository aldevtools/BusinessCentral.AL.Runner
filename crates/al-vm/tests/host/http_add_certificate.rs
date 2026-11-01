//! `HttpClient.AddCertificate` reads the certificate when it is called. Pinned by
//! network/lt-httpclient-config-certificates/P108Cert1.al:21-40 (Text) and P108Cert2.al:21-37 (SecretText): an empty
//! certificate raises HttpRequest:Argument, text that is not Base64 HttpRequest:Format, Base64 of a cut DER sequence
//! (`MIIB`) HttpRequest:Cryptographic, each with the generic request error text. Every other certificate is not pinned and
//! is refused (issue 6484).

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
codeunit 50192 C
{
    local procedure Seen(Name: Text): Text
    begin
        exit(Name + '=' + GetLastErrorCode() + ':' + GetLastErrorText() + '|');
    end;

    procedure TextCases(): Text
    var
        Client: HttpClient;
        Cert: Text;
        Pwd: Text;
        R: Text;
    begin
        Cert := 'not a certificate';
        Pwd := 'pw';
        asserterror Client.AddCertificate(Cert, Pwd);
        R += Seen('garbage_with_password');
        asserterror Client.AddCertificate(Cert);
        R += Seen('garbage_no_password');
        Cert := '';
        asserterror Client.AddCertificate(Cert, Pwd);
        R += Seen('empty');
        Cert := 'MIIB';
        asserterror Client.AddCertificate(Cert, '');
        R += Seen('base64_stub');
        exit(R);
    end;

    procedure SecretCases(): Text
    var
        Client: HttpClient;
        Cert: SecretText;
        Pwd: SecretText;
        R: Text;
    begin
        Cert := SecretStrSubstNo('not a certificate');
        Pwd := SecretStrSubstNo('pw');
        asserterror Client.AddCertificate(Cert, Pwd);
        R += Seen('garbage_with_password');
        asserterror Client.AddCertificate(Cert);
        R += Seen('garbage_no_password');
        Clear(Cert);
        asserterror Client.AddCertificate(Cert, Pwd);
        R += Seen('empty');
        exit(R);
    end;

    procedure CompleteDer()
    var
        Client: HttpClient;
    begin
        Client.AddCertificate('MAIFAA==', 'pw');
    end;

    procedure NotDer()
    var
        Client: HttpClient;
    begin
        Client.AddCertificate('AAAA');
    end;
}
"#;

const ERR: &str =
    "There was an error while executing the HTTP request. Make sure you are connecting to a valid endpoint.";

#[test]
fn text_certificates_raise_the_pinned_errors() {
    assert_eq!(
        text(run(SRC, "TextCases")),
        format!(
            "garbage_with_password=HttpRequest:Format:{ERR}|garbage_no_password=HttpRequest:Format:{ERR}|\
             empty=HttpRequest:Argument:{ERR}|base64_stub=HttpRequest:Cryptographic:{ERR}|"
        )
    );
}

#[test]
fn secret_certificates_raise_the_pinned_errors() {
    assert_eq!(
        text(run(SRC, "SecretCases")),
        format!(
            "garbage_with_password=HttpRequest:Format:{ERR}|garbage_no_password=HttpRequest:Format:{ERR}|\
             empty=HttpRequest:Argument:{ERR}|"
        )
    );
}

#[test]
fn a_certificate_no_test_pins_is_refused() {
    assert!(refused(run(SRC, "CompleteDer")).contains("AddCertificate"));
    assert!(refused(run(SRC, "NotDer")).contains("AddCertificate"));
}
