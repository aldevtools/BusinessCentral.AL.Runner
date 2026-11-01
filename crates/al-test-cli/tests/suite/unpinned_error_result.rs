//! A test that ends with an error whose text and code no corpus test pins is not run: its result would carry a made-up text.
//! The error is the one HttpHeaders.GetValues raises as a statement on response headers for a missing name (that it raises is
//! pinned by network/lt-ay-http/AyHeaderAddResult.al `get=E`; the text only on content headers, network/lt-http/HttpContent.al:290-291).

const SRC: &str = r#"
codeunit 50194 T
{
    Subtype = Test;
    [Test]
    procedure Uncaught()
    var
        R: HttpResponseMessage;
        H: HttpHeaders;
        Vals: List of [Text];
    begin
        H := R.Headers();
        H.GetValues('X-Missing', Vals);
    end;
}
"#;

#[test]
fn a_test_that_ends_with_an_unpinned_error_is_not_run() {
    let outcomes = al_testkit::outcomes_in(SRC, "T");
    match &outcomes[..] {
        [(_, al_testrun::Outcome::NotRun(why))] => assert!(why.contains("http-get-values-missing-error"), "{why}"),
        other => panic!("expected one test not run, got {other:?}"),
    }
}
