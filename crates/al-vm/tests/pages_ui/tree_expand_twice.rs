//! Expand(true) on a tree row already expanded and Expand(false) on one not expanded change nothing: no error, IsExpanded
//! unchanged, the same rows show (pages/lt-page-runtime/P82TestPage.al:283-289 Tree_Expand_IsExpanded, :336-341
//! Tree_Rows_WhileCollapsed). A leaf row stays refused (gap G5-17).

use al_builtins::native::Signal;

const SRC: &str = r#"
table 51620 "TX Row" { fields { field(1; "No."; Code[10]) { } field(2; Indent; Integer) { } } keys { key(PK; "No.") { } } }
page 51621 "TX Tree"
{
    PageType = List; SourceTable = "TX Row";
    layout { area(Content) { repeater(Rows) { IndentationColumn = Rec.Indent; ShowAsTree = true; field(NoCtl; Rec."No.") { } } } }
}
codeunit 51622 C
{
    local procedure Seed()
    var R: Record "TX Row";
    begin
        R."No." := 'N1'; R.Indent := 0; R.Insert();
        R."No." := 'N2'; R.Indent := 1; R.Insert();
        R."No." := 'N3'; R.Indent := 0; R.Insert();
    end;
    local procedure Walk(var Tree: TestPage "TX Tree") T: Text
    begin
        if Tree.First() then repeat T += Tree.NoCtl.Value(); until not Tree.Next();
    end;
    procedure Twice(): Text
    var Tree: TestPage "TX Tree"; T: Text;
    begin
        Seed();
        Tree.OpenView();
        Tree.GoToKey('N1');
        Tree.Expand(false);
        T := Format(Tree.IsExpanded()) + Walk(Tree);
        Tree.GoToKey('N1');
        Tree.Expand(true);
        Tree.Expand(true);
        T += '|' + Format(Tree.IsExpanded()) + Walk(Tree);
        Tree.GoToKey('N1');
        Tree.Expand(false);
        Tree.Expand(false);
        exit(T + '|' + Format(Tree.IsExpanded()) + Walk(Tree));
    end;
    procedure Leaf(): Text
    var Tree: TestPage "TX Tree";
    begin
        Seed();
        Tree.OpenView();
        Tree.GoToKey('N3');
        Tree.Expand(true);
    end;
}
"#;

#[test]
fn a_second_expand_or_collapse_changes_nothing() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Twice")),
        "NoN1N3|YesN1N2N3|NoN1N3"
    );
}

#[test]
fn expand_of_a_leaf_row_stays_refused() {
    assert!(matches!(
        al_testkit::run_in(SRC, "C", "Leaf"),
        Err(Signal::Unsupported(_))
    ));
}
