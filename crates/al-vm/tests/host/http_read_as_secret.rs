//! `HttpContent.ReadAs(var SecretText)` as a statement and on the content of a request that has none (assumption
//! http-readas-secret-statement, measured by the probe al-language-onprem/http/N4ReadAsSecretStatement.al), and the
//! statement on a content written from a stream, which stays refused.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;

const SRC: &str = r#"
codeunit 50192 C
{
    procedure Written(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('plain body');
        Ct.ReadAs(S);
        exit(Format(S.IsEmpty()));
    end;

    procedure NoContentUsed(): Text
    var
        Req: HttpRequestMessage;
        S: SecretText;
    begin
        S := SecretStrSubstNo('before');
        exit(Format(Req.Content.ReadAs(S)) + Format(S.IsEmpty()));
    end;

    procedure NoContentStatement(): Text
    var
        Req: HttpRequestMessage;
        S: SecretText;
    begin
        Req.Content.ReadAs(S);
        exit('');
    end;

    procedure FromStream(): Text
    var
        Ct: HttpContent;
        Src: HttpContent;
        InS: InStream;
        S: SecretText;
    begin
        Src.WriteFrom('x');
        Src.ReadAs(InS);
        Ct.WriteFrom(InS);
        Ct.ReadAs(S);
        exit('');
    end;
}
"#;

#[test]
fn the_statement_form_and_a_request_without_content_are_assumed() {
    take_hits();
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Written")), "No");
    assert!(take_hits().contains(&Assumption::HttpReadasSecretStatement));
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "NoContentUsed")), "NoNo");
    assert!(take_hits().contains(&Assumption::HttpReadasSecretStatement));
    match al_testkit::run_in(SRC, "C", "NoContentStatement") {
        Err(Signal::Error(e)) => assert_eq!(e.code, "NotInitialized"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_statement_on_a_content_written_from_a_stream_is_refused() {
    match al_testkit::run_in(SRC, "C", "FromStream") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("ReadAs(SecretText) as a statement"), "{m}"),
        other => panic!("{other:?}"),
    }
}
