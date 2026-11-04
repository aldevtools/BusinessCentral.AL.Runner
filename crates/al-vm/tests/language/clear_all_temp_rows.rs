//! ClearAll leaves the rows of a global temporary record alone, as Clear does
//! (types/ProbeTypeFormsAndFilters.al:Clear_TemporaryRecordWithRows_ThenCount). Base Application's Purch.-Post
//! `ClearAllVariables` runs ClearAll and then relies on TempWhseRcptHeader, which SetWhseRcptHeader filled before
//! (record/PriorityAssumptions.al:247-281 pins it; Tests-SCM OverReceiptQtyPreservedAfterPartialWhseReceiptPost).

use al_testkit::text;

const SRC: &str = r#"
table 69990 "CAT Row" { fields { field(1; Id; Integer) { } field(2; Txt; Text[10]) { } } keys { key(PK; Id) { } } }
codeunit 69991 C
{
    var Tmp: Record "CAT Row" temporary;

    procedure F(): Text
    begin
        Tmp.Id := 1; Tmp.Txt := 'a'; Tmp.Insert();
        Tmp.SetRange(Id, 5);
        ClearAll();
        exit(Format(Tmp.Count()) + ' ' + Format(Tmp.GetFilters() = ''));
    end;
}
"#;

#[test]
fn clear_all_keeps_the_rows_of_a_global_temporary_record_and_drops_its_filters() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "1 Yes");
}
