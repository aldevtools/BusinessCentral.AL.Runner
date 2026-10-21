//! Report objects, end to end: processing without a request page or a layout. Each test names
//! the corpus test that pins it.
//!
//! The corpus reports drive their loop with `dataitem(X; Integer)`. The Integer virtual table is
//! a platform table the workspace does not have yet (ADR-013), so these fixtures use a real table
//! of numbers instead: "Num Row" holds the rows 1..4, and `where(Number = const(1))` picks one.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_vm::{Engine, Session, TxModel};

const FIXTURES: &str = r#"
table 60860 "Num Row"
{
    fields { field(1; Number; Integer) { } }
    keys { key(PK; Number) { } }
}

table 60866 "RRE Row"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[50]) { } }
    keys { key(PK; "Entry No.") { } key(ByName; Name) { } }
}

report 60899 "RU Probe"
{
    ProcessingOnly = true;
    UsageCategory = None;
    trigger OnPreReport()
    var Log: Record "RRE Log";
    begin
        Log.Log(Format(CurrReport.UseRequestPage()));
    end;
}

table 60867 "RRE Log"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Marker; Text[250]) { } }
    keys { key(PK; "Entry No.") { } }

    procedure Log(NewMarker: Text[250])
    var
        Log: Record "RRE Log";
        NextNo: Integer;
    begin
        if Log.FindLast() then
            NextNo := Log."Entry No.";
        Log.Init();
        Log."Entry No." := NextNo + 1;
        Log.Marker := NewMarker;
        Log.Insert();
    end;
}

table 60007 "ALT Base"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[50]) { } }
    keys { key(PK; "Entry No.") { } }
}

// corpus handlers/TestReportCurrReportLanguage.al, TestReportCurrReportLanguageReport.al
codeunit 60540 "CRL Log"
{
    SingleInstance = true;
    var Lang: Integer; Region: Text; Reads: Integer;
    procedure Note(L: Integer; R: Text) begin Lang := L; Region := R; Reads += 1; end;
    procedure GetLang(): Integer begin exit(Lang); end;
    procedure GetRegion(): Text begin exit(Region); end;
    procedure GetReads(): Integer begin exit(Reads); end;
}

report 60541 "CRL Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    UsageCategory = None;
    dataset
    {
        dataitem(Rows; "Num Row")
        {
            DataItemTableView = where(Number = const(1));
            trigger OnAfterGetRecord()
            var Log: Codeunit "CRL Log";
            begin
                CurrReport.Language := LangToSet;
                Log.Note(CurrReport.Language, RegionAfterSet);
            end;
        }
    }
    trigger OnPreReport()
    begin
        CurrReport.FormatRegion := 'en-US';
        RegionAfterSet := CurrReport.FormatRegion;
    end;
    procedure SetLang(L: Integer) begin LangToSet := L; end;
    var LangToSet: Integer; RegionAfterSet: Text;
}

// Sets the language first, then the region (misc/lt-probes-needed-3/PnC06ReportLanguage.al:58-65).
report 60542 "CRL Lang Then Region"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    UsageCategory = None;
    dataset
    {
        dataitem(Rows; "Num Row")
        {
            DataItemTableView = where(Number = const(1));
        }
    }
    trigger OnPreReport()
    var Log: Codeunit "CRL Log";
    begin
        CurrReport.Language := LangToSet;
        CurrReport.FormatRegion := RegionToSet;
        Log.Note(CurrReport.Language, CurrReport.FormatRegion);
    end;
    procedure Set(L: Integer; R: Text) begin LangToSet := L; RegionToSet := R; end;
    var LangToSet: Integer; RegionToSet: Text;
}

// corpus handlers/TestReportOnInitReportTimingFixtures.al
codeunit 60005 "OIR Log"
{
    SingleInstance = true;
    var InitCount: Integer; BodyCount: Integer; BodySaw: Text;
    procedure Reset() begin InitCount := 0; BodyCount := 0; BodySaw := ''; end;
    procedure NoteInit() begin InitCount += 1; end;
    procedure NoteBody(Seen: Text) begin BodyCount += 1; BodySaw := Seen; end;
    procedure GetInitCount(): Integer begin exit(InitCount); end;
    procedure GetBodyCount(): Integer begin exit(BodyCount); end;
    procedure GetBodySaw(): Text begin exit(BodySaw); end;
}

report 60005 "OIR Init Probe"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    UsageCategory = None;
    dataset
    {
        dataitem(I; "Num Row")
        {
            DataItemTableView = where(Number = const(1));
            trigger OnAfterGetRecord()
            begin
                Log.NoteBody(Setting);
            end;
        }
    }
    trigger OnInitReport()
    begin
        Log.NoteInit();
        Setting := 'from-oninitreport';
    end;
    procedure SetSetting(NewSetting: Text) begin Setting := NewSetting; end;
    var Log: Codeunit "OIR Log"; Setting: Text;
}

report 60006 "OIR Quit Probe"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    UsageCategory = None;
    dataset
    {
        dataitem(I; "Num Row")
        {
            DataItemTableView = where(Number = const(1));
            trigger OnAfterGetRecord()
            begin
                Log.NoteBody('body-ran');
            end;
        }
    }
    trigger OnInitReport()
    begin
        Log.NoteInit();
        CurrReport.Quit();
    end;
    trigger OnPreReport()
    begin
        Log.NoteBody('onprereport-ran');
    end;
    var Log: Codeunit "OIR Log";
}

// corpus handlers/RreProcessingOnlyReport.Report.al
report 60868 "RRE ProcessingOnly Report"
{
    Caption = 'RRE ProcessingOnly Report';
    UsageCategory = ReportsAndAnalysis;
    ApplicationArea = All;
    ProcessingOnly = true;
    dataset
    {
        dataitem(Rows; "RRE Row")
        {
            trigger OnAfterGetRecord()
            var LogRec: Record "RRE Log";
            begin
                RowCount += 1;
                LogRec.Log('A-row');
            end;
        }
    }
    var RowCount: Integer; PreReportRan: Boolean; PostReportRan: Boolean;
    trigger OnPreReport()
    var LogRec: Record "RRE Log";
    begin
        PreReportRan := true;
        LogRec.Log('A-pre');
    end;
    trigger OnPostReport()
    var LogRec: Record "RRE Log";
    begin
        PostReportRan := true;
        LogRec.Log('A-post');
    end;
    procedure RowsProcessed(): Integer begin exit(RowCount); end;
}

// corpus handlers/RreSetTableViewReport.Report.al
report 60913 "STV Counting Report"
{
    Caption = 'STV Counting Report';
    UsageCategory = ReportsAndAnalysis;
    ApplicationArea = All;
    ProcessingOnly = true;
    dataset
    {
        dataitem(Rows; "RRE Row")
        {
            trigger OnAfterGetRecord()
            var LogRec: Record "RRE Log";
            begin
                LogRec.Log(CopyStr('STV-' + Rows.Name, 1, 50));
            end;
        }
    }
}

// A DataItemTableView that filters, next to a SetTableView (Whse.-Source - Create Document and its callers; about 800
// Microsoft tests). Both apply (report/lt-dataitem-view/DataItemTableView.al pins it).
report 60914 "STW Filtered Report"
{
    UsageCategory = ReportsAndAnalysis;
    ApplicationArea = All;
    ProcessingOnly = true;
    dataset
    {
        dataitem(Rows; "RRE Row")
        {
            DataItemTableView = sorting("Entry No.") where("Entry No." = filter(1 .. 2));
            trigger OnAfterGetRecord()
            var LogRec: Record "RRE Log";
            begin
                LogRec.Log(CopyStr('STW-' + Rows.Name, 1, 50));
            end;
        }
    }
}

// corpus handlers/RgfFilterReport.Report.al
report 60998 "RGF Filter Report"
{
    Caption = 'RGF Filter Report';
    UsageCategory = ReportsAndAnalysis;
    ApplicationArea = All;
    ProcessingOnly = true;
    dataset
    {
        dataitem(Rows; "RRE Row")
        {
            trigger OnPreDataItem()
            var LogRec: Record "RRE Log";
            begin
                LogRec.Log(CopyStr('RGF-PRE-DI:' + Rows.GetFilter("Entry No."), 1, 50));
            end;
        }
    }
    trigger OnPreReport()
    var LogRec: Record "RRE Log";
    begin
        LogRec.Log(CopyStr('RGF-PRE-REPORT:' + Rows.GetFilter("Entry No."), 1, 50));
    end;
}

// corpus record/TestTxModelReportRunMarker.Reports.al, with a one-row table for Integer.
report 60026 "TxRpt ReqPage Marker"
{
    Caption = 'TxRpt ReqPage Marker';
    ProcessingOnly = true;
    dataset
    {
        dataitem(Loop; "Num Row")
        {
            DataItemTableView = sorting(Number) where(Number = const(1));
            trigger OnAfterGetRecord()
            var ALTBase: Record "ALT Base";
            begin
                ALTBase.Init(); ALTBase."Entry No." := MarkerNo; ALTBase.Insert();
            end;
        }
    }
    requestpage
    {
        layout { area(Content) { group(Options) { field(EchoText; EchoText) { ApplicationArea = All; } } } }
    }
    var MarkerNo: Integer; EchoText: Text[30];
    procedure SetMarker(NewMarkerNo: Integer) begin MarkerNo := NewMarkerNo; end;
}

report 60027 "TxRpt Update Marker"
{
    Caption = 'TxRpt Update Marker';
    ProcessingOnly = true;
    UseRequestPage = false;
    TransactionType = Update;
    dataset
    {
        dataitem(Loop; "Num Row")
        {
            DataItemTableView = sorting(Number) where(Number = const(1));
            trigger OnAfterGetRecord()
            var ALTBase: Record "ALT Base";
            begin
                ALTBase.Init(); ALTBase."Entry No." := MarkerNo; ALTBase.Insert();
            end;
        }
    }
    var MarkerNo: Integer;
    procedure SetMarker(NewMarkerNo: Integer) begin MarkerNo := NewMarkerNo; end;
}

report 60028 "TxRpt Plain Marker"
{
    Caption = 'TxRpt Plain Marker';
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(Loop; "Num Row")
        {
            DataItemTableView = sorting(Number) where(Number = const(1));
            trigger OnAfterGetRecord()
            var ALTBase: Record "ALT Base";
            begin
                ALTBase.Init(); ALTBase."Entry No." := MarkerNo; ALTBase.Insert();
            end;
        }
    }
    var MarkerNo: Integer;
    procedure SetMarker(NewMarkerNo: Integer) begin MarkerNo := NewMarkerNo; end;
}

report 60029 "TxRpt Error Marker"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(Loop; "Num Row")
        {
            DataItemTableView = sorting(Number) where(Number = const(1));
            trigger OnAfterGetRecord()
            var ALTBase: Record "ALT Base";
            begin
                ALTBase.Init(); ALTBase."Entry No." := 77; ALTBase.Insert();
                Error('txrpt report body error');
            end;
        }
    }
}

// Not from the corpus: a parent and a child over real tables, linked.
table 60870 "Link Parent" { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
table 60871 "Link Child" { fields { field(1; Id; Integer) { } field(2; ParentNo; Integer) { } } keys { key(PK; Id) { } } }

report 60880 "MX Link Filter"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(Parent; "Link Parent")
        {
            dataitem(Child; "Link Child")
            {
                DataItemLink = ParentNo = field(No);
                DataItemTableView = where(ParentNo = const(1));
            }
        }
    }
}

report 60881 "MX Quit In Pre"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    trigger OnPreReport() begin CurrReport.Quit(); end;
}

report 60872 "Link Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(Parent; "Link Parent")
        {
            trigger OnPreDataItem() var L: Record "RRE Log"; begin L.Log('parent-pre'); end;
            trigger OnAfterGetRecord() var L: Record "RRE Log"; begin L.Log('parent-' + Format(Parent.No)); end;
            trigger OnPostDataItem() var L: Record "RRE Log"; begin L.Log('parent-post'); end;
            dataitem(Child; "Link Child")
            {
                DataItemLink = ParentNo = field(No);
                trigger OnPreDataItem() var L: Record "RRE Log"; begin L.Log('child-pre'); end;
                trigger OnAfterGetRecord() var L: Record "RRE Log"; begin L.Log('child-' + Format(Child.Id)); end;
                trigger OnPostDataItem() var L: Record "RRE Log"; begin L.Log('child-post'); end;
            }
        }
    }
    trigger OnPreReport() var L: Record "RRE Log"; begin L.Log('pre-report'); end;
    trigger OnPostReport() var L: Record "RRE Log"; begin L.Log('post-report'); end;
}

report 60873 "Sibling Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(First; "Link Parent") { trigger OnAfterGetRecord() var L: Record "RRE Log"; begin L.Log('first-' + Format(First.No)); end; }
        dataitem(Second; "Link Parent") { trigger OnAfterGetRecord() var L: Record "RRE Log"; begin L.Log('second-' + Format(Second.No)); end; }
    }
}

report 60874 "Skipper Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(Rows; "RRE Row") { trigger OnAfterGetRecord() begin CurrReport.Skip(); end; }
    }
}

report 60875 "Layout Report"
{
    DefaultRenderingLayout = Wordish;
    UseRequestPage = false;
    dataset { dataitem(Rows; "RRE Row") { column(EntryNo; "Entry No.") { } } }
    rendering { layout(Wordish) { Type = RDLC; LayoutFile = './x.rdl'; } }
}

report 60876 "Default Page Report"
{
    ProcessingOnly = true;
    dataset { dataitem(Rows; "RRE Row") { } }
}


// corpus reportextensiontrigger/TestReportExtensionTriggers.al, with a table of numbers for Integer.
codeunit 68000 "RXT Log"
{
    SingleInstance = true;
    var Entries: Dictionary of [Text, Text]; FailExtPreReport: Boolean;
    procedure Reset() begin if Entries.Remove('Report') then; if Entries.Remove('Modify') then; if Entries.Remove('Added') then; FailExtPreReport := false; end;
    procedure Add(Category: Text; Entry: Text)
    var Existing: Text;
    begin
        if Entries.Get(Category, Existing) then
            Entries.Set(Category, Existing + '|' + Entry)
        else
            Entries.Add(Category, Entry);
    end;
    procedure Read(Category: Text): Text
    var Existing: Text;
    begin
        if Entries.Get(Category, Existing) then exit(Existing);
        exit('');
    end;
    procedure SetFailExtPreReport(Fail: Boolean) begin FailExtPreReport := Fail; end;
    procedure ShouldFailExtPreReport(): Boolean begin exit(FailExtPreReport); end;
}

report 68000 "RXT Proc Report"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(BaseItem; "Num Row")
        {
            DataItemTableView = sorting(Number) where(Number = filter(1 .. 2));
            trigger OnAfterGetRecord()
            begin
                Log.Add('Modify', 'base-' + Format(Number));
            end;
        }
    }
    trigger OnPreReport() begin Log.Add('Report', 'base-pre'); end;
    trigger OnPostReport() begin Log.Add('Report', 'base-post'); end;
    var Log: Codeunit "RXT Log";
}

reportextension 68000 "RXT Proc Report Ext" extends "RXT Proc Report"
{
    dataset
    {
        modify(BaseItem)
        {
            trigger OnBeforeAfterGetRecord() begin Log.Add('Modify', 'ext-before-' + Format(BaseItem.Number)); end;
            trigger OnAfterAfterGetRecord() begin Log.Add('Modify', 'ext-after-' + Format(BaseItem.Number)); end;
        }
        addlast(BaseItem)
        {
            dataitem(ExtItem; "Num Row")
            {
                DataItemTableView = sorting(Number) where(Number = filter(1 .. 3));
                trigger OnPreDataItem() begin Log.Add('Added', 'pre'); end;
                trigger OnAfterGetRecord() begin Log.Add('Added', 'row-' + Format(Number)); end;
                trigger OnPostDataItem() begin Log.Add('Added', 'post'); end;
            }
        }
    }
    trigger OnPreReport()
    begin
        Log.Add('Report', 'ext-pre');
        if Log.ShouldFailExtPreReport() then
            Error('RXT extension OnPreReport failed');
    end;
    trigger OnPostReport() begin Log.Add('Report', 'ext-post'); end;
    var Log: Codeunit "RXT Log";
}

// corpus extensionevents/ALTExtEvtFixtures.al:104-125: a reportextension that declares a
// procedure and an event, a subscriber bound through the base report, and a sink.
report 68010 "Ext Proc Base"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(Rows; "Num Row") { } }
}
reportextension 68010 "Ext Proc Ext" extends "Ext Proc Base"
{
    procedure RaiseReportExt(Value: Integer) begin OnReportExtEvent(Value); end;

    [IntegrationEvent(false, false)]
    local procedure OnReportExtEvent(Value: Integer) begin end;
}
codeunit 68012 "Ext Proc Sink"
{
    SingleInstance = true;
    var Count: Integer; Last: Integer;
    procedure Reset() begin Count := 0; Last := 0; end;
    procedure Note(Value: Integer) begin Count += 1; Last := Value; end;
    procedure Seen(): Text begin exit(Format(Count) + ':' + Format(Last)); end;
}
codeunit 68013 "Ext Proc Subscriber"
{
    [EventSubscriber(ObjectType::Report, Report::"Ext Proc Base", 'OnReportExtEvent', '', false, false)]
    local procedure OnReportExtEvent(Value: Integer)
    var Sink: Codeunit "Ext Proc Sink";
    begin
        Sink.Note(Value);
    end;
}

// An extension with a procedure and a global variable: what state a variable of the base report
// keeps for the extension is not pinned, so it is refused.
report 68014 "Ext State Base"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(Rows; "Num Row") { } }
}
reportextension 68014 "Ext State Ext" extends "Ext State Base"
{
    procedure Bump() begin Count += 1; end;
    var Count: Integer;
}

// An empty extension, as the corpus has several (_fixtures/ALTExtensionSubtypeFixtures.al:16-26).
reportextension 68011 "Empty Ext" extends "RRE ProcessingOnly Report"
{
}

// Data items an extension places around nested ones (own fixture; the corpus pins the same rules in
// reportextensiontrigger/TestReportExtensionDataItemPlacementContracts.al).
report 68020 "RXP Base"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset
    {
        dataitem(Parent; "Num Row")
        {
            DataItemTableView = sorting(Number) where(Number = const(1));
            trigger OnAfterGetRecord() begin Log.Add('Place', 'Parent'); end;
            dataitem(ChildA; "Num Row")
            {
                DataItemTableView = sorting(Number) where(Number = const(1));
                trigger OnAfterGetRecord() begin Log.Add('Place', 'ChildA'); end;
            }
            dataitem(ChildB; "Num Row")
            {
                DataItemTableView = sorting(Number) where(Number = const(1));
                trigger OnAfterGetRecord() begin Log.Add('Place', 'ChildB'); end;
            }
        }
    }
    var Log: Codeunit "RXT Log";
}
reportextension 68020 "RXP Ext" extends "RXP Base"
{
    dataset
    {
        addlast(Parent) { dataitem(ExtLast; "Num Row") { DataItemTableView = sorting(Number) where(Number = const(1)); trigger OnAfterGetRecord() begin Log.Add('Place', 'ExtLast'); end; } }
        addafter(ChildA) { dataitem(ExtAfterA; "Num Row") { DataItemTableView = sorting(Number) where(Number = const(1)); trigger OnAfterGetRecord() begin Log.Add('Place', 'ExtAfterA'); end; } }
        addbefore(ChildB) { dataitem(ExtBeforeB; "Num Row") { DataItemTableView = sorting(Number) where(Number = const(1)); trigger OnAfterGetRecord() begin Log.Add('Place', 'ExtBeforeB'); end; } }
        addfirst(Parent) { dataitem(ExtFirst; "Num Row") { DataItemTableView = sorting(Number) where(Number = const(1)); trigger OnAfterGetRecord() begin Log.Add('Place', 'ExtFirst'); end; } }
        addbefore(ChildA) { dataitem(ExtBeforeA; "Num Row") { DataItemTableView = sorting(Number) where(Number = const(1)); trigger OnAfterGetRecord() begin Log.Add('Place', 'ExtBeforeA'); end; } }
    }
    var Log: Codeunit "RXT Log";
}

codeunit 50110 Helpers
{
    procedure Seed()
    var R: Record "RRE Row"; L: Record "RRE Log"; N: Record "Num Row"; i: Integer;
    begin
        R.DeleteAll(); L.DeleteAll(); N.DeleteAll();
        R.Init(); R."Entry No." := 1; R.Name := 'first'; R.Insert();
        R.Init(); R."Entry No." := 2; R.Name := 'second'; R.Insert();
        R.Init(); R."Entry No." := 3; R.Name := 'third'; R.Insert();
        for i := 1 to 4 do begin N.Number := i; N.Insert(); end;
    end;

    procedure Visited(Prefix: Text): Text
    var L: Record "RRE Log"; Out: Text;
    begin
        L.SetFilter(Marker, Prefix + '*');
        if L.FindSet() then
            repeat
                if Out <> '' then Out += ',';
                Out += L.Marker;
            until L.Next() = 0;
        exit(Out);
    end;

    procedure Count(M: Text): Integer
    var L: Record "RRE Log";
    begin
        L.SetRange(Marker, M);
        exit(L.Count());
    end;

    procedure Has(EntryNo: Integer): Boolean
    var B: Record "ALT Base";
    begin
        exit(B.Get(EntryNo));
    end;

    procedure Insert(EntryNo: Integer)
    var B: Record "ALT Base";
    begin
        B.Init(); B."Entry No." := EntryNo; B.Insert();
    end;
}
"#;

fn build(body: &str) -> (Engine0, usize, usize) {
    let src = format!("{FIXTURES}\ncodeunit 50100 C {{ {body} }}");
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    (
        Engine0 {
            program: l.program,
            blocked,
        },
        oi,
        ri,
    )
}

struct Engine0 {
    program: std::sync::Arc<al_ir::Program>,
    blocked: Vec<Vec<Option<String>>>,
}

fn run_model(body: &str, model: TxModel) -> Result<Value, Signal> {
    let (e, oi, ri) = build(body);
    if let Some(Some(r)) = e.blocked.get(oi).and_then(|o| o.get(ri)) {
        return Err(Signal::Unsupported(r.clone()));
    }
    let setup = e.program.objects[oi].routines.iter().position(|r| r.name == "Setup");
    let engine = Engine::new(e.program, e.blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    // `Setup` seeds and commits under the default model, before the test's model is in force.
    if let Some(setup) = setup {
        s.call(oi, setup, inst.clone(), vec![]).expect("Setup");
        s.commit_transaction();
    }
    s.tx_model = model;
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn text_model(vars_and_body: &str, model: TxModel) -> String {
    match run_model(&format!("procedure F(): Text {vars_and_body}"), model) {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

/// Like `text_model`, with a `Setup` that seeds and commits before the test's model is in force.
fn text_after_setup(vars_and_body: &str, model: TxModel) -> String {
    let body =
        format!("procedure Setup() var H: Codeunit Helpers; begin H.Seed(); end; procedure F(): Text {vars_and_body}");
    match run_model(&body, model) {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

fn f(vars_and_body: &str) -> String {
    text_model(vars_and_body, TxModel::AutoCommit)
}

fn refusal(vars_and_body: &str) -> String {
    match run_model(&format!("procedure F(): Text {vars_and_body}"), TxModel::AutoCommit) {
        Err(Signal::Unsupported(r)) => r,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// Base Application's Copy Sales Document reads `CurrReport.UseRequestPage()` in OnPreReport: the running report, no variable
// (186 Tests-ERM tests). The setter on a variable is pinned by handlers/TestReportRuntimeContracts.al:416-440.
#[test]
fn currreport_use_request_page_is_the_running_reports() {
    assert_eq!(
        f(
            "var R: Report \"RU Probe\"; L: Record \"RRE Log\"; begin R.UseRequestPage(false); R.Run(); L.FindLast(); exit(L.Marker); end;"
        ),
        "No"
    );
}

// ---------------------------------------------------------------- OnInitReport

// handlers/TestReportOnInitReportTiming.al:ReportVarRun_OnInitReportRunsOnceBeforeTheBody (:36-47)
#[test]
fn run_runs_oninitreport_once_before_the_body() {
    assert_eq!(
        f(
            "var Probe: Report \"OIR Init Probe\"; Log: Codeunit \"OIR Log\"; H: Codeunit Helpers; begin
            H.Seed();
            Log.Reset();
            Probe.Run();
            exit(Format(Log.GetInitCount()) + '|' + Format(Log.GetBodyCount()) + '|' + Log.GetBodySaw());
        end;"
        ),
        "1|1|from-oninitreport"
    );
}

// handlers/TestReportOnInitReportTiming.al:ReportVarFirstUse_RunsOnInitReport_BeforeRunIsCalled (:50-60)
#[test]
fn the_first_use_of_a_report_variable_runs_oninitreport() {
    assert_eq!(
        f(
            "var Probe: Report \"OIR Init Probe\"; Log: Codeunit \"OIR Log\"; H: Codeunit Helpers; begin
            H.Seed();
            Log.Reset();
            Probe.SetSetting('from-caller');
            exit(Format(Log.GetInitCount()) + '|' + Format(Log.GetBodyCount()));
        end;"
        ),
        "1|0"
    );
}

// handlers/TestReportOnInitReportTiming.al:ReportVarSetterBeforeRun_SurvivesOnInitReport (:63-74)
#[test]
fn a_value_set_before_run_survives_oninitreport() {
    assert_eq!(
        f(
            "var Probe: Report \"OIR Init Probe\"; Log: Codeunit \"OIR Log\"; H: Codeunit Helpers; begin
            H.Seed();
            Log.Reset();
            Probe.SetSetting('from-caller');
            Probe.Run();
            exit(Log.GetBodySaw() + '|' + Format(Log.GetInitCount()));
        end;"
        ),
        "from-caller|1"
    );
}

// handlers/TestReportOnInitReportTiming.al:StaticReportRun_OnInitReportRunsOnce (:77-85)
#[test]
fn the_static_run_runs_oninitreport_once() {
    assert_eq!(
        f("var Log: Codeunit \"OIR Log\"; H: Codeunit Helpers; begin
            H.Seed();
            Log.Reset();
            Report.Run(Report::\"OIR Init Probe\");
            exit(Format(Log.GetInitCount()) + '|' + Log.GetBodySaw());
        end;"),
        "1|from-oninitreport"
    );
}

// handlers/TestReportOnInitReportTiming.al:QuitInOnInitReport_StopsPreReportAndBody (:88-98)
#[test]
fn quit_in_oninitreport_stops_the_prereport_trigger_and_the_body() {
    assert_eq!(
        f(
            "var Probe: Report \"OIR Quit Probe\"; Log: Codeunit \"OIR Log\"; H: Codeunit Helpers; begin
            H.Seed();
            Log.Reset();
            Probe.Run();
            exit(Format(Log.GetInitCount()) + '|' + Format(Log.GetBodyCount()));
        end;"
        ),
        "1|0"
    );
}

// ---------------------------------------------------------------- static Run and RunModal

// handlers/TestReportRunExecution.al:StaticRun_ExecutesTriggersAndBody (:128-142)
#[test]
fn the_static_run_executes_the_triggers_and_the_body() {
    assert_eq!(
        f("var H: Codeunit Helpers; begin
            H.Seed();
            Report.Run(Report::\"RRE ProcessingOnly Report\", false, false);
            exit(Format(H.Count('A-pre')) + '|' + Format(H.Count('A-row')));
        end;"),
        "1|3"
    );
}

// handlers/TestReportRunExecution.al:StaticRunModal_ExecutesTriggersAndBody (:145-158)
#[test]
fn the_static_runmodal_executes_the_triggers_and_the_body() {
    assert_eq!(
        f("var H: Codeunit Helpers; begin
            H.Seed();
            Report.RunModal(Report::\"RRE ProcessingOnly Report\", false, false);
            exit(Format(H.Count('A-pre')) + '|' + Format(H.Count('A-row')));
        end;"),
        "1|3"
    );
}

// handlers/TestReportRunExecution.al:StaticRun_ExecutesTriggersAndBody; OnPostReport after the
// body is pinned only in its order against OnPreReport (reportextensiontrigger/TestReportExtensionTriggers.al:244-250).
#[test]
fn the_report_triggers_run_in_order() {
    assert_eq!(
        f("var H: Codeunit Helpers; begin
            H.Seed();
            Report.Run(Report::\"RRE ProcessingOnly Report\", false, false);
            exit(H.Visited('A-'));
        end;"),
        "A-pre,A-row,A-row,A-row,A-post"
    );
}

// ---------------------------------------------------------------- SetTableView

// handlers/TestReportSetTableView.al:Report_SetTableView_FiltersDataItemToTheAppliedView (:72-86)
#[test]
fn settableview_restricts_the_data_item_to_the_view() {
    assert_eq!(
        f(
            "var H: Codeunit Helpers; Row: Record \"RRE Row\"; R: Report \"STV Counting Report\"; begin
            H.Seed();
            Row.SetRange(\"Entry No.\", 1, 2);
            R.SetTableView(Row);
            R.UseRequestPage(false);
            R.Run();
            exit(H.Visited('STV-'));
        end;"
        ),
        "STV-first,STV-second"
    );
}

// The view of SetTableView and the report's own filtering DataItemTableView both apply: the rows are those that pass
// both (report/lt-dataitem-view/DataItemTableView.al pins it).
#[test]
fn settableview_and_a_filtering_dataitemtableview_both_apply() {
    assert_eq!(
        f(
            "var H: Codeunit Helpers; Row: Record \"RRE Row\"; R: Report \"STW Filtered Report\"; begin
            H.Seed();
            Row.SetRange(\"Entry No.\", 2, 3);
            R.SetTableView(Row);
            R.UseRequestPage(false);
            R.Run();
            exit(H.Visited('STW-'));
        end;"
        ),
        "STW-second"
    );
}

// handlers/TestReportSetTableView.al:Report_Run_WithoutSetTableView_IteratesEveryRowInTheTable (:89-104)
#[test]
fn without_a_view_the_data_item_iterates_every_row() {
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"STV Counting Report\"; begin
            H.Seed();
            R.UseRequestPage(false);
            R.Run();
            exit(H.Visited('STV-'));
        end;"),
        "STV-first,STV-second,STV-third"
    );
}

// ---------------------------------------------------------------- GetFilter inside triggers

// handlers/TestReportGetFilterInTriggers.al:Report_InstanceSetTableView_GetFilter_ReadsAppliedFilter_InOnPreReportAndOnPreDataItem (:58-73)
#[test]
fn the_view_is_visible_to_getfilter_in_onprereport_and_onpredataitem() {
    assert_eq!(
        f(
            "var H: Codeunit Helpers; Row: Record \"RRE Row\"; Probe: Report \"RGF Filter Report\"; begin
            H.Seed();
            Row.SetRange(\"Entry No.\", 1, 2);
            Probe.SetTableView(Row);
            Probe.UseRequestPage(false);
            Probe.RunModal();
            exit(H.Visited('RGF-'));
        end;"
        ),
        "RGF-PRE-REPORT:1..2,RGF-PRE-DI:1..2"
    );
}

// handlers/TestReportGetFilterInTriggers.al:Report_StaticRunModalRecordParam_GetFilter_ReadsAppliedFilter_InOnPreReportAndOnPreDataItem (:76-89)
#[test]
fn the_static_record_parameter_is_the_view() {
    assert_eq!(
        f("var H: Codeunit Helpers; Row: Record \"RRE Row\"; begin
            H.Seed();
            Row.Get(1);
            Row.SetRecFilter();
            Report.RunModal(Report::\"RGF Filter Report\", false, false, Row);
            exit(H.Visited('RGF-'));
        end;"),
        "RGF-PRE-REPORT:1,RGF-PRE-DI:1"
    );
}

// handlers/TestReportGetFilterInTriggers.al:Report_NoFilterApplied_GetFilter_ReadsEmpty_InOnPreReportAndOnPreDataItem (:92-108)
#[test]
fn without_a_view_getfilter_reads_empty() {
    assert_eq!(
        f("var H: Codeunit Helpers; Probe: Report \"RGF Filter Report\"; begin
            H.Seed();
            Probe.UseRequestPage(false);
            Probe.RunModal();
            exit(H.Visited('RGF-'));
        end;"),
        "RGF-PRE-REPORT:,RGF-PRE-DI:"
    );
}

// ---------------------------------------------------------------- DataItemTableView, nesting

// handlers/TestReportMultiDataItemNested.al:5-25 (the inner item runs again for every outer row)
// and handlers/TestReportNestedLinkImplicitParent.al:113-134 (a DataItemLink without a reference
// joins to the enclosing item; each parent row sees only its own children).
#[test]
fn a_nested_item_runs_once_per_parent_row_over_the_rows_linked_to_it() {
    assert_eq!(
        f(
            "var H: Codeunit Helpers; P: Record \"Link Parent\"; C: Record \"Link Child\"; begin
            H.Seed();
            P.No := 1; P.Insert(); P.No := 2; P.Insert();
            C.Id := 10; C.ParentNo := 1; C.Insert();
            C.Id := 11; C.ParentNo := 1; C.Insert();
            C.Id := 12; C.ParentNo := 2; C.Insert();
            Report.Run(Report::\"Link Report\", false, false);
            exit(H.Visited(''));
        end;"
        ),
        // The relative order of a parent's OnAfterGetRecord and its children is not pinned by
        // the corpus (docs/probes-needed.md); this is the documented BC order.
        "pre-report,parent-pre,parent-1,child-pre,child-10,child-11,child-post,parent-2,child-pre,child-12,child-post,parent-post,post-report"
    );
}

// handlers/TestReportMultiDataItemSiblings.al:5-25: sibling items run one after the other.
#[test]
fn sibling_items_run_one_after_the_other() {
    assert_eq!(
        f("var H: Codeunit Helpers; P: Record \"Link Parent\"; begin
            H.Seed();
            P.No := 1; P.Insert(); P.No := 2; P.Insert();
            Report.Run(Report::\"Sibling Report\", false, false);
            exit(H.Visited(''));
        end;"),
        "first-1,first-2,second-1,second-2"
    );
}

// ---------------------------------------------------------------- transactions

// record/TestTxModelReportRun.al:TxReportRun_Test03_PlainReportWithPendingWriteRunsAndIsNotACommit (:94-114)
#[test]
fn a_plain_report_joins_the_callers_transaction() {
    assert_eq!(
        f("var H: Codeunit Helpers; Plain: Report \"TxRpt Plain Marker\"; begin
            H.Seed(); Commit();
            H.Insert(60040031);
            Plain.SetMarker(60040032);
            Plain.Run();
            if not H.Has(60040032) then exit('body did not run');
            asserterror Error('txrpt plain probe');
            exit(Format(H.Has(60040031)) + '|' + Format(H.Has(60040032)));
        end;"),
        "No|No"
    );
}

// record/TestTxModelReportRun.al:TxReportRun_Test07_TransactionTypeUpdateReportCommitsItsOwnWrites (:177-196)
#[test]
fn an_update_report_commits_its_own_writes() {
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"TxRpt Update Marker\"; begin
            H.Seed(); Commit();
            R.SetMarker(60040071);
            R.Run();
            H.Insert(60040072);
            asserterror Error('txrpt update probe');
            exit(Format(H.Has(60040071)) + '|' + Format(H.Has(60040072)));
        end;"),
        "Yes|No"
    );
}

// record/TestTxModelReportRun.al:TxReportRun_Test08_TransactionTypeUpdateReportWithPendingWriteIsRefused (:199-211)
#[test]
fn an_update_report_is_refused_while_the_caller_has_an_uncommitted_write() {
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"TxRpt Update Marker\"; begin
            H.Seed(); Commit();
            H.Insert(60040081);
            R.SetMarker(60040082);
            asserterror R.Run();
            exit(GetLastErrorText() + '|' + Format(H.Has(60040082)) + '|' + Format(H.Has(60040081)));
        end;"),
        "An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.|No|No"
    );
}

// record/TestTxModelReportRun.al:TxReportRun_Test04 and Test05 (:117-148): a report that uses a
// request page enters a transaction world before the page is shown, and is refused with a
// pending write. Run and RunModal alike.
#[test]
fn a_request_page_report_is_refused_while_the_caller_has_an_uncommitted_write() {
    for call in ["R.Run()", "R.RunModal()"] {
        assert_eq!(
            f(&format!(
                "var H: Codeunit Helpers; R: Report \"TxRpt ReqPage Marker\"; begin
                    H.Seed(); Commit();
                    H.Insert(60040041);
                    R.SetMarker(60040042);
                    asserterror {call};
                    exit(GetLastErrorText() + '|' + Format(H.Has(60040042)) + '|' + Format(H.Has(60040041)));
                end;"
            )),
            "An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.|No|No",
            "{call}"
        );
    }
}

// codeunit/TestWriteTxTestBoundary.al:WriteTxBoundary_Test11_AReportMayWriteUnderNone (:259-279)
#[test]
fn a_report_may_write_under_transactionmodel_none() {
    assert_eq!(
        text_after_setup(
            "var H: Codeunit Helpers; R: Report \"TxRpt Plain Marker\"; begin
                R.SetMarker(9412);
                R.UseRequestPage(false);
                R.RunModal();
                exit(Format(H.Has(9412)) + '|' + Format(Database.IsInWriteTransaction()));
            end;",
            TxModel::None
        ),
        "Yes|No"
    );
}

// record/TestTxModelReportExec.al:TxReportExec_Test12 (:243-257): under AutoRollback the report
// runs and its write stays readable.
#[test]
fn an_update_report_runs_under_autorollback_without_committing() {
    assert_eq!(
        text_after_setup(
            "var H: Codeunit Helpers; R: Report \"TxRpt Update Marker\"; begin
                R.SetMarker(60029122);
                R.Run();
                exit(Format(H.Has(60029122)));
            end;",
            TxModel::AutoRollback
        ),
        "Yes"
    );
}

// record/TestTxModelReportRun.al:TxReportRun_Test12 (:252-269), without the request page: a
// report that raises inside its transaction world does not commit the write made before.
#[test]
fn an_error_in_the_body_propagates() {
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"TxRpt Error Marker\"; begin
            H.Seed(); Commit();
            asserterror R.Run();
            exit(GetLastErrorText() + '|' + Format(H.Has(77)));
        end;"),
        "txrpt report body error|No"
    );
}

// ---------------------------------------------------------------- Execute, Print, RunRequestPage

// record/TestTxModelReportExec.al:TxReportExec_Test08_ExecutePlainReportWithPendingWriteRunsAndIsNotACommit (:174-190)
#[test]
fn execute_runs_a_plain_report_inside_the_callers_transaction() {
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"TxRpt Plain Marker\"; begin
            H.Seed(); Commit();
            H.Insert(60029081);
            R.SetMarker(60029082);
            R.Execute('');
            if not H.Has(60029082) then exit('body did not run');
            asserterror Error('txexec plain probe');
            exit(Format(H.Has(60029081)) + '|' + Format(H.Has(60029082)));
        end;"),
        "No|No"
    );
}

// record/TestTxModelReportExec.al:TxReportExec_Test09 (:193-211): Execute never shows the
// request page, so a report that has one runs without a handler.
#[test]
fn execute_does_not_show_the_request_page() {
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"TxRpt ReqPage Marker\"; begin
            H.Seed(); Commit();
            H.Insert(60029091);
            R.SetMarker(60029092);
            R.Execute('');
            if not H.Has(60029092) then exit('body did not run');
            asserterror Error('txexec reqpage probe');
            exit(Format(H.Has(60029091)) + '|' + Format(H.Has(60029092)));
        end;"),
        "No|No"
    );
}

// record/TestTxModelReportExec.al:TxReportExec_Test06 (:138-150) and Test07 (:153-171)
#[test]
fn execute_of_an_update_report_commits_its_writes_or_is_refused() {
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"TxRpt Update Marker\"; begin
            H.Seed(); Commit();
            R.SetMarker(60029071);
            R.Execute('');
            H.Insert(60029072);
            asserterror Error('txexec update probe');
            exit(Format(H.Has(60029071)) + '|' + Format(H.Has(60029072)));
        end;"),
        "Yes|No"
    );
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"TxRpt Update Marker\"; begin
            H.Seed(); Commit();
            H.Insert(60029061);
            R.SetMarker(60029062);
            asserterror R.Execute('');
            exit(GetLastErrorText() + '|' + Format(H.Has(60029062)) + '|' + Format(H.Has(60029061)));
        end;"),
        "An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.|No|No"
    );
}

// record/TestTxModelReportExec.al:TxReportExec_Test10 (:214-224): the static Execute.
#[test]
fn the_static_execute_is_refused_the_same_way() {
    assert_eq!(
        f("var H: Codeunit Helpers; begin
            H.Seed(); Commit();
            H.Insert(60029101);
            asserterror Report.Execute(Report::\"TxRpt Update Marker\", '');
            exit(GetLastErrorText() + '|' + Format(H.Has(60029101)));
        end;"),
        "An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.|No"
    );
}

// record/TestTxModelReportExec.al:TxReportExec_Test11 (:227-239): Print of an update report.
#[test]
fn print_of_an_update_report_is_refused_with_a_pending_write() {
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"TxRpt Update Marker\"; begin
            H.Seed(); Commit();
            H.Insert(60029111);
            R.SetMarker(60029112);
            asserterror R.Print('');
            exit(GetLastErrorText() + '|' + Format(H.Has(60029112)) + '|' + Format(H.Has(60029111)));
        end;"),
        "An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.|No|No"
    );
}

// record/TestTxModelReportExec.al:TxReportExec_Test02 and Test03 (:77-102)
#[test]
fn runrequestpage_is_refused_with_a_pending_write() {
    assert_eq!(
        f(
            "var H: Codeunit Helpers; R: Report \"TxRpt ReqPage Marker\"; P: Text; begin
            H.Seed(); Commit();
            H.Insert(60029021);
            asserterror P := R.RunRequestPage();
            exit(GetLastErrorText() + '|' + Format(H.Has(60029021)));
        end;"
        ),
        "An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.|No"
    );
    assert_eq!(
        f("var H: Codeunit Helpers; P: Text; begin
            H.Seed(); Commit();
            H.Insert(60029031);
            asserterror P := Report.RunRequestPage(Report::\"TxRpt ReqPage Marker\");
            exit(GetLastErrorText() + '|' + Format(H.Has(60029031)));
        end;"),
        "An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.|No"
    );
}

// ---------------------------------------------------------------- refusals

#[test]
fn a_request_page_that_would_open_with_no_handler_raises_unhandled_ui() {
    // handlers/TestReportRuntimeContracts.al:416-430: the page opens for a run that does not
    // switch it off, and with no handler the run raises the Unhandled UI error.
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"TxRpt ReqPage Marker\"; begin
            H.Seed(); Commit(); asserterror R.Run(); exit(GetLastErrorText() + '|' + GetLastErrorCode());
        end;"),
        "Unhandled UI: RequestPage 60026|MissingUIHandler"
    );
}

#[test]
fn a_report_without_a_request_page_section_opens_its_page_by_default() {
    // handlers/TestReportRuntimeContracts.al:416-430: even without a requestpage section.
    assert_eq!(
        f("var H: Codeunit Helpers; begin
            H.Seed(); Commit(); asserterror Report.Run(Report::\"Default Page Report\");
            exit(GetLastErrorText());
        end;"),
        "Unhandled UI: RequestPage 60876"
    );
}

#[test]
fn currreport_skip_in_on_after_get_record_skips_the_row() {
    // handlers/TestReportRuntimeContracts.al:322-337.
    assert_eq!(
        f("var R: Report \"Skipper Report\"; H: Codeunit Helpers; begin
            H.Seed(); R.UseRequestPage(false); R.Run(); exit('ran');
        end;"),
        "ran"
    );
}

#[test]
fn a_report_that_renders_output_is_refused() {
    let why = refusal(
        "var H: Codeunit Helpers; begin
            H.Seed(); Report.Run(Report::\"Layout Report\", false, false); exit('ran');
        end;",
    );
    assert!(why.contains("Layout Report"), "{why}");
}

// ---------------------------------------------------------------- reportextensions

// reportextensiontrigger/TestReportExtensionTriggers.al:ReportRun_ExtPreAndPostReport_RunAfterTheBaseTriggers (:244-250)
#[test]
fn an_extensions_report_triggers_run_after_the_base_triggers() {
    assert_eq!(
        f("var H: Codeunit Helpers; Log: Codeunit \"RXT Log\"; begin
            H.Seed(); Log.Reset();
            Report.Run(Report::\"RXT Proc Report\", false);
            exit(Log.Read('Report'));
        end;"),
        "base-pre|ext-pre|base-post|ext-post"
    );
}

// :ReportRun_AddedDataItem_RunsItsTriggersOverItsRows (:253-259)
#[test]
fn an_added_data_item_runs_for_every_parent_row() {
    assert_eq!(
        f("var H: Codeunit Helpers; Log: Codeunit \"RXT Log\"; begin
            H.Seed(); Log.Reset();
            Report.Run(Report::\"RXT Proc Report\", false);
            exit(Log.Read('Added'));
        end;"),
        "pre|row-1|row-2|row-3|post|pre|row-1|row-2|row-3|post"
    );
}

// :ReportRun_ModifiedDataItem_ExtTriggersBracketTheBaseAfterGetRecord (:262-268)
#[test]
fn modify_triggers_bracket_the_base_aftergetrecord() {
    assert_eq!(
        f("var H: Codeunit Helpers; Log: Codeunit \"RXT Log\"; begin
            H.Seed(); Log.Reset();
            Report.Run(Report::\"RXT Proc Report\", false);
            exit(Log.Read('Modify'));
        end;"),
        "ext-before-1|base-1|ext-after-1|ext-before-2|base-2|ext-after-2"
    );
}

// :ReportRun_ExtPreReportError_FailsTheRun (:271-280)
#[test]
fn an_error_in_the_extensions_prereport_fails_the_run() {
    assert_eq!(
        f("var H: Codeunit Helpers; Log: Codeunit \"RXT Log\"; begin
            H.Seed(); Log.Reset();
            Log.SetFailExtPreReport(true);
            asserterror Report.Run(Report::\"RXT Proc Report\", false);
            Log.SetFailExtPreReport(false);
            exit(GetLastErrorText() + '|' + Log.Read('Report'));
        end;"),
        "RXT extension OnPreReport failed|base-pre|ext-pre"
    );
}

// Own fixture (RXP Base/RXP Ext); the order is pinned by
// reportextensiontrigger/TestReportExtensionDataItemPlacementContracts.al:214-257.
#[test]
fn added_data_items_run_where_addfirst_addbefore_addafter_and_addlast_put_them() {
    assert_eq!(
        f("var H: Codeunit Helpers; Log: Codeunit \"RXT Log\"; begin
            H.Seed();
            Report.Run(Report::\"RXP Base\", false);
            exit(Log.Read('Place'));
        end;"),
        "Parent|ExtFirst|ExtBeforeA|ChildA|ExtAfterA|ExtBeforeB|ChildB|ExtLast"
    );
}

// extensionevents/TestExtensionDeclaredEvents.al:ReportExtEvent_SubscriberBoundToBaseReport_FiresOnceWithArgument (:105-117)
#[test]
fn an_extension_procedure_is_called_through_the_base_report_and_raises_its_event() {
    assert_eq!(
        f(
            "var R: Report \"Ext Proc Base\"; Sink: Codeunit \"Ext Proc Sink\"; begin
            Sink.Reset();
            R.RaiseReportExt(23);
            exit(Sink.Seen());
        end;"
        ),
        "1:23"
    );
}

#[test]
fn an_extension_with_a_procedure_and_a_global_variable_runs() {
    // Pinned by reportext/W3RpExtProcedures.al: it used to be refused.
    assert_eq!(
        f("var R: Report \"Ext State Base\"; begin R.UseRequestPage(false); exit('x'); end;"),
        "x"
    );
}

#[test]
fn an_empty_extension_changes_nothing() {
    assert_eq!(
        f("var H: Codeunit Helpers; begin
            H.Seed();
            Report.Run(Report::\"RRE ProcessingOnly Report\", false, false);
            exit(Format(H.Count('A-row')));
        end;"),
        "3"
    );
}

// ---- mutation-testing additions (docs/status.md debt) ----

// UseRequestPage(true) overrides a report that declares UseRequestPage = false, and the getter
// answers what the next Run does (handlers/TestReportRuntimeContracts.al:416-440).
#[test]
fn use_request_page_overrides_the_report_property_and_the_getter_reads_it() {
    assert_eq!(
        f("var R: Report \"TxRpt Plain Marker\"; begin R.UseRequestPage(true); exit(Format(R.UseRequestPage())); end;"),
        "Yes"
    );
    assert_eq!(
        f("var R: Report \"TxRpt Plain Marker\"; begin exit(Format(R.UseRequestPage())); end;"),
        "No"
    );
    assert_eq!(
        f(
            "var R: Report \"TxRpt ReqPage Marker\"; begin R.UseRequestPage(true); R.UseRequestPage(false); exit('set'); end;"
        ),
        "set"
    );
}

// Execute takes only the empty parameter string and no RecordRef (record/TestTxModelReportExec.al:138-211).
#[test]
fn execute_with_parameters_or_a_record_ref_is_refused() {
    let why = refusal("var R: Report \"TxRpt Plain Marker\"; begin R.Execute('<xml/>'); exit('ran'); end;");
    assert!(why.contains("only '' is pinned"), "{why}");
    let why =
        refusal("var R: Report \"TxRpt Plain Marker\"; RR: RecordRef; begin R.Execute('', RR); exit('ran'); end;");
    assert!(why.contains("RecordRef"), "{why}");
    let why = refusal("begin Report.Execute(Report::\"TxRpt Plain Marker\", '<xml/>'); exit('ran'); end;");
    assert!(why.contains("only '' is pinned"), "{why}");
}

// "With the Run method, the variable is automatically cleared after the method is run"
// (handlers/TestReportSetTableView.al:17-22): the next Run is a new instance, whose marker is 0.
#[test]
fn a_report_variable_is_cleared_after_run() {
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"TxRpt Plain Marker\"; begin
            H.Seed(); Commit();
            R.SetMarker(60050001);
            R.Run();
            R.Run();
            exit(Format(H.Has(60050001)) + '|' + Format(H.Has(0)));
        end;"),
        "Yes|Yes"
    );
}

// CurrReport.Quit in OnPreReport ends the run (handlers/TestReportRuntimeContracts.al:355-358).
#[test]
fn quit_in_onprereport_ends_the_run() {
    assert_eq!(
        f("var H: Codeunit Helpers; R: Report \"MX Quit In Pre\"; begin H.Seed(); R.Run(); exit('ran'); end;"),
        "ran"
    );
}

// A table view that is more than filters (marked only, re-sorted, descending, temporary) is taken as well: the marks, key and
// direction carry over and a temporary record counts by its filters only (corpus errors/lt-ms-refusals-136/P136Report.al:
// P136_Report_ViewTemporaryMarkedAndSorted).
#[test]
fn set_table_view_takes_marks_key_and_direction() {
    for setup in [
        "",
        "V.MarkedOnly(true);",
        "V.SetCurrentKey(Name);",
        "V.Ascending(false);",
    ] {
        let r = run_model(
            &format!(
                "procedure F(): Text
                var R: Report \"STV Counting Report\"; V: Record \"RRE Row\";
                begin V.SetRange(\"Entry No.\", 1); {setup} R.SetTableView(V); exit('set'); end;"
            ),
            TxModel::AutoCommit,
        );
        assert!(matches!(r, Ok(Value::Text(ref t)) if t == "set"), "{setup}: {r:?}");
    }
    let r = run_model(
        "procedure F(): Text
        var R: Report \"STV Counting Report\"; V: Record \"RRE Row\" temporary;
        begin R.SetTableView(V); exit('set'); end;",
        TxModel::AutoCommit,
    );
    assert!(matches!(r, Ok(Value::Text(ref t)) if t == "set"), "temporary: {r:?}");
}

// A report run that enters a transaction world inside a TryFunction is not pinned.
#[test]
fn a_world_entering_run_inside_a_try_function_is_refused() {
    let r = run_model(
        "[TryFunction] procedure T() var R: Report \"TxRpt Update Marker\"; begin R.Run(); end;
         procedure F(): Text var H: Codeunit Helpers; begin H.Seed(); Commit(); if T() then exit('ok'); exit('failed'); end;",
        TxModel::AutoCommit,
    );
    match r {
        Err(Signal::Unsupported(m)) => assert!(m.contains("inside a TryFunction"), "{m}"),
        other => panic!("{other:?}"),
    }
}

// codeunit/TestWriteTxTestBoundary.al:WriteTxBoundary_Test11 (:259-279): the transaction a report
// begins under None ends with the run, and a direct write from the test is refused again.
#[test]
fn after_a_report_under_none_a_direct_write_is_refused_again() {
    assert_eq!(
        text_after_setup(
            "var H: Codeunit Helpers; R: Report \"TxRpt Plain Marker\"; begin
                R.SetMarker(9413);
                R.UseRequestPage(false);
                R.RunModal();
                asserterror H.Insert(9414);
                exit(GetLastErrorText());
            end;",
            TxModel::None
        ),
        "A transaction must be started before changes can be made to the database."
    );
}

// A filter on a field that a DataItemLink joins applies next to the link (report/lt-f3-reports/F3ReportTests.al:68); the report runs
// (see report_link_filter.rs for the rows).
#[test]
fn a_filter_on_a_field_the_data_item_link_joins_runs() {
    assert_eq!(
        f(
            "var H: Codeunit Helpers; begin H.Seed(); Report.Run(Report::\"MX Link Filter\", false, false); exit('ran'); end;"
        ),
        "ran"
    );
}

// ---------------------------------------------------------------- CurrReport.Language and FormatRegion

// handlers/TestReportCurrReportLanguage.al:TestReport_SettingCurrReportLanguage_RunsAndIsReadableBack (:58-84):
// a language set in a trigger reads back at once (1031, not the session's 1033); FormatRegion set in
// OnPreReport reads back empty for a SaveAs-Xml run. In a report started by Run it is kept and reads back
// (misc/lt-probes-needed-3/PnC06ReportLanguage.al:Language_SetAndRead), except `en-US`, the session's own region, which reads
// back empty (misc/lt-probes-needed-8/PnH4CurrReport.al:244).
#[test]
fn curr_report_language_reads_back_and_format_region_is_kept_by_run() {
    assert_eq!(
        f(
            "var Probe: Report \"CRL Report\"; Log: Codeunit \"CRL Log\"; H: Codeunit Helpers; begin
            H.Seed();
            Probe.SetLang(1031);
            Probe.Run();
            exit(Format(Log.GetReads()) + '|' + Format(Log.GetLang()) + '|[' + Log.GetRegion() + ']');
        end;"
        ),
        // `en-US` set in OnPreReport, before the language changes, reads back empty
        // (misc/lt-probes-needed-8/PnH4CurrReport.al:244 `region=[]`).
        "1|1031|[]"
    );
}

// After `CurrReport.Language := 1031` the region `de-DE` is kept and reads back (misc/lt-probes-needed-3/PnC06ReportLanguage.al:
// 58-65 `region1:de-DE`). Any other region set after a language change is not pinned (`en-US` reads back empty only while the
// report keeps the session's language, misc/lt-probes-needed-8/PnH4CurrReport.al:244), so it is refused.
#[test]
fn format_region_after_a_language_change_is_pinned_for_de_de_only() {
    let body = |lang: i32, region: &str| {
        format!(
            "var Probe: Report \"CRL Lang Then Region\"; Log: Codeunit \"CRL Log\"; H: Codeunit Helpers; begin
            H.Seed();
            Probe.Set({lang}, '{region}');
            Probe.Run();
            exit(Format(Log.GetLang()) + '|[' + Log.GetRegion() + ']');
        end;"
        )
    };
    assert_eq!(f(&body(1031, "de-DE")), "1031|[de-DE]");
    for (lang, region) in [
        (1031, "en-US"),
        (1033, "en-US"),
        (1036, "en-US"),
        (1031, "fr-FR"),
        (1036, "de-DE"),
    ] {
        let r = refusal(&body(lang, region));
        assert!(
            r.contains("CurrReport.FormatRegion") && r.contains(region),
            "{lang} {region}: {r}"
        );
    }
}

// The same file :87-103 says BC rejects language 0 with an error that names the language; the text is
// `Invalid language ID supplied: 0.` / Argument (record/ProbeClearAllSetKeyHyperlink.al:341-353, BC 27.5), and a
// language nobody set reads as the session's 1033.
#[test]
fn curr_report_language_zero_raises_the_invalid_language_error() {
    assert_eq!(
        f("var Probe: Report \"CRL Report\"; H: Codeunit Helpers; begin
        H.Seed();
        Probe.SetLang(0);
        asserterror Probe.Run();
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;"),
        "Invalid language ID supplied: 0.|Argument"
    );
}

/// SetTableView with a record of a table that only a reportextension's data item uses (Microsoft's Get Source Documents and
/// its Serv. Get Source Documents extension, SCM Warehouse Management II): no corpus test pins it, so it is refused, not
/// answered with the error for a report without such a data item.
#[test]
fn set_table_view_on_a_data_item_a_reportextension_adds_is_refused() {
    let src = r#"
table 51960 "SV Head" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 51961 "SV Other" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
report 51962 "SV Base" { ProcessingOnly = true; dataset { dataitem(H; "SV Head") { } } }
reportextension 51963 "SV Ext" extends "SV Base" { dataset { addlast(H) { dataitem(O; "SV Other") { } } } }
codeunit 51964 T {
    procedure F(): Text
    var R: Report "SV Base"; O: Record "SV Other";
    begin
        R.SetTableView(O);
        exit('ran');
    end;
}
"#;
    match al_testkit::run_in(src, "T", "F") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("reportextension"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
