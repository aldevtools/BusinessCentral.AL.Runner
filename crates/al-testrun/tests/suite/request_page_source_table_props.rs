//! Properties of a report request page that has a source table (corpus report/lt-y-request-page/YRequestPageTests.al):
//! SourceTableView, SourceTableTemporary, ModifyAllowed/InsertAllowed/DeleteAllowed, DataCaptionFields/DataCaptionExpression,
//! CardPageId, LinksAllowed, PopulateAllFields and HelpLink. What the corpus does not pin stays refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50520 "RPS Line"
{
    fields { field(1; DocNo; Code[10]) { } field(2; LineNo; Integer) { } field(3; Descr; Text[30]) { } }
    keys { key(PK; DocNo, LineNo) { Clustered = true; } key(ByDescr; Descr) { } }
}
page 50520 "RPS Card" { PageType = Card; SourceTable = "RPS Line"; layout { area(Content) { field(D; Rec.DocNo) { } } } }
codeunit 50520 "RPS Seen"
{
    SingleInstance = true;
    var Values: Dictionary of [Text, Text];
    procedure Put(K: Text; V: Text) begin Values.Set(K, V); end;
    procedure Get(K: Text): Text var V: Text; begin if Values.Get(K, V) then exit(V); exit('?'); end;
}
report 50521 "RPS View"
{
    ProcessingOnly = true;
    dataset { dataitem(Line; "RPS Line") { } }
    requestpage
    {
        SourceTable = "RPS Line";
        SourceTableView = sorting(DocNo, LineNo) where(DocNo = const('A'));
        layout { area(Content) { field(DocNoCtl; Rec.DocNo) { } } }
        trigger OnOpenPage()
        var S: Codeunit "RPS Seen";
        begin
            S.Put('view', Rec.GetView());
            S.Put('filters', Rec.GetFilters());
            S.Put('count', Format(Rec.Count()));
        end;
    }
}
report 50522 "RPS Temp"
{
    ProcessingOnly = true;
    dataset { dataitem(Line; "RPS Line") { } }
    requestpage
    {
        SourceTable = "RPS Line";
        SourceTableTemporary = true;
        layout { area(Content) { field(DocNoCtl; Rec.DocNo) { } } }
        trigger OnOpenPage()
        var S: Codeunit "RPS Seen";
        begin
            S.Put('temporary', Format(Rec.IsTemporary()));
            S.Put('count_before', Format(Rec.Count()));
            Rec.Init();
            Rec.DocNo := 'T';
            Rec.Insert();
            S.Put('count_after', Format(Rec.Count()));
        end;
    }
}
report 50523 "RPS Locked"
{
    ProcessingOnly = true;
    dataset { dataitem(Line; "RPS Line") { } }
    requestpage
    {
        SourceTable = "RPS Line";
        DeleteAllowed = false;
        ModifyAllowed = false;
        InsertAllowed = false;
        layout { area(Content) { field(DocNoCtl; Rec.DocNo) { } field(DescrCtl; Rec.Descr) { } } }
    }
}
report 50524 "RPS Captions"
{
    ProcessingOnly = true;
    Caption = 'RPS request';
    dataset { dataitem(Line; "RPS Line") { } }
    requestpage
    {
        SourceTable = "RPS Line";
        DataCaptionFields = DocNo, Descr;
        DataCaptionExpression = 'Expression caption';
        CardPageId = "RPS Card";
        LinksAllowed = false;
        PopulateAllFields = true;
        HelpLink = 'https://example.com/rps';
        layout { area(Content) { field(DocNoCtl; Rec.DocNo) { } } }
    }
}
report 50525 "RPS Open"
{
    ProcessingOnly = true;
    dataset { dataitem(Line; "RPS Line") { } }
    requestpage
    {
        SourceTable = "RPS Line";
        ModifyAllowed = true;
        layout { area(Content) { field(DocNoCtl; Rec.DocNo) { } field(DescrCtl; Rec.Descr) { } } }
    }
}
report 50526 "RPS Other Sort"
{
    ProcessingOnly = true;
    dataset { dataitem(Line; "RPS Line") { } }
    requestpage
    {
        SourceTable = "RPS Line";
        SourceTableView = sorting(Descr);
        layout { area(Content) { field(DocNoCtl; Rec.DocNo) { } } }
    }
}
codeunit 50525 "RPS Tests"
{
    Subtype = Test;
    var S: Codeunit "RPS Seen";

    local procedure Seed()
    var L: Record "RPS Line";
    begin
        L.DeleteAll();
        L.DocNo := 'A'; L.LineNo := 10000; L.Descr := 'first'; L.Insert();
        L.DocNo := 'A'; L.LineNo := 20000; L.Descr := 'second'; L.Insert();
        L.DocNo := 'B'; L.LineNo := 10000; L.Descr := 'other'; L.Insert();
        Commit();
    end;

    local procedure Rows(): Text
    var L: Record "RPS Line"; R: Text;
    begin
        if L.FindSet() then
            repeat R += L.DocNo + '/' + Format(L.LineNo) + '/' + L.Descr + ';'; until L.Next() = 0;
        exit(R);
    end;

    local procedure Check(K: Text; Expected: Text)
    begin
        if S.Get(K) <> Expected then Error('%1: %2 <> %3', K, S.Get(K), Expected);
    end;

    // YRequestPageTests.al:72-87: the view's sorting, not its filter; the caption alone.
    [Test]
    [HandlerFunctions('ViewHandler')]
    procedure View()
    begin
        Seed();
        Report.RunRequestPage(Report::"RPS View");
        Check('view', 'VERSION(1) SORTING(DocNo,LineNo)');
        Check('filters', '');
        Check('count', '3');
        Check('caption', 'RPS View');
        Check('docno', 'A');
    end;

    [RequestPageHandler]
    procedure ViewHandler(var RP: TestRequestPage "RPS View")
    begin
        S.Put('caption', RP.Caption());
        S.Put('docno', RP.DocNoCtl.Value());
        RP.Cancel().Invoke();
    end;

    // YRequestPageTests.al:99-113.
    [Test]
    [HandlerFunctions('TempHandler')]
    procedure Temp()
    begin
        Seed();
        Report.RunRequestPage(Report::"RPS Temp");
        Check('temporary', 'Yes');
        Check('count_before', '0');
        Check('count_after', '1');
        if Rows() <> 'A/10000/first;A/20000/second;B/10000/other;' then Error('rows %1', Rows());
    end;

    [RequestPageHandler]
    procedure TempHandler(var RP: TestRequestPage "RPS Temp")
    begin
        RP.OK().Invoke();
    end;

    // YRequestPageTests.al:130-164.
    [Test]
    [HandlerFunctions('LockedHandler')]
    procedure Locked()
    begin
        Seed();
        Report.RunRequestPage(Report::"RPS Locked");
        Check('caption', 'RPS Locked');
        Check('editable', 'No');
        Check('modify', 'ok changed');
        Check('new', 'New method failed because Insert is not allowed. Page = RPS Locked, Id = 0.|DB:ClientInsertDenied');
        if Rows() <> 'A/10000/first;A/20000/second;B/10000/other;' then Error('rows %1', Rows());
    end;

    [RequestPageHandler]
    procedure LockedHandler(var RP: TestRequestPage "RPS Locked")
    begin
        S.Put('caption', RP.Caption());
        S.Put('editable', Format(RP.DescrCtl.Editable()));
        if TryModifyLocked(RP) then S.Put('modify', 'ok ' + RP.DescrCtl.Value()) else S.Put('modify', GetLastErrorText());
        if TryNewLocked(RP) then S.Put('new', 'ok') else S.Put('new', GetLastErrorText() + '|' + GetLastErrorCode());
        RP.Cancel().Invoke();
    end;

    [TryFunction]
    local procedure TryModifyLocked(var RP: TestRequestPage "RPS Locked") begin RP.DescrCtl.SetValue('changed'); end;

    [TryFunction]
    local procedure TryNewLocked(var RP: TestRequestPage "RPS Locked") begin RP.New(); end;

    // YRequestPageTests.al:218-257, 348-390: the request page shows the report's caption, whatever the data caption properties say.
    [Test]
    [HandlerFunctions('CaptionsHandler')]
    procedure Captions()
    begin
        Seed();
        Report.RunRequestPage(Report::"RPS Captions");
        Check('caption', 'RPS request');
        Check('docno', 'A');
    end;

    [RequestPageHandler]
    procedure CaptionsHandler(var RP: TestRequestPage "RPS Captions")
    begin
        S.Put('caption', RP.Caption());
        S.Put('docno', RP.DocNoCtl.Value());
        RP.Cancel().Invoke();
    end;

    // Not pinned: whether a request page writes a changed row it leaves for a new one.
    [Test]
    [HandlerFunctions('OpenHandler')]
    procedure NewAfterAChange()
    begin
        Seed();
        Report.RunRequestPage(Report::"RPS Open");
    end;

    [RequestPageHandler]
    procedure OpenHandler(var RP: TestRequestPage "RPS Open")
    begin
        RP.DescrCtl.SetValue('changed');
        RP.New();
        RP.Cancel().Invoke();
    end;

    // Not pinned: a SourceTableView whose sorting is not the primary key.
    [Test]
    [HandlerFunctions('OtherSortHandler')]
    procedure OtherSort()
    begin
        Seed();
        Report.RunRequestPage(Report::"RPS Other Sort");
    end;

    [RequestPageHandler]
    procedure OtherSortHandler(var RP: TestRequestPage "RPS Other Sort")
    begin
        RP.Cancel().Invoke();
    end;
}
"#;

fn outcomes() -> CorpusRuntime {
    CorpusRuntime::run_suites_with(
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
    )
}

fn outcome(rt: &CorpusRuntime, name: &str) -> Outcome {
    rt.first_outcome(&CaseId {
        codeunit: 50525,
        name: name.into(),
    })
}

#[test]
fn the_pinned_request_page_properties_run() {
    let rt = outcomes();
    let got: Vec<(&str, Outcome)> = ["View", "Temp", "Locked", "Captions"]
        .into_iter()
        .map(|n| (n, outcome(&rt, n)))
        .collect();
    assert!(got.iter().all(|(_, o)| *o == Outcome::Pass), "{got:?}");
}

#[test]
fn what_the_corpus_does_not_pin_is_refused() {
    let rt = outcomes();
    match outcome(&rt, "NewAfterAChange") {
        Outcome::NotRun(r) => assert!(r.contains("request page") && r.contains("changed row"), "{r}"),
        other => panic!("{other:?}"),
    }
    match outcome(&rt, "OtherSort") {
        Outcome::NotRun(r) => assert!(r.contains("SourceTableView"), "{r}"),
        other => panic!("{other:?}"),
    }
}
