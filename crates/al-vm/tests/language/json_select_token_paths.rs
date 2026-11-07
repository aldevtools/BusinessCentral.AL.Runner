//! `SelectToken` path forms of json/DbJPathLeads.al:465-873 (`P6924_*`, al-runner issue 6924), `Union_OfKeys` (:85) and
//! `LastArrayElement` (:154): wildcards, slices with a step, quoted names and unions, odd indexers, `..`, and the statement form.

fn run(body: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let src = format!(
        "codeunit 50100 C {{
        local procedure Doc(var O: JsonObject) begin
            O.ReadFrom('{{\"a\":{{\"b\":1,\"c\":[10,20,30],\"d e\":2,\"x.y\":3,\"it''s\":4,\"\":5,\"n\":null}},\"arr\":[{{\"k\":1}},{{\"k\":2}}],\"one\":[{{\"k\":7}}],\"empty\":[],\"s\":\"txt\"}}'); end;
        local procedure Sel(P: Text): Text var O: JsonObject; T: JsonToken; S: Text; B: Boolean; begin
            Doc(O); ClearLastError();
            B := O.SelectToken(P, T);
            if B then T.WriteTo(S);
            exit(Format(B) + ' [' + S + '] ' + GetLastErrorCode()); end;
        local procedure Stmt(P: Text): Text var O: JsonObject; T: JsonToken; S: Text; begin
            Doc(O); O.SelectToken(P, T); T.WriteTo(S); exit(S); end;
        procedure F(): Text begin {body} end; }}"
    );
    al_testkit::run_in(&src, "C", "F")
}

fn sel(paths: &[&str]) -> Vec<String> {
    paths
        .iter()
        .map(|p| al_testkit::text(run(&format!("exit(Sel('{}'));", p.replace('\'', "''")))))
        .collect()
}

fn stmt_err(p: &str) -> String {
    al_testkit::error_message(run(&format!("exit(Stmt('{}'));", p.replace('\'', "''"))))
}

#[test]
fn wildcard_names_and_empty_names() {
    // DbJPathLeads.al:465-503
    let got = sel(&["a.*", "$.*", "*", "one.*", "a.", "$.", ".a.b", "@.a"]);
    assert_eq!(
        got,
        [
            "No [] ",
            "No [] ",
            "No [] ",
            "No [] ",
            "No [] InvalidOperation",
            "No [] InvalidOperation",
            "Yes [1] ",
            "No [] "
        ]
    );
}

#[test]
fn quoted_names_unions_and_blanks() {
    // DbJPathLeads.al:545-583 and Union_OfKeys :85-152
    let got = sel(&[
        "a['d e']",
        "a.d e",
        "a['x.y']",
        "a.x.y",
        "a['it\\'s']",
        "a['']",
        "$['a']['b']",
        "$.a['n']",
        "$['a','n']",
        "$['a','arr']",
        "$.[x,y]",
    ]);
    assert_eq!(
        got,
        [
            "Yes [2] ",
            "No [] InvalidOperation",
            "Yes [3] ",
            "No [] ",
            "Yes [4] ",
            "Yes [5] ",
            "Yes [1] ",
            "Yes [null] ",
            "Yes [{\"b\":1,\"c\":[10,20,30],\"d e\":2,\"x.y\":3,\"it's\":4,\"\":5,\"n\":null}] ",
            "No [] ",
            "No [] InvalidOperation"
        ]
    );
}

#[test]
fn slices_wildcard_indexers_and_odd_indexers() {
    // DbJPathLeads.al:585-703 and LastArrayElement :154-175
    let got = sel(&[
        "a.c[:1]",
        "a.c[2:]",
        "a.c[0:3:2]",
        "a.c[::3]",
        "a.c[1:2]",
        "a.c[0:2]",
        "a.c[5:9]",
        "arr[1:2].k",
        "a.c[-1:]",
        "a.c[-2:]",
        "one[*].k",
        "arr[*].k",
        "empty[*]",
        "a[*]",
        "$[*]",
        "a.c[ 1 ]",
        "a.c[+1]",
        "a.c[01]",
        "a.c[1.0]",
        "a.c[x]",
        "a.c['1']",
        "a[0]",
        "a.c[(@.length-1)]",
        "a.c[-1]",
    ]);
    let want = [
        "Yes [10] ",
        "Yes [30] ",
        "No [] ",
        "Yes [10] ",
        "Yes [20] ",
        "No [] ",
        "No [] ",
        "Yes [2] ",
        "Yes [30] ",
        "No [] ",
        "Yes [7] ",
        "No [] ",
        "No [] ",
        "No [] ",
        "No [] ",
        "Yes [20] ",
        "No [] InvalidOperation",
        "Yes [20] ",
        "No [] InvalidOperation",
        "No [] InvalidOperation",
        "No [] ",
        "No [] ",
        "No [] InvalidOperation",
        "No [] ArgumentOutOfRange",
    ];
    assert_eq!(got, want);
}

#[test]
fn recursive_descent() {
    // DbJPathLeads.al:505-543
    let got = sel(&[
        "a..b",
        "$..k",
        "$..zzz",
        "$..b",
        "$..[0]",
        "$..c[1]",
        "$..*",
        "$..one[0].k",
    ]);
    assert_eq!(
        got,
        [
            "Yes [1] ",
            "No [] ",
            "No [] ",
            "Yes [1] ",
            "No [] ",
            "Yes [20] ",
            "No [] ",
            "Yes [7] "
        ]
    );
}

#[test]
fn the_statement_form_raises_for_none_and_for_many() {
    // DbJPathLeads.al:745-827
    let none = |p: &str| format!("The JSON path '{p}' does not match any tokens.");
    let many = |p: &str| format!("The set of tokens matching the JSON path '{p}' contains more than one element.");
    for p in ["nope", "a.zzz", "a.c[9]", "empty[*]", "$..zzz", "arr.k"] {
        assert_eq!(stmt_err(p), none(p));
    }
    for p in ["$..k", "a.c[0:2]", "a.*", "arr[*].k"] {
        assert_eq!(stmt_err(p), many(p));
    }
    for p in ["a.", "a[\"b\"]"] {
        assert_eq!(
            stmt_err(p),
            "The requested operation cannot be performed in this context."
        );
    }
    let ok = |p: &str| al_testkit::text(run(&format!("exit(Stmt('{}'));", p.replace('\'', "''"))));
    assert_eq!(ok("one[*]"), r#"{"k":7}"#);
    assert_eq!(ok("$..b"), "1");
    assert_eq!(ok("a.c[1:2]"), "20");
    assert_eq!(ok("a['d e']"), "2");
}
