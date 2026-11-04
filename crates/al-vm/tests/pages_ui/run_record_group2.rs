//! The filters a record handed to Page.Run carries in filter group 2 stay when the page's SourceTableView is applied
//! (group 2 too). Base Application's `Bank Acc. Reconciliation.OpenWorksheet` sets the statement in group 2 and runs
//! "Payment Reconciliation Journal", whose SourceTableView names the statement type; the page showed the first line of
//! the whole table (Tests-Bank Bank Pmt. Appl. Algorithm TestCustSingleAmountMatch). How the record and the view combine
//! is pinned by types/text-code-char/P145PageRunView.al (P145_A_RecFilterInGroup2).

use al_testkit::text;

const SRC: &str = r#"
table 69970 "G2 Row" { fields { field(1; Kind; Code[10]) { } field(2; K; Code[10]) { } } keys { key(PK; Kind, K) { } } }
page 69971 "G2 List" { PageType = List; SourceTable = "G2 Row"; SourceTableView = where(Kind = const('A'));
  layout { area(content) { repeater(R) { field(KCtl; Rec.K) { } } } } }
codeunit 69972 C
{
    procedure F(): Text
    var R: Record "G2 Row"; TP: TestPage "G2 List";
    begin
        R.Kind := 'A'; R.K := '1'; R.Insert();
        R.Kind := 'A'; R.K := '2'; R.Insert();
        R.Kind := 'B'; R.K := '3'; R.Insert();
        R.Reset();
        R.FilterGroup := 2;
        R.SetRange(K, '2');
        R.FilterGroup := 0;
        TP.Trap();
        Page.Run(Page::"G2 List", R);
        TP.First();
        exit(TP.KCtl.Value());
    end;
}
"#;

#[test]
fn a_filter_in_group_2_of_the_record_survives_the_source_table_view() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "2");
}
