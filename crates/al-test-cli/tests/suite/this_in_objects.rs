//! `this` is the running object in every object type, not only in a codeunit (issue 32): Base Application's
//! "Calculate Inventory" report calls `this.GetLocation(..)` in `OnAfterGetRecord`. Evidence is Microsoft Learn,
//! "this keyword" (AL language version 14: "self-reference for all objects"); no corpus test uses it outside a
//! codeunit (docs/probes-needed.md).

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

#[test]
fn a_report_calls_its_own_procedure_and_reads_its_own_variable_through_this() {
    let src = r#"
codeunit 63401 "TI Log" { SingleInstance = true; var L: Text; procedure Add(T: Text) begin L += T; end; procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end; }
table 63403 "TI Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
report 63400 "TI Report"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset { dataitem(Loop; "TI Row") {
        trigger OnAfterGetRecord() var Log: Codeunit "TI Log"; begin Log.Add(this.Name() + ':' + Format(this.Counter)); end; } }
    trigger OnPreReport() begin Counter := 7; end;
    local procedure Name(): Text begin exit('rep'); end;
    var Counter: Integer;
}
codeunit 63402 T
{
    Subtype = Test;
    [Test] procedure ThisInAReport() var Log: Codeunit "TI Log"; R: Record "TI Row"; begin R.Id := 1; R.Insert(); Report.Run(Report::"TI Report", false, false); if Log.Take() <> 'rep:7' then Error('got %1', Log.Take()); end;
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

// A query column named like a query method: `Q.Open()` is the method (Base Application's "Summarized Stock By
// Item Trkg." has a filter column `Open` and its caller opens it). Real BC compiles that code.
#[test]
fn a_query_method_call_is_not_taken_for_a_column_of_the_same_name() {
    let src = r#"
table 63410 "QC Row" { fields { field(1; Id; Integer) { } field(2; Open; Boolean) { } } keys { key(PK; Id) { } } }
query 63411 "QC Query" { elements { dataitem(R; "QC Row") { column(Id; Id) { } filter(Open; Open) { } } } }
codeunit 63412 T
{
    Subtype = Test;
    [Test] procedure OpenIsTheMethod() var Q: Query "QC Query"; Row: Record "QC Row"; N: Integer;
    begin
        Row.Id := 1; Row.Open := true; Row.Insert(); Row.Id := 2; Row.Open := false; Row.Insert();
        Q.SetRange(Open, true);
        Q.Open();
        while Q.Read() do N += 1;
        Q.Close();
        if N <> 1 then Error('rows %1', N);
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

// A bare column name in a query's own procedure reads the current row (Base Application's "Summarized Stock By
// Item Trkg." `GetItemTrackingSetup`); documented assumption, no corpus test (docs/probes-needed.md).
#[test]
fn a_bare_column_name_in_a_query_procedure_reads_the_current_row() {
    let src = r#"
table 63420 "QB Row" { fields { field(1; Id; Integer) { } field(2; Txt; Text[20]) { } } keys { key(PK; Id) { } } }
query 63421 "QB Query" { elements { dataitem(R; "QB Row") { column(Id; Id) { } column(Txt; Txt) { } } }
    procedure Describe(): Text begin exit(Txt + '/' + Format(Id)); end; }
codeunit 63422 T
{
    Subtype = Test;
    [Test] procedure BareColumn() var Q: Query "QB Query"; Row: Record "QB Row"; Got: Text;
    begin
        Row.Id := 5; Row.Txt := 'five'; Row.Insert();
        Q.Open(); if not Q.Read() then Error('no row'); Got := Q.Describe(); Q.Close();
        if Got <> 'five/5' then Error('got %1', Got);
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
