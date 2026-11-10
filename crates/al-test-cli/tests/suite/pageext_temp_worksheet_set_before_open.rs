//! A temporary Worksheet whose row was set before it opened, with a page extension whose OnOpenPage does not move `Rec`,
//! still runs (al-runner issue 6924, SalesPriceShipmentTypeActivateLinesDuplicatesShowOK/ShowCancel regressed to not run when
//! the open began to count an extension-only OnOpenPage). Only the base page's own OnOpenPage reaches the temporary
//! Worksheet branch, as before.

use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const SRC: &str = r#"
table 50310 "TW Row"
{
    TableType = Temporary;
    fields { field(1; "Code"; Code[10]) { } field(2; "Name"; Text[20]) { } }
    keys { key(PK; "Code") { Clustered = true; } key(ByName; "Name") { } }
}
page 50311 "TW Ws"
{
    PageType = Worksheet;
    SourceTable = "TW Row";
    SourceTableTemporary = true;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } field(Name; Rec.Name) { } } } }
    procedure SetRows(var Src: Record "TW Row")
    begin
        Rec.Copy(Src, true);
    end;
}
pageextension 50312 "TW Ws Ext" extends "TW Ws"
{
    trigger OnOpenPage()
    var
        X: Integer;
    begin
        X := 1;
    end;
}
codeunit 50313 T
{
    Subtype = Test;

    [Test]
    [HandlerFunctions('Handler')]
    procedure RunsInLookupModeOnARowSetBefore()
    var
        Src: Record "TW Row" temporary;
        P: Page "TW Ws";
    begin
        Src.Code := 'A'; Src.Name := 'z'; Src.Insert();
        Src.Code := 'B'; Src.Name := 'y'; Src.Insert();
        Src.Code := 'C'; Src.Name := 'x'; Src.Insert();
        Src.SetCurrentKey(Name);
        Src.Get('B');
        P.SetRows(Src);
        P.LookupMode(true);
        P.RunModal();
    end;

    [ModalPageHandler]
    procedure Handler(var P: TestPage "TW Ws")
    begin
        P.OK().Invoke();
    end;
}"#;

#[test]
fn issue_6924_temp_worksheet_with_extension_open_page_still_runs() {
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
