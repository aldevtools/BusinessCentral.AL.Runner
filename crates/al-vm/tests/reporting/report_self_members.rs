//! A report's own members called without a receiver inside the report (`UseRequestPage(false); RunModal();` in a
//! procedure of the report). The receiver form `Rep.UseRequestPage(false)` is pinned by
//! codeunit/TestWriteTxTestBoundary.al:272; AL resolves a bare member name inside an object to the object itself, so
//! the call without the receiver is the same call. BC compiles it (never reject code BC accepts, ADR-009).

use al_testkit::{boolean, error_message, run_in};

const SRC: &str = r#"
table 50960 "RSM Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

report 50961 "RSM Seek"
{
    ProcessingOnly = true;
    UsageCategory = None;
    dataset
    {
        dataitem(Row; "RSM Row")
        {
        }
    }
    requestpage
    {
        layout { area(Content) { field(FoundField; Found) { ApplicationArea = All; } } }
    }
    trigger OnPreReport()
    begin
        Found := true;
    end;

    var
        Found: Boolean;

    local procedure GetResult(): Boolean
    begin
        UseRequestPage(false);
        RunModal();
        exit(Found);
    end;

    procedure Seek(): Boolean
    begin
        exit(GetResult());
    end;
}

codeunit 50962 "RSM Caller"
{
    procedure Go(): Boolean
    var
        Rep: Report "RSM Seek";
    begin
        exit(Rep.Seek());
    end;
}
"#;

#[test]
fn use_request_page_without_a_receiver_inside_the_report_runs() {
    assert!(boolean(run_in(SRC, "RSM Caller", "Go")));
}

/// Without `UseRequestPage(false)` the request page would open, and with no handler the call fails: the bare call set
/// the flag on the instance that runs.
#[test]
fn without_the_bare_use_request_page_the_request_page_opens() {
    let src = SRC.replace("        UseRequestPage(false);\n", "");
    let e = error_message(run_in(&src, "RSM Caller", "Go"));
    assert!(e.contains("Unhandled UI: RequestPage 50961"), "{e}");
}

const IN_TRIGGER: &str = r#"
table 50963 "RSM Row2"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}

report 50964 "RSM Self Trigger"
{
    ProcessingOnly = true;
    UsageCategory = None;
    dataset { dataitem(Row; "RSM Row2") { } }
    trigger OnPreReport()
    begin
        RunModal();
    end;
}

codeunit 50965 "RSM Caller2"
{
    procedure Go()
    var
        Rep: Report "RSM Self Trigger";
    begin
        Rep.UseRequestPage(false);
        Rep.RunModal();
    end;
}
"#;

/// The bare `RunModal()` in a trigger would run the report inside its own run: not pinned, refused.
#[test]
fn a_bare_run_modal_in_a_trigger_of_the_report_is_refused() {
    match run_in(IN_TRIGGER, "RSM Caller2", "Go") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("not pinned"), "{why}"),
        other => panic!("{other:?}"),
    }
}
