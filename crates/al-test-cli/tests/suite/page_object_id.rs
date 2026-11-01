//! `Page.ObjectId(UseNames)` (issue 42), taken the way the probe pins `Report.ObjectId`: `Page <id>`, or with names
//! (the default) `Page <Caption>`, the object name when the page has no Caption (handlers/PWRefusedBuiltins.al:78-107
//! pins the report form on BC 27.5). Documented assumption for pages (docs/probes-needed.md).

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

#[test]
fn page_object_id_reads_like_report_object_id() {
    let src = r#"
table 63600 "PO Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 63601 "PO Log" { SingleInstance = true; var L: Text; procedure Add(T: Text) begin L += T; end; procedure Take(): Text begin exit(L); end; }
page 63602 "PO Card"
{
    PageType = Card; SourceTable = "PO Row"; Caption = 'PO caption';
    layout { area(Content) { field(Id; Rec.Id) { } } }
    actions { area(Processing) { action(Ids) { trigger OnAction() var Log: Codeunit "PO Log"; begin Log.Add(CurrPage.ObjectId(false) + '|' + CurrPage.ObjectId(true) + '|' + CurrPage.ObjectId()); end; } } }
}
page 63603 "PO Plain"
{
    PageType = Card; SourceTable = "PO Row";
    layout { area(Content) { field(Id; Rec.Id) { } } }
}
codeunit 63604 T
{
    Subtype = Test;
    [Test] procedure FromCurrPage() var P: TestPage "PO Card"; Log: Codeunit "PO Log"; begin
        P.OpenView(); P.Ids.Invoke(); P.Close();
        if Log.Take() <> 'Page 63602|Page PO caption|Page PO caption' then Error('got %1', Log.Take()); end;
    [Test] procedure FromAVariable() var Pg: Page "PO Plain"; begin
        if Pg.ObjectId(false) + '|' + Pg.ObjectId(true) <> 'Page 63603|Page PO Plain' then Error('got %1', Pg.ObjectId(true)); end;
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
    for (name, outcome) in run_codeunit(&engine, &l, oi) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}
