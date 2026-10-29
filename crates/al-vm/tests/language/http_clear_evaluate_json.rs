//! Outside issues 6170, 6171, 6172 and 6175 (corpus writer UF, branch al-runtime-rs-staging-uf, commit 3d7d7b66): Clear of
//! HTTP values; Evaluate of an RFC 1123 date into a DateTime; Evaluate into a JsonArray.

use al_testkit::text;

const SRC: &str = r#"
codeunit 69797 C
{
    procedure HttpClear(): Text
    var Req: HttpRequestMessage; Cnt: HttpContent; Resp: HttpResponseMessage; Txt: Text; T: Text;
    begin
        Cnt.WriteFrom('first body');
        Req.Method := 'PUT';
        Req.SetRequestUri('http://localhost/uf');
        Req.Content := Cnt;
        Clear(Req);
        T += Req.Method + ';[' + Req.GetRequestUri() + '];';
        Clear(Cnt);
        Cnt.ReadAs(Txt);
        T += '[' + Txt + '];' + Format(Cnt.IsSecretContent()) + ';';
        Clear(Resp);
        T += Format(Resp.HttpStatusCode());
        exit(T);
    end;

    procedure Rfc(): Text
    var D: DateTime; T: Text;
    begin
        T += Format(Evaluate(D, 'Fri, 05 Sep 2025 14:23:45 GMT', 0)) + ';' + Format(D, 0, 9) + ';';
        Clear(D);
        T += Format(Evaluate(D, 'Mon, 05 Sep 2025 14:23:45 GMT', 0)) + ';' + GetLastErrorCode() + ';';
        T += Format(Evaluate(D, '05 Sep 2025 14:23:45', 0)) + ';' + GetLastErrorCode();
        exit(T);
    end;

    procedure JsonArr(): Text
    var Arr: JsonArray; Out: Text; T: Text;
    begin
        T += Format(Evaluate(Arr, ' [ "x" ] ')) + ';';
        Arr.WriteTo(Out); T += Out + ';';
        T += Format(Evaluate(Arr, '{"a":1}')) + ';' + GetLastErrorText() + ';';
        Arr.WriteTo(Out); T += Out + ';';
        T += Format(Evaluate(Arr, '[1,')) + ';' + GetLastErrorCode();
        exit(T);
    end;
}
"#;

#[test]
fn clear_of_http_values_gives_new_ones() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "HttpClear")), "GET;[];[];No;200");
}

#[test]
fn evaluate_reads_an_rfc_1123_date() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Rfc")),
        "Yes;2025-09-05T14:23:45Z;No;;No;DateInvalid"
    );
}

#[test]
fn evaluate_into_a_json_array() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "JsonArr")),
        "Yes;[\"x\"];No;JSON tokens of type 'Object' are not supported by this implementation.;[\"x\"];No;InvalidJsonData"
    );
}
