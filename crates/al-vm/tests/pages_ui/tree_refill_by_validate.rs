//! A filter field's OnValidate that empties and refills the temporary source of a collapsed tree Worksheet (ShowAsTree,
//! TreeInitialState = CollapseAll): the TestPage walks the new top-level rows with the text OnAfterGetRecord gives them, as the
//! flat Worksheet does (al-runner issue 6550, a follow-up to 5779 and 6523; passes on BC 28.5 and 29.0 per the reporter).

const SRC: &str = r#"
table 51610 "TR Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Indent; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
page 51611 "TR Tree"
{
    PageType = Worksheet; SourceTable = "TR Row"; SourceTableTemporary = true;
    InsertAllowed = false; ModifyAllowed = false; DeleteAllowed = false;
    layout
    {
        area(Content)
        {
            group(Filters) { field(GroupFilterField; GroupFilter) { trigger OnValidate() begin Fill(); end; } }
            repeater(Rows)
            {
                Editable = false; IndentationColumn = Rec.Indent; IndentationControls = RowName; ShowAsTree = true; TreeInitialState = CollapseAll;
                field(RowName; NameText) { }
            }
        }
    }
    var GroupFilter: Text[10]; NameText: Text; IsChild: Boolean; EndOnLast: Boolean; Log: Codeunit "TR Log";
    trigger OnOpenPage() begin Fill(); end;
    trigger OnAfterGetRecord() begin NameText := Rec.Name; Log.Add('R:' + Rec.Name); end;
    trigger OnAfterGetCurrRecord() begin IsChild := Rec.Indent > 0; Log.Add('C:' + Rec.Name); end;
    procedure SetGroups(NewGroups: Text[10]) begin GroupFilter := NewGroups; end;
    procedure SetEndOnLast() begin EndOnLast := true; end;
    local procedure Fill()
    var i: Integer; NextId: Integer;
    begin
        Rec.Reset();
        Rec.DeleteAll();
        for i := 1 to StrLen(GroupFilter) do begin
            InsertRow(NextId, CopyStr(GroupFilter, i, 1), 0);
            InsertRow(NextId, CopyStr(GroupFilter, i, 1) + '-1', 1);
            InsertRow(NextId, CopyStr(GroupFilter, i, 1) + '-2', 1);
        end;
        if EndOnLast then begin
            if Rec.FindLast() then;
        end else
            if Rec.FindFirst() then;
    end;
    local procedure InsertRow(var NextId: Integer; NewName: Text[30]; NewIndent: Integer)
    begin
        NextId += 1;
        Rec.Init(); Rec.Id := NextId; Rec.Name := NewName; Rec.Indent := NewIndent; Rec.Insert();
    end;
}
codeunit 51614 "TR Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
codeunit 51613 C
{
    local procedure OpenTree(var TreePage: TestPage "TR Tree"; Groups: Text[10])
    var TreeWorksheet: Page "TR Tree";
    begin
        TreePage.Trap();
        TreeWorksheet.SetGroups(Groups);
        TreeWorksheet.Run();
    end;
    local procedure WalkTree(var TreePage: TestPage "TR Tree"; RootsOnly: Boolean) Names: Text
    begin
        if TreePage.First() then
            repeat
                if not (RootsOnly and (StrPos(TreePage.RowName.Value(), '-') > 0)) then
                    Names += TreePage.RowName.Value() + ',';
            until not TreePage.Next();
    end;
    procedure Fewer(): Text
    var TreePage: TestPage "TR Tree";
    begin
        OpenTree(TreePage, 'AB');
        TreePage.GroupFilterField.SetValue('B');
        exit(WalkTree(TreePage, false));
    end;
    procedure SameKeys(): Text
    var TreePage: TestPage "TR Tree";
    begin
        OpenTree(TreePage, 'D');
        TreePage.GroupFilterField.SetValue('AB');
        exit(WalkTree(TreePage, true));
    end;
    procedure Expanded(): Text
    var TreePage: TestPage "TR Tree";
    begin
        OpenTree(TreePage, 'AB');
        TreePage.First();
        TreePage.Expand(true);
        TreePage.First();
        TreePage.GroupFilterField.SetValue('B');
        exit(Format(TreePage.IsExpanded()) + '|' + WalkTree(TreePage, false));
    end;
    procedure KeptKey(): Text
    var TreePage: TestPage "TR Tree"; S: Text;
    begin
        OpenTree(TreePage, 'AB');
        TreePage.First();
        TreePage.Next();
        TreePage.GroupFilterField.SetValue('CD');
        S := TreePage.RowName.Value() + '|';
        exit(S + WalkTree(TreePage, false));
    end;
    procedure FewerLog(): Text
    var TreePage: TestPage "TR Tree"; Log: Codeunit "TR Log"; S: Text;
    begin
        Log.Take();
        OpenTree(TreePage, 'AB');
        S := Log.Take() + '|';
        TreePage.GroupFilterField.SetValue('B');
        exit(S + Log.Take());
    end;
    procedure EndOnLastRow(): Text
    var TreePage: TestPage "TR Tree"; TreeWorksheet: Page "TR Tree"; Log: Codeunit "TR Log"; S: Text;
    begin
        Log.Take();
        TreePage.Trap();
        TreeWorksheet.SetGroups('A');
        TreeWorksheet.SetEndOnLast();
        TreeWorksheet.Run();
        S := Log.Take() + '|';
        TreePage.GroupFilterField.SetValue('AB');
        S += Log.Take() + '|' + TreePage.RowName.Value() + '|' + Format(TreePage.IsExpanded()) + '|';
        S += WalkTree(TreePage, false) + '|';
        exit(S + Log.Take());
    end;
    procedure ExpandAfterRefill(): Text
    var TreePage: TestPage "TR Tree"; Log: Codeunit "TR Log"; S: Text;
    begin
        OpenTree(TreePage, 'D');
        TreePage.GroupFilterField.SetValue('AB');
        TreePage.First();
        Log.Take();
        TreePage.Expand(true);
        S := Log.Take() + '|' + Format(TreePage.IsExpanded()) + '|';
        exit(S + WalkTree(TreePage, false));
    end;
    procedure Empty(): Text
    var TreePage: TestPage "TR Tree";
    begin
        OpenTree(TreePage, 'AB');
        TreePage.GroupFilterField.SetValue('');
        exit(WalkTree(TreePage, false));
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

// Issue 6550 TreeFilterRefilledWithFewerRowsListsTheNewRootRow: refilled from A and B to B only, the walk is `B,`.
#[test]
fn a_tree_refilled_with_fewer_rows_by_a_filter_field_walks_the_new_top_level_rows() {
    assert_eq!(run("Fewer"), "B,");
}

// Issue 6550 TreeFilterRefilledUnderTheSameKeysListsTheNewRootRows: refilled from D to A and B, the top-level rows are `A,B,`.
#[test]
fn a_tree_refilled_under_the_same_keys_by_a_filter_field_shows_the_new_rows() {
    assert_eq!(run("SameKeys"), "A,B,");
}

// Y34_Tree_ExpandedBefore_AB_To_B: the expanded key carries to the new row under it: `Yes`, rows `B B-1 B-2`.
#[test]
fn a_tree_refilled_while_a_row_is_expanded_keeps_that_key_expanded() {
    assert_eq!(run("Expanded"), "Yes|B,B-1,B-2,");
}

// Y34_Tree_SameKeys_AB_To_CD_FromSecondRoot: the client keeps key 4 (now D, not Rec's row C) and walks `C D`.
#[test]
fn a_tree_refill_keeps_the_held_key_when_it_came_back() {
    assert_eq!(run("KeptKey"), "D|C,D,");
}

// Y34_Tree_ToEmpty_AB: the reads of a refill to no rows (`C:;`) are not built, so it is refused rather than kept stale.
#[test]
fn a_tree_refilled_to_no_rows_is_refused() {
    let r = al_testkit::run_in(SRC, "C", "Empty");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(m)) if m.contains("left no rows")),
        "{r:?}"
    );
}

// Y34_Tree_Fewer_AB_To_B: the client reads every row of the tree, then the row twice and OnAfterGetCurrRecord
// (`open log` `R:A;R:A-1;R:A-2;R:B;R:B-1;R:B-2;R:A;R:A;C:A;`, `validate log` `R:B;R:B-1;R:B-2;R:B;R:B;C:B;`).
#[test]
fn a_tree_refill_reads_every_row_of_the_tree() {
    assert_eq!(
        run("FewerLog"),
        "R:A;R:A-1;R:A-2;R:B;R:B-1;R:B-2;R:A;R:A;C:A;|R:B;R:B-1;R:B-2;R:B;R:B;C:B;"
    );
}

// Y34_Tree_EndOnLastRow_A_To_AB: the client stands on key 3 (A-2, its parent expanded), Rec ends on the last row; the client keeps
// key 3, reads as the flat page does from a later row, and walks `A A-1 A-2 B`.
#[test]
fn a_tree_refill_from_a_child_row_keeps_it_and_reads_around_it() {
    assert_eq!(
        run("EndOnLastRow"),
        "R:A-2;R:A-1;R:A;R:A-2;R:A-2;C:A-2;|R:A-2;R:A-2;R:B;R:B-1;R:B-2;R:A-2;R:A-1;R:A;R:A-2;R:A-2;C:A-2;|A-2|No|A,A-1,A-2,B,|\
         R:A;R:A;C:A;R:A-1;R:A-1;C:A-1;R:A-2;R:A-2;C:A-2;R:B;R:B;C:B;"
    );
}

// Y34_Tree_ExpandAfterRefill_D_To_AB: Expand on a new top-level row reads nothing and shows its children.
#[test]
fn expand_after_a_tree_refill_shows_the_new_children() {
    assert_eq!(run("ExpandAfterRefill"), "|Yes|A,A-1,A-2,B,");
}
