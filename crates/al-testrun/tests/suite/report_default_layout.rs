//! `Report.DefaultLayout(Number)`: the kind the report names with `DefaultLayout` or with the layout `DefaultRenderingLayout`
//! picks, else RDLC; a processing-only report that names none is `None`
//! (corpus metadata/lt-report-metadata-columns/ReportMetadata.al:272 P175_DefaultLayout_PerReport,
//! report/lt-y-report-props/YReportPropsTests.al ExcelLayoutMultipleDataSheets_*, YReportExt.al:152: a Word or RDLC layout
//! property alone leaves RDLC). Anything else is refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
report 50570 "DL Proc" { ProcessingOnly = true; dataset { dataitem(Item; Integer) { DataItemTableView = where(Number = const(1)); } } }
report 50571 "DL Plain" { UsageCategory = None; dataset { dataitem(Item; Integer) { DataItemTableView = where(Number = const(1)); column(Num; Number) { } } } }
report 50572 "DL Word" { UsageCategory = None; DefaultLayout = Word; WordLayout = 'a.docx'; dataset { dataitem(Item; Integer) { DataItemTableView = where(Number = const(1)); column(Num; Number) { } } } }
report 50573 "DL Excel" { UsageCategory = None; DefaultRenderingLayout = XlLayout; dataset { dataitem(Item; Integer) { DataItemTableView = where(Number = const(1)); column(Num; Number) { } } }
    rendering { layout(XlLayout) { Type = Excel; LayoutFile = 'a.xlsx'; Caption = 'x'; } } }
report 50574 "DL ProcWord" { ProcessingOnly = true; DefaultLayout = Word; dataset { dataitem(Item; Integer) { DataItemTableView = where(Number = const(1)); } } }
codeunit 50570 "T"
{
    Subtype = Test;
    local procedure Kind(Id: Integer): Text
    begin
        exit(Format(Report.DefaultLayout(Id)));
    end;
    [Test] procedure ProcessingOnlyIsNone() begin if Kind(50570) <> 'None' then Error('%1', Kind(50570)); end;
    [Test] procedure PlainIsRdlc() begin if Kind(50571) <> 'RDLC' then Error('%1', Kind(50571)); end;
    [Test] procedure DeclaredWord() begin if Kind(50572) <> 'Word' then Error('%1', Kind(50572)); end;
    [Test] procedure RenderingExcel() begin if Kind(50573) <> 'Excel' then Error('%1', Kind(50573)); end;
    [Test] procedure ProcessingOnlyWithAKindIsRefused() begin if Kind(50574) = '' then; end;
    [Test] procedure UnknownIdIsRefused() begin if Kind(99999) = '' then; end;
    [TryFunction] local procedure TryKind(Id: Integer)
    begin
        if Kind(Id) = '' then;
    end;
    // From a try function: the missing-object error (report/lt-report-members-testrequestpage/P91Static.al:113-115).
    [Test] procedure UnknownIdFromATryFunctionRaises()
    begin
        if TryKind(99999) then Error('no error');
        if GetLastErrorCode() <> 'AL:MetadataNotFound' then Error('%1', GetLastErrorCode());
    end;
}
"#;

fn outcome(name: &str) -> Outcome {
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
    rt.first_outcome(&CaseId {
        codeunit: 50570,
        name: name.into(),
    })
}

#[test]
fn the_default_layout_of_a_report_by_number() {
    for t in [
        "ProcessingOnlyIsNone",
        "PlainIsRdlc",
        "DeclaredWord",
        "RenderingExcel",
        "UnknownIdFromATryFunctionRaises",
    ] {
        assert_eq!(outcome(t), Outcome::Pass, "{t}");
    }
}

#[test]
fn unpinned_cases_are_refused() {
    for t in ["ProcessingOnlyWithAKindIsRefused", "UnknownIdIsRefused"] {
        assert!(matches!(outcome(t), Outcome::NotRun(_)), "{t}: {:?}", outcome(t));
    }
}
