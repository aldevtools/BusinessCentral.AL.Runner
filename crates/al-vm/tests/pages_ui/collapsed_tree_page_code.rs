//! The AL code of a collapsed tree page (ShowAsTree, TreeInitialState = CollapseAll) sees every row of Rec; the collapse is only what
//! the TestPage shows (al-runner issue 6523; corpus writer Y28, branch al-runtime-rs-staging-y28,
//! pageext/lt-page-ext-properties/Y28CollapsedTreeTests.al, measured on BC 29 SaaS). The TestPage shows the rows without a parent in
//! the view, plus the children of the parent of the row Rec stands on after OnOpenPage or an action.

const SRC: &str = r#"
table 51600 "CT Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Indent; Integer) { } } keys { key(PK; Id) { } } }
codeunit 51601 "CT Mode"
{
    SingleInstance = true;
    var M: Text; E: Text;
    procedure Set(T: Text) begin M := T; end;
    procedure Get(): Text begin exit(M); end;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51602 "CT Tree"
{
    PageType = List; SourceTable = "CT Row";
    layout { area(Content) { repeater(Rows) { IndentationColumn = Rec.Indent; ShowAsTree = true; TreeInitialState = CollapseAll; field(IdCtl; Rec.Id) { } field(NameCtl; Rec.Name) { } } } }
    actions
    {
        area(Processing)
        {
            action(CountAct) { trigger OnAction() begin Mode.Add('act=' + Format(Rec.Count()) + '/' + Rec.GetFilters()); end; }
            action(ResetAct) { trigger OnAction() begin Rec.Reset(); Mode.Add('reset=' + Format(Rec.Count())); CurrPage.Update(false); end; }
            action(GoChildAct) { trigger OnAction() begin Mode.Add('get2=' + Format(Rec.Get(2))); CurrPage.Update(false); end; }
        }
    }
    trigger OnOpenPage()
    begin
        case Mode.Get() of
            'reset':
                Rec.Reset();
            'findset':
                if Rec.FindSet() then
                    repeat
                    until Rec.Next() = 0;
            'get':
                Rec.Get(2);
            'children':
                Rec.SetRange(Indent, 1);
        end;
        Mode.Add('open=' + Format(Rec.Count()) + '/' + Rec.GetFilters());
    end;
    var Mode: Codeunit "CT Mode";
}
page 51604 "CT Tree Log"
{
    PageType = List; SourceTable = "CT Row";
    layout { area(Content) { repeater(Rows) { IndentationColumn = Rec.Indent; ShowAsTree = true; TreeInitialState = CollapseAll; field(IdCtl; Rec.Id) { } } } }
    actions
    {
        area(Processing)
        {
            action(ResetAct) { trigger OnAction() begin Rec.Reset(); Mode.Add('reset count=' + Format(Rec.Count())); CurrPage.Update(false); end; }
            action(GoChildAct) { trigger OnAction() begin Mode.Add('get2=' + Format(Rec.Get(2))); CurrPage.Update(false); end; }
        }
    }
    trigger OnAfterGetRecord() begin Mode.Add('agr' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Mode.Add('agcr' + Format(Rec.Id) + ' n=' + Format(Rec.Count())); end;
    var Mode: Codeunit "CT Mode";
}
page 51605 "CT Var Tree"
{
    PageType = List; SourceTable = "CT Row";
    layout { area(Content) { repeater(Rows) { IndentationColumn = Level; ShowAsTree = true; TreeInitialState = CollapseAll; field(IdCtl; Rec.Id) { } } } }
    trigger OnAfterGetRecord() begin Level := Rec.Indent; end;
    var Level: Integer;
}
codeunit 51603 C
{
    local procedure Rows()
    var R: Record "CT Row";
    begin
        R.DeleteAll();
        Add(1, 'Root 1', 0); Add(2, 'Child 1A', 1); Add(3, 'Child 1B', 1); Add(4, 'Root 2', 0); Add(5, 'Child 2A', 1);
    end;
    local procedure Add(I: Integer; N: Text[30]; L: Integer)
    var R: Record "CT Row";
    begin
        R.Id := I; R.Name := N; R.Indent := L; R.Insert();
    end;
    local procedure Walk(var P: TestPage "CT Tree") S: Text
    begin
        if P.First() then
            repeat
                S += P.IdCtl.Value() + ',';
            until not P.Next();
    end;
    procedure Open(M: Text): Text
    var Mode: Codeunit "CT Mode"; P: TestPage "CT Tree"; S: Text;
    begin
        Rows();
        Mode.Set(M);
        Mode.Take();
        P.OpenView();
        S := Mode.Take() + '|' + P.IdCtl.Value() + '|' + Walk(P);
        P.Close();
        exit(S);
    end;
    procedure Action(A: Text): Text
    var Mode: Codeunit "CT Mode"; P: TestPage "CT Tree"; S: Text;
    begin
        Rows();
        Mode.Set('');
        P.OpenView();
        Mode.Take();
        case A of
            'count':
                begin
                    P.Last();
                    P.CountAct.Invoke();
                end;
            'reset':
                P.ResetAct.Invoke();
            'get':
                P.GoChildAct.Invoke();
        end;
        S := Mode.Take() + '|' + P.IdCtl.Value() + '|' + Walk(P);
        P.Close();
        exit(S);
    end;
    procedure OpenLog(): Text
    var Mode: Codeunit "CT Mode"; P: TestPage "CT Tree Log"; S: Text;
    begin
        Rows();
        Mode.Take();
        P.OpenView();
        S := Mode.Take() + '|' + P.IdCtl.Value() + '|';
        if P.First() then
            repeat
                S += P.IdCtl.Value() + ',';
            until not P.Next();
        S += '|' + Mode.Take();
        P.Close();
        exit(S);
    end;
    procedure LogResetAction(): Text
    begin
        exit(LogAction(false));
    end;
    procedure LogGetAction(): Text
    begin
        exit(LogAction(true));
    end;
    local procedure LogAction(Get: Boolean): Text
    var Mode: Codeunit "CT Mode"; P: TestPage "CT Tree Log"; S: Text;
    begin
        Rows();
        P.OpenView();
        Mode.Take();
        if Get then
            P.GoChildAct.Invoke()
        else
            P.ResetAct.Invoke();
        S := Mode.Take() + '|' + P.IdCtl.Value() + '|';
        if P.First() then
            repeat
                S += P.IdCtl.Value() + ',';
            until not P.Next();
        exit(S);
    end;
    procedure TestPageSetFilter(): Text
    var P: TestPage "CT Tree"; S: Text;
    begin
        Rows();
        P.OpenView();
        P.Filter.SetFilter(Indent, '1');
        S := Walk(P) + '|';
        P.Filter.SetFilter(Indent, '');
        P.Filter.SetFilter(Id, '2..4');
        exit(S + Walk(P));
    end;
    procedure VarTree(): Text
    var P: TestPage "CT Var Tree"; S: Text;
    begin
        Rows();
        P.OpenView();
        if P.First() then
            repeat
                S += P.IdCtl.Value() + ',';
            until not P.Next();
        exit(S);
    end;
    procedure TestPageFilter(): Text
    var P: TestPage "CT Tree";
    begin
        Rows();
        P.OpenView();
        exit('[' + P.Filter.GetFilter(Indent) + '][' + P.Filter.GetFilter(Id) + ']');
    end;
    procedure OpenPlain(): Text begin exit(Open('')); end;
    procedure OpenReset(): Text begin exit(Open('reset')); end;
    procedure OpenFindSet(): Text begin exit(Open('findset')); end;
    procedure OpenGet(): Text begin exit(Open('get')); end;
    procedure OpenChildren(): Text begin exit(Open('children')); end;
    procedure ActCount(): Text begin exit(Action('count')); end;
    procedure ActReset(): Text begin exit(Action('reset')); end;
    procedure ActGet(): Text begin exit(Action('get')); end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

// Y28CollapsedTreeTests.al Y28_CollapsedOpenAndWalk: `open count=5 filters=`, current 1, walk 1,4.
#[test]
fn onopenpage_counts_every_row_of_a_collapsed_tree() {
    assert_eq!(run("OpenPlain"), "open=5/;|1|1,4,");
}

// Y28_ResetInOnOpenPage: Rec.Reset() changes nothing; the TestPage still walks the top-level rows.
#[test]
fn reset_in_onopenpage_keeps_the_collapse() {
    assert_eq!(run("OpenReset"), "open=5/;|1|1,4,");
}

// Y28_FindSetInOnOpenPage: Rec left on row 5, the page opens on it and its parent shows expanded.
#[test]
fn findset_in_onopenpage_opens_on_the_last_row_with_its_parent_expanded() {
    assert_eq!(run("OpenFindSet"), "open=5/;|5|1,4,5,");
}

// Y28_GetChildInOnOpenPage: Rec.Get(2) opens on row 2 with Root 1 expanded.
#[test]
fn get_of_a_child_in_onopenpage_expands_its_parent() {
    assert_eq!(run("OpenGet"), "open=5/;|2|1,2,3,4,");
}

// Y28_SetRangeChildrenInOnOpenPage: rows without a parent in the view show as top level.
#[test]
fn rows_without_a_parent_in_the_view_show_as_top_level() {
    assert_eq!(run("OpenChildren"), "open=3/Indent: 1;|2|2,3,5,");
}

// Y28_ActionCountOnRoot: an action sees every row and no filter.
#[test]
fn an_action_counts_every_row() {
    assert_eq!(run("ActCount"), "act=5/;|4|1,4,");
}

// Y28_ActionResetAndUpdate: `reset count=5`, current 1, walk 1,4.
#[test]
fn an_action_that_resets_rec_keeps_the_collapse() {
    assert_eq!(run("ActReset"), "reset=5;|1|1,4,");
}

// Y28_ActionGetChildAndUpdate: `get2=Yes`, current 2, walk 1,2,3,4.
#[test]
fn an_action_that_gets_a_child_expands_its_parent() {
    assert_eq!(run("ActGet"), "get2=Yes;|2|1,2,3,4,");
}

// Y28_CollapsedOpenAndWalk: a collapsed tree with row triggers reads every row when it opens, as an ExpandAll tree does
// (`agr1;agr2;agr3;agr4;agr5;agr1;agr1;agcr1 n=5;`), shows the top-level rows and reads `agr4;agr4;agcr4 n=5;` on the walk.
#[test]
fn a_collapsed_tree_with_row_triggers_reads_every_row_when_it_opens() {
    assert_eq!(
        run("OpenLog"),
        "agr1;agr2;agr3;agr4;agr5;agr1;agr1;agcr1 n=5;|1|1,4,|agr4;agr4;agcr4 n=5;"
    );
}

// Y28_CollapsedOpenAndWalk `testpage-filter-indent` reads '': the collapse is no filter of the page.
#[test]
fn the_testpage_filter_holds_no_collapse() {
    assert_eq!(run("TestPageFilter"), "[][]");
}

// Y28_ActionResetAndUpdate: the refresh after an action reads every row again, the hidden ones too
// (`agr1;reset count=5;agr1;agr1;agr2;agr3;agr4;agr5;agr1;agr1;agcr1 n=5;`), current 1, walk 1,4.
#[test]
fn the_refresh_after_an_action_reads_every_row_of_a_collapsed_tree() {
    assert_eq!(
        run("LogResetAction"),
        "agr1;reset count=5;agr1;agr1;agr2;agr3;agr4;agr5;agr1;agr1;agcr1 n=5;|1|1,4,"
    );
}

// Y28_ActionGetChildAndUpdate: `agr1;get2=Yes;agr2;agr2;agr3;agr4;agr5;agr2;agr1;agr2;agr2;agcr2 n=5;`, current 2, walk 1,2,3,4.
#[test]
fn an_action_that_gets_a_child_of_a_tree_with_row_triggers_reads_from_it() {
    assert_eq!(
        run("LogGetAction"),
        "agr1;get2=Yes;agr2;agr2;agr3;agr4;agr5;agr2;agr1;agr2;agr2;agcr2 n=5;|2|1,2,3,4,"
    );
}

// Y28_TestPageFilterOnChildren: a TestPage filter changes the view; rows without a parent in it show as top level.
#[test]
fn a_testpage_filter_shows_the_rows_without_a_parent_in_the_new_view() {
    assert_eq!(run("TestPageSetFilter"), "2,3,5,|2,3,4,");
}

// Y28_VariableIndentationColumn: a tree over a page variable shows the top-level rows too (walk 1,4); which rows a variable puts at
// level 0 is not modelled, so a move between its rows is refused rather than shown whole.
#[test]
fn a_tree_over_a_page_variable_is_refused() {
    let r = al_testkit::run_in(SRC, "C", "VarTree");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(m)) if m.contains("IndentationColumn")),
        "{r:?}"
    );
}
