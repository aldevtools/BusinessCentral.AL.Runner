//! `Report.SaveAsHtml(Number, FileName, Record)`. Pinned only for a report with an RDLC layout, the answer used: false,
//! `ReportUnsupportedFormat`, no file (al-language-onprem/reports/layouts-saveas/AMRdlcSaveAs.al:454-469 `static html
//! filtered`). Every other shape is refused before the report runs, because what the record filters is not pinned.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const FIXTURE: &str = r#"
table 50980 "SHR Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

report 50981 "SHR Rdlc"
{
    DefaultRenderingLayout = Rdlc;
    dataset { dataitem(T; "SHR Row") { column(TId; Id) { } } }
    rendering { layout(Rdlc) { Type = RDLC; LayoutFile = './a.rdl'; } }
}

report 50982 "SHR No Layout"
{
    dataset { dataitem(T; "SHR Row") { column(TId; Id) { } } }
}

report 50983 "SHR Processing Only"
{
    ProcessingOnly = true;
    dataset { dataitem(T; "SHR Row") { } }
    trigger OnPreReport() begin Error('the report ran'); end;
}

codeunit 50984 "SHR Tests"
{
    procedure RdlcUsed(): Text
    var R: Record "SHR Row"; Ok: Boolean;
    begin
        R.SetRange(Id, 1);
        Ok := Report.SaveAsHtml(Report::"SHR Rdlc", 'out/a.html', R);
        exit(Format(Ok) + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure NoLayoutStatement()
    var R: Record "SHR Row";
    begin
        Report.SaveAsHtml(Report::"SHR No Layout", 'out/a.html', R);
    end;

    procedure ProcessingOnlyUsed(): Text
    var R: Record "SHR Row"; Ok: Boolean;
    begin
        Ok := Report.SaveAsHtml(Report::"SHR Processing Only", 'out/a.html', R);
        exit(Format(Ok));
    end;
}
"#;

fn run(routine: &str) -> Result<Value, Signal> {
    al_testkit::run_in(FIXTURE, "SHR Tests", routine)
}

fn assert_refused(r: Result<Value, Signal>) {
    match r {
        Err(Signal::Unsupported(m)) => assert!(m.contains("record argument"), "{m}"),
        other => panic!("expected the record refusal, got {other:?}"),
    }
}

#[test]
fn an_rdlc_report_with_a_record_answers_the_pinned_error() {
    assert_eq!(
        text(run("RdlcUsed")),
        "No|The Html output format is not supported by the current report layout format.|ReportUnsupportedFormat"
    );
}

#[test]
fn a_report_without_a_layout_is_refused_before_it_runs() {
    assert_refused(run("NoLayoutStatement"));
}

#[test]
fn a_processing_only_report_is_refused_before_it_runs() {
    assert_refused(run("ProcessingOnlyUsed"));
}
