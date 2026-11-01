//! `Page.Run(Page, Rec)` for a page that declares `SourceTableView`: Microsoft's "Assembly Orders"
//! (`SourceTableView = where("Document Type" = filter(Order))`) is run on a temporary record by
//! `Assemble-to-Order Link.ShowAsmOrders`; 31 tests of one bucket were refused. How the record and the view combine
//! is pinned by types/lt-code-digits-page-run-view/P145PageRunView.al (the page alone: handlers/TestPageSourceTableView.al):
//! the page shows the rows of the record that the view admits.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50100 "PV Row" { fields { field(1; Id; Integer) { } field(2; Kind; Option) { OptionMembers = A,B; } } keys { key(PK; Id) { } } }
page 50101 "PV List"
{
    PageType = List; SourceTable = "PV Row"; SourceTableView = where(Kind = const(B));
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
}
page 50103 "PV Desc List"
{
    PageType = List; SourceTable = "PV Row"; SourceTableView = sorting(Id) order(descending);
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
}
codeunit 50102 "T"
{
    Subtype = Test;
    var Seen: Integer; Ids: Text;
    [PageHandler]
    procedure H(var P: TestPage "PV List")
    begin
        if P.First() then
            repeat
                if P.Id.Value <> '0' then begin
                    Seen += 1;
                    Ids += P.Id.Value + ',';
                end;
            until not P.Next();
    end;
    [Test]
    procedure OpenViewAlone()
    var R: Record "PV Row"; P: TestPage "PV List"; n: Integer;
    begin
        R.Id := 21; R.Kind := R.Kind::B; R.Insert();
        R.Id := 22; R.Kind := R.Kind::A; R.Insert();
        R.Id := 23; R.Kind := R.Kind::B; R.Insert();
        P.OpenView();
        if P.First() then
            repeat
                n += 1;
            until not P.Next();
        if n <> 2 then Error('the page alone showed %1 rows', n);
    end;
    [PageHandler]
    procedure HDesc(var P: TestPage "PV Desc List")
    begin
        if P.First() then
            Ids := P.Id.Value;
    end;
    [Test]
    [HandlerFunctions('HDesc')]
    procedure RunOnARecordWithAnOrderInTheView()
    var R: Record "PV Row" temporary;
    begin
        R.Id := 1; R.Insert();
        R.Id := 2; R.Insert();
        R.Id := 3; R.Insert();
        Page.Run(Page::"PV Desc List", R);
        if Ids <> '3' then Error('the first row was %1', Ids);
    end;
    [Test]
    [HandlerFunctions('H')]
    procedure RunOnARecord()
    var R: Record "PV Row" temporary;
    begin
        R.Id := 1; R.Kind := R.Kind::B; R.Insert();
        R.Id := 2; R.Kind := R.Kind::A; R.Insert();
        R.Id := 3; R.Kind := R.Kind::B; R.Insert();
        Page.Run(Page::"PV List", R);
        if Seen <> 2 then Error('the page showed %1 rows: %2', Seen, Ids);
    end;
}
"#;

#[test]
fn a_page_run_on_a_record_shows_the_rows_its_source_table_view_admits() {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    let alone = rt.first_outcome(&CaseId {
        codeunit: 50102,
        name: "OpenViewAlone".into(),
    });
    assert_eq!(alone, Outcome::Pass);
    let o = rt.first_outcome(&CaseId {
        codeunit: 50102,
        name: "RunOnARecord".into(),
    });
    assert_eq!(o, Outcome::Pass);
    // A view with order(descending) on a page run on a record is read as the view alone is
    // (types/lt-code-digits-page-run-view/P145PageRunView.al:228-467, SF_B_*, SF_C_*).
    let o = rt.first_outcome(&CaseId {
        codeunit: 50102,
        name: "RunOnARecordWithAnOrderInTheView".into(),
    });
    assert_eq!(o, Outcome::Pass);
}
