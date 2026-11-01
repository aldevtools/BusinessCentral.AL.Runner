//! A report extension's request page declares its own local procedure named as a local procedure of the base report, and calls
//! it from the request page's OnOpenPage and from the OnAfterValidate of a modified base field (al-runner issue 6651). The
//! base's local procedure is out of the extension's reach (AL0161), so the call is the extension's own. A base procedure the
//! extension could reach under the same name leaves the binding unpinned: refused. The OnAfterValidate of a control bound to a
//! variable is not pinned and stays refused, so the run here validates no field.

use al_test_cli::report::Outcome;

fn outcome(base_local: bool) -> Outcome {
    let access = if base_local { "local " } else { "" };
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
    {access}procedure MyProc() begin BaseHits += 100; end;
}}
reportextension 62802 "RQP Report Ext" extends "RQP Report"
{{
    requestpage
    {{
        layout
        {{
            addlast(Content) {{ field(HitsField; Hits) {{ }} }}
            modify(MyField) {{ Caption = 'Mine'; }}
        }}
        trigger OnOpenPage() begin MyProc(); end;
        local procedure MyProc() begin Hits += 1; end;
    }}
    var Hits: Integer;
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
        P.OK().Invoke();
    end;
}}"#
    );
    al_testkit::outcomes_in(&src, "T")[0].1.clone()
}

#[test]
fn a_request_page_procedure_of_a_report_extension_is_its_own_next_to_a_local_base_procedure() {
    assert_eq!(outcome(true), Outcome::Pass);
}

#[test]
fn a_request_page_procedure_next_to_a_reachable_base_procedure_is_refused() {
    let o = outcome(false);
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("which one a call in the request page runs is not pinned")),
        "{o:?}"
    );
}
