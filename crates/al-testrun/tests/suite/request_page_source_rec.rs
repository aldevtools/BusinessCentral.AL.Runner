//! The Rec of a report request page that states SourceTable (al-runner issue 5756), pinned by corpus
//! report/lt-y-request-page/YRequestPageTests.al:443 RS_RequestPageRec_ModifyAndInsert_InOnOpenPage (OnOpenPage modifies and inserts
//! through Rec: both are stored, and the page opens on the row Rec was left on, `docno = N`) and :471
//! RS_RequestPageRec_ReadInReportTriggers (OnOpenPage moves Rec to the second row, the page opens on it, a value typed into a control
//! bound to Rec is written to the row at OK and the report triggers read it). What the corpus does not pin stays refused: a Rec left on
//! a key no row has, a filter set in OnOpenPage, an error out of OnOpenPage with the handler bound (RsRequestPageRec.al:153), and OK with a changed row on a table that runs code when it is modified.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50530 "RSR Line"
{
    fields { field(1; DocNo; Code[10]) { } field(2; LineNo; Integer) { } field(3; Descr; Text[30]) { } }
    keys { key(PK; DocNo, LineNo) { Clustered = true; } }
}
table 50531 "RSR Hooked"
{
    fields { field(1; DocNo; Code[10]) { } field(2; LineNo; Integer) { } field(3; Descr; Text[30]) { } }
    keys { key(PK; DocNo, LineNo) { Clustered = true; } }
    trigger OnModify() begin Descr := Descr + '!'; end;
}
codeunit 50530 "RSR Seen"
{
    SingleInstance = true;
    var Values: Dictionary of [Text, Text];
    procedure Put(K: Text; V: Text) begin Values.Set(K, V); end;
    procedure Get(K: Text): Text var V: Text; begin if Values.Get(K, V) then exit(V); exit('?'); end;
    procedure Clear() begin System.Clear(Values); end;
}
report 50531 "RSR Modify Found"
{
    ProcessingOnly = true;
    requestpage
    {
        SourceTable = "RSR Line";
        layout { area(Content) { field(DocNoCtl; Rec.DocNo) { } field(DescrCtl; Rec.Descr) { } } }
        trigger OnOpenPage()
        var Seen: Codeunit "RSR Seen";
        begin
            Rec.FindLast();
            Rec.Descr := 'modified';
            Seen.Put('modify', Format(Rec.Modify()));
            Rec.Init();
            Rec.DocNo := 'N';
            Rec.LineNo := 1;
            Rec.Descr := 'inserted';
            Seen.Put('insert', Format(Rec.Insert()));
        end;
    }
}
report 50532 "RSR Rec In Report"
{
    ProcessingOnly = true;
    requestpage
    {
        SourceTable = "RSR Line";
        layout { area(Content) { field(DocNoCtl; Rec.DocNo) { } field(DescrCtl; Rec.Descr) { } } }
        trigger OnOpenPage()
        begin
            Rec.FindFirst();
            Rec.Next();
        end;
    }
    trigger OnPreReport()
    var Seen: Codeunit "RSR Seen";
    begin
        Seen.Put('pre', '[' + Rec.DocNo + '/' + Format(Rec.LineNo) + '/' + Rec.Descr + ']');
    end;
}
report 50533 "RSR No Row"
{
    ProcessingOnly = true;
    requestpage
    {
        SourceTable = "RSR Line";
        layout { area(Content) { field(DocNoCtl; Rec.DocNo) { } } }
        trigger OnOpenPage()
        begin
            Rec.DocNo := 'Z';
        end;
    }
}
report 50534 "RSR Filtered"
{
    ProcessingOnly = true;
    requestpage
    {
        SourceTable = "RSR Line";
        layout { area(Content) { field(DocNoCtl; Rec.DocNo) { } } }
        trigger OnOpenPage()
        begin
            Rec.SetRange(DocNo, 'A');
            Rec.FindLast();
        end;
    }
}
report 50535 "RSR Hooked"
{
    ProcessingOnly = true;
    requestpage
    {
        SourceTable = "RSR Hooked";
        layout { area(Content) { field(DescrCtl; Rec.Descr) { } } }
    }
}
report 50536 "RSR Modify Raises"
{
    ProcessingOnly = true;
    requestpage
    {
        SourceTable = "RSR Line";
        trigger OnOpenPage()
        begin
            Rec.Modify();
        end;
    }
}
codeunit 50532 "RSR Helper"
{
    procedure Seed()
    var Line: Record "RSR Line"; Hooked: Record "RSR Hooked"; Seen: Codeunit "RSR Seen";
    begin
        Seen.Clear();
        Line.DeleteAll();
        Hooked.DeleteAll();
        Put('A', 10000, 'first');
        Put('A', 20000, 'second');
        Put('B', 10000, 'other');
        Hooked.DocNo := 'A';
        Hooked.LineNo := 1;
        Hooked.Descr := 'h';
        Hooked.Insert();
        Commit();
    end;

    local procedure Put(DocNo: Code[10]; LineNo: Integer; Descr: Text[30])
    var Line: Record "RSR Line";
    begin
        Line.DocNo := DocNo;
        Line.LineNo := LineNo;
        Line.Descr := Descr;
        Line.Insert();
    end;

    procedure Rows(): Text
    var Line: Record "RSR Line"; R: Text;
    begin
        if Line.FindSet() then
            repeat
                R += Line.DocNo + '/' + Format(Line.LineNo) + '/' + Line.Descr + ';';
            until Line.Next() = 0;
        exit(R);
    end;
}
codeunit 50533 "RSR Pinned"
{
    Subtype = Test;

    [Test]
    [HandlerFunctions('FoundHandler')]
    procedure ModifyAndInsert()
    var Helper: Codeunit "RSR Helper"; Seen: Codeunit "RSR Seen"; Got: Text;
    begin
        Helper.Seed();
        Report.RunModal(Report::"RSR Modify Found", true, false);
        Got := Seen.Get('modify') + ',' + Seen.Get('insert') + ',' + Seen.Get('docno') + ',' + Seen.Get('descr') + ',' + Helper.Rows();
        if Got <> 'Yes,Yes,N,inserted,A/10000/first;A/20000/second;B/10000/modified;N/1/inserted;' then
            Error('%1', Got);
    end;

    [RequestPageHandler]
    procedure FoundHandler(var RP: TestRequestPage "RSR Modify Found")
    var Seen: Codeunit "RSR Seen";
    begin
        Seen.Put('docno', RP.DocNoCtl.Value());
        Seen.Put('descr', RP.DescrCtl.Value());
        RP.Cancel().Invoke();
    end;

    [Test]
    [HandlerFunctions('TypeHandler')]
    procedure TypedAtOk()
    var Helper: Codeunit "RSR Helper"; Seen: Codeunit "RSR Seen"; Got: Text;
    begin
        Helper.Seed();
        Report.RunModal(Report::"RSR Rec In Report", true, false);
        Got := Seen.Get('docno') + ',' + Seen.Get('descr') + ',' + Seen.Get('pre') + ',' + Helper.Rows();
        if Got <> 'A,second,[A/20000/typed],A/10000/first;A/20000/typed;B/10000/other;' then
            Error('%1', Got);
    end;

    [RequestPageHandler]
    procedure TypeHandler(var RP: TestRequestPage "RSR Rec In Report")
    var Seen: Codeunit "RSR Seen";
    begin
        Seen.Put('docno', RP.DocNoCtl.Value());
        Seen.Put('descr', RP.DescrCtl.Value());
        RP.DescrCtl.SetValue('typed');
        RP.OK().Invoke();
    end;
}
codeunit 50534 "RSR No Row Test"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('H')]
    procedure NoRow()
    var Helper: Codeunit "RSR Helper";
    begin
        Helper.Seed();
        Report.RunModal(Report::"RSR No Row", true, false);
    end;
    [RequestPageHandler]
    procedure H(var RP: TestRequestPage "RSR No Row") begin RP.Cancel().Invoke(); end;
}
codeunit 50535 "RSR Filtered Test"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('H')]
    procedure Filtered()
    var Helper: Codeunit "RSR Helper";
    begin
        Helper.Seed();
        Report.RunModal(Report::"RSR Filtered", true, false);
    end;
    [RequestPageHandler]
    procedure H(var RP: TestRequestPage "RSR Filtered") begin RP.Cancel().Invoke(); end;
}
codeunit 50536 "RSR Hooked Test"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('H')]
    procedure Hooked()
    var Helper: Codeunit "RSR Helper";
    begin
        Helper.Seed();
        Report.RunModal(Report::"RSR Hooked", true, false);
    end;
    [RequestPageHandler]
    procedure H(var RP: TestRequestPage "RSR Hooked") begin RP.DescrCtl.SetValue('typed'); RP.OK().Invoke(); end;
}
codeunit 50537 "RSR Raises Test"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('H')]
    procedure Raises()
    var Helper: Codeunit "RSR Helper";
    begin
        Helper.Seed();
        Report.RunModal(Report::"RSR Modify Raises", true, false);
    end;
    [RequestPageHandler]
    procedure H(var RP: TestRequestPage "RSR Modify Raises") begin RP.Cancel().Invoke(); end;
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

fn outcome(rt: &CorpusRuntime, codeunit: u32, name: &str) -> Outcome {
    rt.first_outcome(&CaseId {
        codeunit,
        name: name.into(),
    })
}

#[test]
fn the_request_page_opens_on_the_row_onopenpage_left_rec_on_and_ok_writes_a_typed_value() {
    let rt = outcomes();
    assert_eq!(outcome(&rt, 50533, "ModifyAndInsert"), Outcome::Pass);
    assert_eq!(outcome(&rt, 50533, "TypedAtOk"), Outcome::Pass);
}

#[test]
fn what_the_corpus_does_not_pin_is_refused() {
    let rt = outcomes();
    for (codeunit, name, says) in [
        (50534, "NoRow", "key no row has"),
        (50535, "Filtered", "filter"),
        (50536, "Hooked", "changed row"),
        (50537, "Raises", "error out of the OnOpenPage"),
    ] {
        match outcome(&rt, codeunit, name) {
            Outcome::NotRun(r) => assert!(r.contains("request page") && r.contains(says), "{name}: {r}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}
