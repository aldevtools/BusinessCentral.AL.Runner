//! `SelectToken(Path, var JsonToken)` with a filter `[?(@.name == x)]` or `[?(@.name > n)]`: the forms
//! json/lt-db-json-leads/DbJPathLeads.al:52-85 and json/lt-dc-json-leads/DcJsonLeads.al:286-345 pin, and the root forms
//! `$[?(..)]` and `$.[?(..)]` on a JsonArray of al-runner issue 6588.

use al_builtins::assumptions::{Assumption, take_hits};

fn run(body: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let src = format!(
        "codeunit 50100 C {{
        local procedure Sel(O: JsonObject; P: Text): Text var T: JsonToken; S: Text; begin
            ClearLastError();
            if O.SelectToken(P, T) then T.WriteTo(S);
            exit(Format(O.SelectToken(P, T)) + ' [' + S + '] ' + GetLastErrorCode()); end;
        local procedure Stmt(O: JsonObject; P: Text): Text var T: JsonToken; S: Text; begin
            O.SelectToken(P, T); T.WriteTo(S); exit(S); end;
        procedure F(): Text var O: JsonObject; A: JsonArray; T: JsonToken; S: Text; begin {body} end; }}"
    );
    al_testkit::run_in(&src, "C", "F")
}

fn text(body: &str) -> String {
    al_testkit::text(run(body))
}

const EMPLOYEES: &str =
    r#"O.ReadFrom('{"company":{"employees":[{"id":"John","salary":100},{"id":"Jane","salary":200}]}}');"#;

#[test]
fn an_equality_filter_selects_the_one_match_by_text_or_number() {
    let out = text(&format!(
        "{EMPLOYEES}
         S := Sel(O, '$.company.employees[?(@.id==''John'')].salary');
         S += '|' + Sel(O, '$.company.employees[?(@.id==''Nobody'')].salary');
         S += '|' + Sel(O, '$.company.employees[?(@.salary==200)].id');
         S += '|' + Sel(O, '$.company.employees[?(@.salary>50)].id');
         exit(S);"
    ));
    assert_eq!(out, r#"Yes [100] |No [] |Yes ["Jane"] |No [] "#);
}

#[test]
fn a_double_quoted_text_in_a_filter_answers_false_and_sets_the_last_error() {
    let out = text(&format!(
        "{EMPLOYEES} exit(Sel(O, '$.company.employees[?(@.id==\"John\")].salary'));"
    ));
    assert_eq!(out, "No [] InvalidOperation");
}

#[test]
fn a_filter_statement_without_one_match_raises() {
    let body = r#"O.ReadFrom('{"a":[{"id":1,"name":"x"},{"id":2,"name":"y"},{"id":2,"name":"z"}]}');"#;
    assert_eq!(
        text(&format!("{body} exit(Stmt(O, '$.a[?(@.id==1)].name'));")),
        r#""x""#
    );
    assert_eq!(
        al_testkit::error_message(run(&format!("{body} exit(Stmt(O, '$.a[?(@.id==9)].name'));"))),
        "The JSON path '$.a[?(@.id==9)].name' does not match any tokens."
    );
    assert_eq!(
        al_testkit::error_message(run(&format!("{body} exit(Stmt(O, '$.a[?(@.id==2)].name'));"))),
        "The set of tokens matching the JSON path '$.a[?(@.id==2)].name' contains more than one element."
    );
}

#[test]
fn a_filter_on_the_root_array_selects_the_match() {
    take_hits();
    let body = r#"A.ReadFrom('[{"id":1,"name":"first"},{"id":2,"name":"second"}]');
        A.SelectToken('$[?(@.id==2)]', T); T.AsObject().Get('name', T); S := T.AsValue().AsText();"#;
    assert_eq!(text(&format!("{body} exit(S);")), "second");
    assert!(!take_hits().contains(&Assumption::SelectTokenDotBeforeIndexer));
    let body = r#"A.ReadFrom('[{"id":1,"name":"first"},{"id":2,"name":"second"}]');
        A.SelectToken('$.[?(@.id==2)]', T); T.AsObject().Get('name', T); S := T.AsValue().AsText();"#;
    assert_eq!(text(&format!("{body} exit(S);")), "second");
    assert!(take_hits().contains(&Assumption::SelectTokenDotBeforeIndexer));
}

#[test]
fn a_double_quoted_indexer_answers_false_when_the_result_is_used() {
    let body = r#"O.ReadFrom('{"a":[{"id":1,"name":"x"},{"id":2,"name":"y"}]}');"#;
    assert_eq!(
        text(&format!(
            "{body} exit(Format(O.SelectToken('$[\"a\"][1][\"name\"]', T)));"
        )),
        "No"
    );
}
