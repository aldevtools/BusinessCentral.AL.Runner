//! An error in a report column's source expression is wrapped: "The SourceExpr for report control '<column>' is not valid. The
//! following error occurred: <error>", code ReportSourceExpression (report/lt-qw-ternary-columns/QwTernaryColumns.al:506-516,
//! a ternary column whose Integer true branch makes a Decimal 12.5 false branch overflow). Only that conversion error is pinned;
//! any other error in a column's source expression is refused.

use al_testkit::text;

fn src(column: &str) -> String {
    format!(
        "table 50110 T {{ fields {{ field(1; Id; Integer) {{ }} field(2; Flag; Boolean) {{ }} field(3; Amount; Decimal) {{ }} field(4; Data; Blob) {{ }} }} keys {{ key(PK; Id) {{ }} }} }} \
    report 50111 R {{ ProcessingOnly = false; DefaultLayout = RDLC; UseRequestPage = false; dataset {{ dataitem(Row; T) {{ \
      column(Id; Row.Id) {{ }} column(IntOrDec; {column}) {{ }} }} }} \
      local procedure Fail(): Integer begin Error('boom'); end; }} \
    codeunit 50110 C {{ procedure F(): Text var Row: Record T; Buf: Record T; OutStr: OutStream; \
      begin Row.Id := 1; Row.Amount := 12.5; Row.Insert(); \
      Buf.Data.CreateOutStream(OutStr); if TrySave(OutStr) then exit('ok'); exit(GetLastErrorText() + '|' + GetLastErrorCode()); end; \
      [TryFunction] local procedure TrySave(var OutStr: OutStream) begin Report.SaveAs(Report::R, '', ReportFormat::Xml, OutStr); end; }}"
    )
}

#[test]
fn a_conversion_error_in_a_column_is_wrapped_with_the_column_name() {
    assert_eq!(
        text(al_testkit::run_in(&src("Row.Flag ? 7 : Row.Amount"), "C", "F")),
        "The SourceExpr for report control 'IntOrDec' is not valid. The following error occurred: Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 12.5 to System.Int32.|ReportSourceExpression"
    );
}

#[test]
fn another_error_in_a_column_is_refused() {
    match al_testkit::run_in(&src("Fail()"), "C", "F") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("IntOrDec"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
