//! A doubled quote inside a quoted identifier is one escaped quote (issue 30). Microsoft's page
//! "Sales Order Statistics" names a control `"TotalAdjCostLCY[2] - TotalSalesLineLCY[2].""Unit Cost (LCY)"""`
//! and its test reads it by that name; real BC compiles both. No corpus test uses the escape.

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

#[test]
fn a_control_named_with_an_escaped_quote_is_reached_by_that_name() {
    let src = r#"
table 63200 "QI Row" { fields { field(1; "No."; Code[20]) { } field(2; Amount; Decimal) { } } keys { key(PK; "No.") { } } }
page 63201 "QI Card"
{
    PageType = Card; SourceTable = "QI Row";
    layout { area(Content) { field("Amount.""Quoted"""; Rec.Amount) { Editable = false; } } }
}
codeunit 63202 T
{
    Subtype = Test;
    [Test] procedure ReadsTheControl() var R: Record "QI Row"; P: TestPage "QI Card"; D: Decimal;
    begin
        R."No." := 'A'; R.Amount := 12.5; R.Insert();
        P.OpenView(); P.GoToRecord(R);
        D := P."Amount.""Quoted""".AsDecimal();
        if D <> 12.5 then Error('got %1', D);
    end;
}
"#;
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let o = run_codeunit(&engine, &l, oi);
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0]);
}
