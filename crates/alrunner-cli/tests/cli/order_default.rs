//! ADR-066 amendment 12 (owner, 2026-11-09): by default every test codeunit starts with fresh SingleInstance state and
//! the install baseline's NumberSequence values. A failed test that read state an earlier codeunit would have set on BC
//! carries a hint (console, JUnit property `orderHint`, JSON `orderHint`); the codeunits that take numbers after another
//! one are listed (console, JUnit property `numberSequenceOrder`). `--strict-order` runs them in BC's order.

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
    assert!(
        out.contains("number order: test codeunit 50974 \"Order Cli Second Taker\""),
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
    assert!(
        case("TakesAnother").contains("<property name=\"numberSequenceOrder\""),
        "{x}"
    );
    assert!(!case("TakesOne").contains("numberSequenceOrder"), "{x}");
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
    // `--strict-order`: BC's order, the reader passes and nothing is hinted or listed.
    let o = alrunner(&["test", a, "--no-download", "--strict-order"]);
    assert_eq!(code(&o), 0, "{}", text(&o));
    assert!(
        !text(&o).contains("order hint: ") && !text(&o).contains("number order: "),
        "{}",
        text(&o)
    );
}
