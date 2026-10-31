//! `JsonObject.WriteWithSecretsTo(Path, Secret, var Result)` and `WriteWithSecretsTo(Secrets, var Result)`, the forms
//! json/lt-json-select-secrets/WriteWithSecrets.al (onprem suite) pins.

fn run(decl: &str, body: &str) -> String {
    let src = format!(
        "codeunit 50100 C {{
        local procedure Sec(V: Text) R: SecretText begin R := V; end;
        [NonDebuggable] local procedure P(R: SecretText): Text begin exit(R.Unwrap()); end;
        procedure F(): Text var O: JsonObject; R: SecretText; B: Boolean; S: Text; {decl} begin
            O.ReadFrom('{{\"u\":\"anna\",\"pw\":\"x\",\"n\":{{\"t\":\"t0\",\"l\":[1]}},\"k\":5}}');
            {body} end; }}"
    );
    al_testkit::text(al_testkit::run_in(&src, "C", "F"))
}

#[test]
fn one_path_substitutes_a_string_and_keeps_the_result_on_failure() {
    let out = run(
        "X: SecretText; S2: Text;",
        "X := Sec('a\"b');
         B := O.WriteWithSecretsTo('$.pw', X, R); S += Format(B) + P(R) + '|';
         B := O.WriteWithSecretsTo('n.t', X, R); S += Format(B) + P(R) + '|';
         B := O.WriteWithSecretsTo('k', X, R); S += Format(B) + P(R) + '|';
         B := O.WriteWithSecretsTo('n.l[0]', X, R); S += Format(B) + '|';
         B := O.WriteWithSecretsTo('missing', X, R); S += Format(B) + '|';
         B := O.WriteWithSecretsTo('', X, R); S += Format(B) + '|';
         B := O.WriteWithSecretsTo('bad[', X, R); S += Format(B) + '|';
         Clear(R); Clear(X);
         B := O.WriteWithSecretsTo('nope', X, R); S += Format(B) + P(R) + '|';
         B := O.WriteWithSecretsTo('pw', X, R); S += Format(B) + P(R) + '|';
         O.WriteTo(S2); exit(S + S2);",
    );
    assert_eq!(
        out,
        "Yes{\"u\":\"anna\",\"pw\":\"a\\\"b\",\"n\":{\"t\":\"t0\",\"l\":[1]},\"k\":5}|\
         Yes{\"u\":\"anna\",\"pw\":\"x\",\"n\":{\"t\":\"a\\\"b\",\"l\":[1]},\"k\":5}|\
         No{\"u\":\"anna\",\"pw\":\"x\",\"n\":{\"t\":\"a\\\"b\",\"l\":[1]},\"k\":5}|No|No|No|No|No|\
         Yes{\"u\":\"anna\",\"pw\":\"\",\"n\":{\"t\":\"t0\",\"l\":[1]},\"k\":5}|\
         {\"u\":\"anna\",\"pw\":\"x\",\"n\":{\"t\":\"t0\",\"l\":[1]},\"k\":5}"
    );
}

#[test]
fn a_dictionary_substitutes_every_path_or_keeps_the_result() {
    let out = run(
        "D: Dictionary of [Text, SecretText];",
        "B := O.WriteWithSecretsTo(D, R); S += Format(B) + P(R) + '|';
         D.Add('$.pw', Sec('pw')); D.Add('n.t', Sec('tk'));
         B := O.WriteWithSecretsTo(D, R); S += Format(B) + P(R) + '|';
         D.Add('missing', Sec('z'));
         B := O.WriteWithSecretsTo(D, R); S += Format(B) + P(R) + '|';
         Clear(D); D.Add('$.k', Sec('7'));
         B := O.WriteWithSecretsTo(D, R); S += Format(B) + P(R);
         exit(S);",
    );
    let pw = "{\"u\":\"anna\",\"pw\":\"pw\",\"n\":{\"t\":\"tk\",\"l\":[1]},\"k\":5}";
    assert_eq!(
        out,
        format!("Yes{{\"u\":\"anna\",\"pw\":\"x\",\"n\":{{\"t\":\"t0\",\"l\":[1]}},\"k\":5}}|Yes{pw}|No{pw}|No{pw}")
    );
}
