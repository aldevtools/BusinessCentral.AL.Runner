//! ADR-066 amendment 12 (owner, 2026-11-09): by default every test codeunit starts with fresh SingleInstance state and
//! the install baseline's NumberSequence values. A failed test that read state an earlier codeunit would have set on BC
//! carries a hint (console, JUnit property `orderHint`, JSON `orderHint`). ADR-070 decision 3 dropped the static list of
//! codeunits that take numbers after another one: only a failed taker gets a numbers hint. `--strict-order` runs them in
//! BC's order.

use super::*;

const SRC: &str = r#"
codeunit 50961 "Order Cli State"
{
    SingleInstance = true;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Flag: Text;
}
codeunit 50962 "Order Cli Installer"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('ORDERCLI', 1, 1); end;
}
codeunit 50971 "Order Cli Setter"
{
    Subtype = Test;
    [Test] procedure SetsTheFlag() var S: Codeunit "Order Cli State"; begin S.SetFlag('left'); end;
}
codeunit 50972 "Order Cli Reader"
{
    Subtype = Test;
    [Test] procedure ReadsTheFlag() var S: Codeunit "Order Cli State"; begin if S.GetFlag() <> 'left' then Error('flag is empty'); end;
}
codeunit 50973 "Order Cli First Taker"
{
    Subtype = Test;
    [Test] procedure TakesOne() var N: BigInteger; begin N := NumberSequence.Next('ORDERCLI'); end;
}
codeunit 50974 "Order Cli Second Taker"
{
    Subtype = Test;
    [Test] procedure TakesAnother() var N: BigInteger; begin N := NumberSequence.Next('ORDERCLI'); end;
}
"#;

#[test]
fn the_default_run_hints_at_bcs_order_and_strict_order_follows_it() {
    let d = tempfile::tempdir().unwrap();
    let app = one_file_app(
        d.path(),
        "order",
        "11111111-0000-0000-0000-000000007506",
        "Order",
        "src/Tests.al",
        SRC,
    );
    let a = app.to_str().unwrap();
    let junit = d.path().join("r.xml");
    let json = d.path().join("r.json");
    let o = alrunner(&[
        "test",
        a,
        "--no-download",
        "--junit",
        junit.to_str().unwrap(),
        "--output-json",
        json.to_str().unwrap(),
    ]);
    assert_eq!(code(&o), 1, "{}", text(&o));
    let out = text(&o);
    assert!(
        out.contains("order hint: ") && out.contains("Order Cli State") && out.contains("--strict-order"),
        "{out}"
    );
    let x = std::fs::read_to_string(&junit).unwrap();
    // A test case and its properties are on one line.
    let case = |name: &str| {
        x.lines()
            .find(|l| l.contains(&format!("name=\"{name}\"")))
            .unwrap_or_else(|| panic!("{x}"))
    };
    assert!(
        case("ReadsTheFlag").contains("<property name=\"orderHint\" value=\"this test read global Flag"),
        "{x}"
    );
    // A passing taker gets no numbers hint (ADR-070 decision 3: only a failed test does).
    assert!(!case("TakesAnother").contains("orderHint"), "{x}");
    let j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    let row = j["tests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["method"] == "ReadsTheFlag")
        .unwrap();
    assert!(
        row["orderHint"]
            .as_str()
            .is_some_and(|h| h.contains("Order Cli Setter")),
        "{row}"
    );
    // `--strict-order`: BC's order, the reader passes and nothing is hinted.
    let o = alrunner(&["test", a, "--no-download", "--strict-order"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(
        !text(&o).contains("order hint: ") && !text(&o).contains("number order: "),
        "{}",
        text(&o)
    );
}

/// ADR-070 migration step 4: the reports of the two order modes.
const REPORTS: &str = r#"
codeunit 50981 "Order Rpt State"
{
    SingleInstance = true;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
    var Flag: Text;
}
codeunit 50982 "Order Rpt Installer"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany() begin NumberSequence.Insert('ORDERRPT', 1, 1); end;
}
codeunit 50983 "Order Rpt Setter"
{
    Subtype = Test;
    [Test] procedure SetsTheFlag() var S: Codeunit "Order Rpt State"; begin S.SetFlag('left'); end;
}
codeunit 50984 "Order Rpt Passing Reader"
{
    Subtype = Test;
    [Test] procedure ReadsTheEmptyFlag() var S: Codeunit "Order Rpt State"; begin if S.GetFlag() <> '' then Error('flag is set'); end;
    [Test] procedure ReadsNothing() begin end;
}
codeunit 50985 "Order Rpt First Taker"
{
    Subtype = Test;
    [Test] procedure TakesOne() var N: BigInteger; begin N := NumberSequence.Next('ORDERRPT'); end;
}
codeunit 50986 "Order Rpt Second Taker"
{
    Subtype = Test;
    [Test] procedure TakesTheSecond() begin if NumberSequence.Next('ORDERRPT') <> 2 then Error('took the first number'); end;
}
"#;

/// Runs [`REPORTS`] with `extra`, returning the output, the JUnit file and the JSON document.
fn reports_run(extra: &[&str]) -> (Output, String, serde_json::Value) {
    let d = tempfile::tempdir().unwrap();
    let app = one_file_app(
        d.path(),
        "orderrpt",
        "11111111-0000-0000-0000-000000007507",
        "OrderRpt",
        "src/Tests.al",
        REPORTS,
    );
    let junit = d.path().join("r.xml");
    let json = d.path().join("r.json");
    let mut args = vec![
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--junit",
        junit.to_str().unwrap(),
        "--output-json",
        json.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    let o = alrunner(&args);
    let x = std::fs::read_to_string(&junit).unwrap_or_default();
    let j = serde_json::from_str(&std::fs::read_to_string(&json).unwrap_or_default()).unwrap_or_default();
    (o, x, j)
}

/// The JUnit line of test case `name` (a test case and its properties are on one line).
fn junit_case<'a>(x: &'a str, name: &str) -> &'a str {
    x.lines()
        .find(|l| l.contains(&format!("name=\"{name}\"")))
        .unwrap_or_else(|| panic!("{x}"))
}

#[test]
fn the_run_states_its_order_mode_in_the_header_junit_and_json() {
    let (o, x, j) = reports_run(&[]);
    assert!(text(&o).contains("Order mode: isolated"), "{}", text(&o));
    assert!(x.contains("<property name=\"orderMode\" value=\"isolated\" />"), "{x}");
    assert_eq!(j["orderMode"], "isolated", "{j}");
    let (o, x, j) = reports_run(&["--strict-order"]);
    assert!(text(&o).contains("Order mode: bc"), "{}", text(&o));
    assert!(x.contains("<property name=\"orderMode\" value=\"bc\" />"), "{x}");
    assert_eq!(j["orderMode"], "bc", "{j}");
}

#[test]
fn passing_tests_that_read_carried_single_instance_state_get_a_summary_line_and_a_property() {
    let (o, x, _) = reports_run(&[]);
    let out = text(&o);
    assert!(
        out.contains(
            "2 passing tests read SingleInstance state that an earlier codeunit would have left differently on BC. \
             --strict-order runs BC's order."
        ),
        "{out}"
    );
    assert!(
        junit_case(&x, "ReadsTheEmptyFlag").contains("<property name=\"singleInstanceCarried\" value=\""),
        "{x}"
    );
    // A test after the first reader of its codeunit counts too: it starts with the state that read left (the rule of the
    // failure hint, `order_hint::hints_for`).
    assert!(junit_case(&x, "ReadsNothing").contains("singleInstanceCarried"), "{x}");
    assert!(!junit_case(&x, "SetsTheFlag").contains("singleInstanceCarried"), "{x}");
    // In BC's order the state is BC's: no line.
    let (o, _, _) = reports_run(&["--strict-order"]);
    assert!(!text(&o).contains("passing tests read SingleInstance state"), "{}", text(&o));
}

#[test]
fn a_failed_test_that_took_numbers_an_earlier_codeunit_also_took_gets_a_numbers_hint() {
    let (o, x, j) = reports_run(&[]);
    let out = text(&o);
    let hint = "this test took numbers from sequence ORDERRPT after test codeunit 50985 \"Order Rpt First Taker\" took some on BC";
    assert!(out.contains(hint), "{out}");
    // The static list of amendment 12 is gone (ADR-070 decision 3).
    assert!(!out.contains("number order: "), "{out}");
    assert!(!x.contains("numberSequenceOrder"), "{x}");
    assert!(
        junit_case(&x, "TakesTheSecond").contains(&hint.replace('"', "&quot;")),
        "{x}"
    );
    assert!(!junit_case(&x, "TakesOne").contains("orderHint"), "{x}");
    let row = j["tests"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["method"] == "TakesTheSecond")
        .unwrap()
        .clone();
    assert!(row["orderHint"].as_str().is_some_and(|h| h.contains(hint)), "{row}");
    // In BC's order the numbers chain: the test passes and nothing is hinted.
    let (o, x, _) = reports_run(&["--strict-order"]);
    assert!(!text(&o).contains("took numbers from sequence"), "{}", text(&o));
    assert!(!junit_case(&x, "TakesTheSecond").contains("orderHint"), "{x}");
}

/// A codeunit that only reads the sequence the takers took from: in BC's order it sees 2, in the default run 1.
const CURRENT_READER: &str = r#"
codeunit 50987 "Order Rpt Current Reader"
{
    Subtype = Test;
    [Test] procedure ReadsTheCurrent() begin if NumberSequence.Current('ORDERRPT') <> 2 then Error('saw the first number'); end;
}
"#;

/// ADR-070 decision 7: the stability dimension `isolation.singleInstance` compares two plain runs (the default isolated
/// run against BC's order). A flip whose codeunit read no carried globals gets "took numbers ... after test codeunit X"
/// when the fresh run gave the test a numbers hint, else "cause not known"; a flip of a reader of carried globals keeps
/// the reads.
#[test]
fn stability_compares_two_plain_runs_and_names_the_cause_of_each_flip() {
    let d = tempfile::tempdir().unwrap();
    let app = one_file_app(
        d.path(),
        "orderstab",
        "11111111-0000-0000-0000-000000007508",
        "OrderStab",
        "src/Tests.al",
        &format!("{REPORTS}{CURRENT_READER}"),
    );
    let cfg = d.path().join("si.json");
    std::fs::write(
        &cfg,
        r#"{"format":1,"dimensions":{"isolation.singleInstance":["carried","fresh"]}}"#,
    )
    .unwrap();
    let (json, junit) = (d.path().join("out.json"), d.path().join("out.xml"));
    let o = alrunner(&[
        "test",
        app.to_str().unwrap(),
        "--no-download",
        "--stability",
        cfg.to_str().unwrap(),
        "--output-json",
        json.to_str().unwrap(),
        "--junit",
        junit.to_str().unwrap(),
    ]);
    let out = text(&o);
    assert_eq!(code(&o), 1, "a flip is exit 1: {out}");
    let doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
    let flips = doc["stability"]["flips"].as_array().unwrap().clone();
    let reason = |test: &str| {
        flips
            .iter()
            .find(|f| f["test"] == test)
            .unwrap_or_else(|| panic!("{test}: {flips:?}"))["reason"]
            .clone()
    };
    let numbers = "took numbers from sequence ORDERRPT after test codeunit 50985 \"Order Rpt First Taker\"";
    let taker = reason("Codeunit50986.TakesTheSecond");
    assert_eq!(taker["kind"], "passesOnlyAfterEarlierCodeunits", "{taker}");
    assert_eq!(taker["cause"], numbers, "{taker}");
    let si = reason("Codeunit50984.ReadsTheEmptyFlag");
    assert_eq!(si["reads"][0]["global"], "Flag", "{si}");
    let reader = reason("Codeunit50987.ReadsTheCurrent");
    assert_eq!(reader["cause"], "cause not known", "{reader}");
    assert!(out.contains("Other flips in the default isolated run:"), "{out}");
    assert!(out.contains(&format!("Codeunit50986.TakesTheSecond: {numbers}")), "{out}");
    let x = std::fs::read_to_string(&junit).unwrap();
    assert!(x.contains("(cause not known)"), "{x}");
}
