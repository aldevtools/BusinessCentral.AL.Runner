//! `TestPage.Filter.GetFilter` of a Boolean field that a page's SourceTableView or an action's RunPageView filters with
//! `const(true)` reads `1` (pages/lt-rz-outside-page-moves/RzRunPageLinkView.al:62 RZ_Link_WhereView_I1, :112
//! RZ_LinkAndRunPageViewWhere_DescendingView_I1). `const(false)` there is not pinned and refused.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50130 "Bool Entry"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Positive; Boolean) { } }
    keys { key(PK; "Entry No.") { } }
}
page 50131 "Bool Positive"
{
    PageType = List; SourceTable = "Bool Entry"; SourceTableView = where(Positive = const(true));
    layout { area(Content) { repeater(R) { field("Entry No."; Rec."Entry No.") { } } } }
}
page 50132 "Bool Negative"
{
    PageType = List; SourceTable = "Bool Entry"; SourceTableView = where(Positive = const(false));
    layout { area(Content) { repeater(R) { field("Entry No."; Rec."Entry No.") { } } } }
}
codeunit 50133 T
{
    procedure Seed()
    var E: Record "Bool Entry";
    begin
        E."Entry No." := 1; E.Positive := true; E.Insert();
        E."Entry No." := 2; E.Positive := false; E.Insert();
    end;

    procedure PositiveView(): Text
    var P: TestPage "Bool Positive";
    begin
        Seed();
        P.OpenView();
        exit(P.Filter.GetFilter(Positive));
    end;

    procedure NegativeView(): Text
    var P: TestPage "Bool Negative";
    begin
        Seed();
        P.OpenView();
        exit(P.Filter.GetFilter(Positive));
    end;
}
"#;

#[test]
fn a_view_const_true_on_a_boolean_reads_one() {
    assert_eq!(text(run_in(SRC, "T", "PositiveView")), "1");
    let r = run_in(SRC, "T", "NegativeView");
    assert!(matches!(&r, Err(Signal::Unsupported(_))), "{r:?}");
}
