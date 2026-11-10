//! A list whose base page declares no OnOpenPage, and whose page extension's OnOpenPage moves `Rec`, opens on that row
//! (al-runner issue 6924, `IndicateDefaultShipToCodeOnShipToAddressList`: Microsoft's "Ship-to Address List" has no
//! triggers, the partner's page extension moves `Rec` through a table procedure and a codeunit that take it as `var`).
//! The corpus pins the same open when the base page also declares OnOpenPage and the extension's runs after it
//! (pages/onopenpage-moves-rec/OnOpenPageGetChain.al:550 Og_ExtList_OpenView_Base1Ext0, :587 Og_ExtList_OpenView_Base0Ext1,
//! :624 Og_ExtList_OpenView_Base0Ext2). Whether the base declares the trigger does not change which triggers run, so the
//! open treats the extension's trigger as the page's.

use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const SRC: &str = r#"
table 50300 "PE Addr"
{
    fields { field(1; "Cust"; Code[20]) { } field(2; "Code"; Code[10]) { } field(3; "Def"; Integer) { } }
    keys { key(PK; "Cust", "Code") { Clustered = true; } }
}
codeunit 50301 "PE Get Default"
{
    procedure GetDefault(var Addr: Record "PE Addr")
    var
        Found: Record "PE Addr";
    begin
        if not Addr.FindSet() then exit;
        Found.CopyFilters(Addr);
        Found.SetFilter(Def, '>%1', 0);
        if not Found.FindFirst() then exit;
        Addr.Get(Found.Cust, Found.Code);
    end;
}
page 50302 "PE Addr List"
{
    PageType = List;
    SourceTable = "PE Addr";
    layout { area(Content) { repeater(R) { field(Cust; Rec.Cust) { } field(Code; Rec.Code) { } } } }
}
pageextension 50303 "PE Addr List Ext" extends "PE Addr List"
{
    trigger OnOpenPage()
    var
        M: Codeunit "PE Get Default";
    begin
        M.GetDefault(Rec);
    end;
}
codeunit 50304 T
{
    Subtype = Test;

    [Test]
    procedure OpensOnTheDefaultRow()
    var
        A: Record "PE Addr";
        P: TestPage "PE Addr List";
    begin
        A.Cust := 'C1'; A.Code := 'A'; A.Insert();
        A.Code := 'B'; A.Def := 1; A.Insert();
        A.Code := 'C'; A.Def := 0; A.Insert();
        P.OpenView();
        if P.Code.Value() <> 'B' then
            Error('opened on %1', P.Code.Value());
    end;
}"#;

#[test]
fn issue_6924_extension_only_open_page_moves_rec() {
    let sources = vec![AppSource {
        id: "app".into(),
        name: "app".into(),
        dependencies: vec![],
        files: vec![("app.al".into(), SRC.to_string())],
    }];
    let (ws, l) = al_sema::lower::compile_apps(sources, vec![AppKind::Workspace], &Default::default());
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let results = run_codeunit(&Engine::new(l.program.clone(), blocked), &l, oi);
    assert!(!results.is_empty());
    for (name, outcome) in results {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}
