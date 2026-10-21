//! ADR-040 in a test run: `TestRequestPage.SaveAsPdf` and `SaveAsExcel` in a request page handler run a listed report
//! there and then, write nothing, and close the page; a look at the file afterwards is refused. A report that is not
//! listed keeps the refusal. (The save forms on a report variable are tested in `al-vm`, tests/reporting.)

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};

const BASE_APPLICATION: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";

fn base() -> &'static str {
    r#"
table 50800 "RRD Log" { fields { field(1; Id; Integer) { AutoIncrement = true; } field(2; What; Text[30]) { } } keys { key(PK; Id) { } } }
table 50801 "RRD Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }

report 1002 "Post Inventory Cost to G/L"
{
    UsageCategory = None;
    dataset
    {
        dataitem(T; "RRD Row")
        {
            trigger OnAfterGetRecord() var L: Record "RRD Log"; begin L.What := 'row'; L.Insert(); end;
        }
    }
    requestpage { layout { area(Content) { group(Options) { field(Opt; Opt) { ApplicationArea = All; } } } } }
    var Opt: Text;
}

report 50802 "RRD Not Listed"
{
    UsageCategory = None;
    dataset { dataitem(T; "RRD Row") { } }
    requestpage { layout { area(Content) { group(Options) { field(Opt; Opt) { ApplicationArea = All; } } } } }
    var Opt: Text;
}
"#
}

fn tests() -> &'static str {
    r#"
codeunit 60800 "RRD Tests"
{
    Subtype = Test;

    local procedure Seed() var R: Record "RRD Row"; L: Record "RRD Log"; begin L.DeleteAll(); R.DeleteAll(); R.Id := 1; R.Insert(); R.Id := 2; R.Insert(); Commit(); end;

    [Test] [HandlerFunctions('SavePdf')]
    procedure PdfRunsTheReportAndLeavesNoFile() var L: Record "RRD Log"; begin
        Seed();
        Report.Run(Report::"Post Inventory Cost to G/L");
        if L.Count() <> 2 then Error('the report ran %1 rows', L.Count());
    end;

    [Test] [HandlerFunctions('SaveExcel')]
    procedure ExcelRunsTheReport() var L: Record "RRD Log"; begin
        Seed();
        Report.Run(Report::"Post Inventory Cost to G/L");
        if L.Count() <> 2 then Error('the report ran %1 rows', L.Count());
    end;

    [Test] [HandlerFunctions('SavePdf')]
    procedure LookingAtTheFileIsRefused() begin
        Seed();
        Report.Run(Report::"Post Inventory Cost to G/L");
        if File.Exists('out/a.pdf') then Error('it exists');
    end;

    [Test] [HandlerFunctions('SavePdfOfOther')]
    procedure AReportThatIsNotListedIsRefused() begin
        Seed();
        Report.Run(Report::"RRD Not Listed");
    end;

    [RequestPageHandler] procedure SavePdf(var RP: TestRequestPage "Post Inventory Cost to G/L") begin RP.SaveAsPdf('out/a.pdf'); end;
    [RequestPageHandler] procedure SaveExcel(var RP: TestRequestPage "Post Inventory Cost to G/L") begin RP.SaveAsExcel('out/a.xlsx'); end;
    [RequestPageHandler] procedure SavePdfOfOther(var RP: TestRequestPage "RRD Not Listed") begin RP.SaveAsPdf('out/b.pdf'); end;
}
"#
}

fn run() -> CorpusRuntime {
    let app = |id: &str, name: &str, deps: &[&str], src: &str| AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{name}.al"), src.into())],
    };
    let manifest = |p: &str| Manifest {
        publisher: p.into(),
        version: "1.0.0.0".into(),
        resources: vec![],
        include_source: false,
        target: String::new(),
        symbols: vec![],
        layout_files: Vec::new(),
        id_ranges: Vec::new(),
    };
    CorpusRuntime::try_run_suites_with(
        vec![
            app("main", "Main", &[BASE_APPLICATION], tests()),
            app(BASE_APPLICATION, "Base Application", &[], base()),
        ],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        &[manifest("Me"), manifest("Microsoft")],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
    .expect("the run starts")
}

fn outcome(rt: &CorpusRuntime, test: &str) -> Outcome {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .map(|r| r.outcome.clone())
        .unwrap_or_else(|| panic!("{test} did not run"))
}

#[test]
fn a_request_page_handler_that_saves_a_listed_report_as_pdf_or_excel_runs_it_and_writes_nothing() {
    let rt = run();
    assert_eq!(
        outcome(&rt, "PdfRunsTheReportAndLeavesNoFile"),
        Outcome::Pass,
        "{:?}",
        rt.results()
    );
    assert_eq!(outcome(&rt, "ExcelRunsTheReport"), Outcome::Pass);
}

#[test]
fn looking_at_the_file_after_the_save_is_refused_and_the_other_report_stays_refused() {
    let rt = run();
    match outcome(&rt, "LookingAtTheFileIsRefused") {
        Outcome::NotRun(why) => assert!(
            why.contains("rendered output of report Post Inventory Cost to G/L is not produced"),
            "{why}"
        ),
        other => panic!("{other:?}"),
    }
    match outcome(&rt, "AReportThatIsNotListedIsRefused") {
        Outcome::NotRun(why) => assert!(why.contains("layout renderer"), "{why}"),
        other => panic!("{other:?}"),
    }
}
