//! SetView of a view GetView wrote when a field's caption differs from its name (pinned by
//! record/views/SetViewByCaption.al; Tests-ERM ERM Purchase Payables PurchaseInvoiceAmountExclVATWithInheritFromParentAllocationAccount
//! runs Purchase Alloc. Acc. Mgt. ReplaceInheritFromParent: `SetView(GetView())` on a table whose "Line No." has Caption 'Primary Key').

use al_testkit::text;

const SRC: &str = r#"
table 69980 "SVC Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Line No."; Integer) { Caption = 'Primary Key'; }
        field(3; Name; Code[10]) { }
    }
    keys { key(PK; Id, "Line No.") { } }
}
codeunit 69981 C
{
    procedure F(): Text
    var A: Record "SVC Row"; B: Record "SVC Row";
    begin
        A.SetRange("Line No.", 2, 4);
        B.SetView(A.GetView());
        exit(B.GetView());
    end;
}
"#;

#[test]
fn a_view_that_names_a_field_by_its_caption_sets_back() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "F")),
        "VERSION(1) SORTING(Id,Primary Key) WHERE(Primary Key=FILTER(2..4))"
    );
}
