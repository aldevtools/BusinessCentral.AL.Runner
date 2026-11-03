//! OK or Cancel on a request page with a source table, after the handler changed a field of the row: the row is written
//! (corpus report/lt-y80-reportext-request-page/Y80ReportExtRequestPage.al Y80_RX_SetTxt_OK, _SetTxt_Cancel). What the table's
//! own OnModify does at that write is observed by no corpus case and needed by no Microsoft test, so a table with write code
//! is refused (needs probe), a table without it passes.

use al_test_cli::report::Outcome;

fn outcome(table_code: bool) -> Outcome {
    let code = if table_code {
        "trigger OnModify() begin Seen += 1; end;"
    } else {
        ""
    };
    let src = format!(
        r#"
table 62800 "RC Row"
{{
    fields {{ field(1; Id; Integer) {{ }} field(2; Txt; Text[30]) {{ }} field(3; Seen; Integer) {{ }} }}
    keys {{ key(PK; Id) {{ Clustered = true; }} }}
    {code}
}}
report 62801 "RC Report"
{{
    ProcessingOnly = true;
    dataset {{ dataitem(T; "RC Row") {{ }} }}
    requestpage {{ SourceTable = "RC Row"; layout {{ area(Content) {{ field(Txt; Rec.Txt) {{ }} }} }} }}
}}
codeunit 62802 T
{{
    Subtype = Test;
    [Test] [HandlerFunctions('H')] procedure Run()
    var A: Record "RC Row";
    begin
        A.DeleteAll(); A.Id := 1; A.Insert(); Commit();
        Report.Run(Report::"RC Report");
        A.Get(1); if A.Txt <> 'typed' then Error('stored %1', A.Txt);
    end;
    [RequestPageHandler] procedure H(var P: TestRequestPage "RC Report")
    begin P.Txt.SetValue('typed'); P.OK().Invoke(); end;
}}"#
    );
    al_testkit::outcomes_in(&src, "T")[0].1.clone()
}

#[test]
fn a_table_without_write_code_stores_the_changed_row_at_ok() {
    assert_eq!(outcome(false), Outcome::Pass);
}

#[test]
fn a_table_with_write_code_is_refused() {
    let o = outcome(true);
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("runs code when it is written")),
        "{o:?}"
    );
}
