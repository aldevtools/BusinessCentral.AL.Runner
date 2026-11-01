//! A report extension's request page declares its own local procedure named as a procedure of the base report, and calls it from
//! the request page's OnOpenPage and from the OnAfterValidate of a modified base field bound to a variable (al-runner issue 6651).
//! Inside the extension's request page its own procedure is the call: next to a local or a public base procedure, and next to an
//! extension-level procedure of the same name, which the extension's other triggers call (corpus writer Y64, branch
//! al-runtime-rs-staging-y64 85d86e7b, reportext/lt-y64-local-proc-names/Y64LocalProcNames.al:438-477). In a report itself, a
//! request page procedure next to a report procedure of the same name is not pinned: refused.

use al_test_cli::report::Outcome;

fn outcome(base_access: &str, ext_level: bool) -> Outcome {
    let ext_proc = if ext_level {
        "local procedure MyProc() begin ExtHits += 1000; end;"
    } else {
        ""
    };
    let src = format!(
        r#"
table 62800 "RQP Row" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ Clustered = true; }} }} }}
report 62801 "RQP Report"
{{
    ProcessingOnly = true; UseRequestPage = true;
    dataset {{ dataitem(Row; "RQP Row") {{ }} }}
    requestpage
    {{
        layout {{ area(Content) {{ field(MyField; MyOption) {{ trigger OnValidate() begin MyProc(); end; }} }} }}
        trigger OnOpenPage() begin MyProc(); end;
    }}
    var MyOption: Integer; BaseHits: Integer;
    {base_access}procedure MyProc() begin BaseHits += 100; end;
}}
reportextension 62802 "RQP Report Ext" extends "RQP Report"
{{
    requestpage
    {{
        layout
        {{
            addlast(Content) {{ field(HitsField; Hits) {{ }} }}
            modify(MyField) {{ trigger OnAfterValidate() begin MyProc(); end; }}
        }}
        trigger OnOpenPage() begin MyProc(); end;
        local procedure MyProc() begin Hits += 1; end;
    }}
    {ext_proc}
    var Hits: Integer; ExtHits: Integer;
}}
codeunit 62890 T
{{
    Subtype = Test;
    [Test] [HandlerFunctions('H')] procedure Run()
    var R: Report "RQP Report";
    begin
        Commit();
        R.UseRequestPage(true); R.Run();
    end;
    [RequestPageHandler] procedure H(var P: TestRequestPage "RQP Report")
    begin
        if P.HitsField.Value <> '1' then Error('after open: %1', P.HitsField.Value);
        P.MyField.SetValue(5);
        if P.HitsField.Value <> '2' then Error('after validate: %1', P.HitsField.Value);
        P.OK().Invoke();
    end;
}}"#
    );
    al_testkit::outcomes_in(&src, "T")[0].1.clone()
}

#[test]
fn a_request_page_procedure_of_a_report_extension_is_its_own() {
    assert_eq!(outcome("local ", false), Outcome::Pass);
    assert_eq!(outcome("", false), Outcome::Pass);
    // An extension-level procedure makes the extension change the run, and with triggers on its request page that is still
    // refused (`reportext_request_page_run.rs`).
    assert!(
        matches!(outcome("local ", true), Outcome::NotRun(why) if why.contains("a trigger on the extension's request page"))
    );
}

#[test]
fn a_request_page_procedure_next_to_a_report_procedure_of_the_same_name_is_refused() {
    let src = r#"
table 62810 "RQP2 Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
report 62811 "RQP2 Report"
{
    ProcessingOnly = true; UseRequestPage = true;
    dataset { dataitem(Row; "RQP2 Row") { } }
    requestpage
    {
        trigger OnOpenPage() begin MyProc(); end;
        local procedure MyProc() begin end;
    }
    local procedure MyProc() begin end;
}
codeunit 62891 T
{
    Subtype = Test;
    [Test] [HandlerFunctions('H')] procedure Run() var R: Report "RQP2 Report"; begin Commit(); R.Run(); end;
    [RequestPageHandler] procedure H(var P: TestRequestPage "RQP2 Report") begin P.OK().Invoke(); end;
}"#;
    let o = al_testkit::outcomes_in(src, "T")[0].1.clone();
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("which one a call in the request page runs is not pinned")),
        "{o:?}"
    );
}
