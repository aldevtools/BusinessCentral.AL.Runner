//! A page opened on a record: the row it lands on, the marks it takes over, when OnInit runs, what a Page-typed handler
//! reads, and the error of a page id that does not exist. Each test names the corpus test it mirrors.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62500 "OR Row"
{
    fields { field(1; "No."; Code[10]) { } field(2; Grp; Code[10]) { } }
    keys { key(PK; "No.") { } }
}
table 62501 "OR Int Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[20]) { } }
    keys { key(PK; Id) { } }
}
page 62502 "OR List"
{
    PageType = List; SourceTable = "OR Row";
    layout { area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } } }
}
page 62503 "OR Card"
{
    PageType = Card; SourceTable = "OR Int Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
}
codeunit 62504 "OR Log"
{
    SingleInstance = true;
    var Trail: Text;
    procedure Reset() begin Trail := ''; end;
    procedure Add(T: Text) begin Trail += T; end;
    procedure Get(): Text begin exit(Trail); end;
}
page 62505 "OR Life"
{
    PageType = Card; SourceTable = "OR Int Row";
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
    trigger OnInit() var L: Codeunit "OR Log"; begin L.Add('I'); end;
    trigger OnOpenPage() var L: Codeunit "OR Log"; begin L.Add('O'); end;
}
"#;

const HELPERS: &str = r#"
    procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
    local procedure Seed()
    var R: Record "OR Row"; I: Record "OR Int Row";
    begin
        R.DeleteAll(); I.DeleteAll();
        R."No." := 'A'; R.Grp := 'x'; R.Insert();
        R."No." := 'B'; R.Grp := 'y'; R.Insert();
        R."No." := 'C'; R.Grp := 'x'; R.Insert();
        R."No." := 'D'; R.Grp := 'x'; R.Insert();
        I.Id := 1; I.Insert(); I.Id := 2; I.Insert(); I.Id := 3; I.Insert();
    end;
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src =
        format!("{OBJECTS}\ncodeunit 62590 T {{ Subtype = Test; var AtOpen: Text; Rows: Text; {HELPERS} {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

const LIST_HANDLER: &str = r#"
    [ModalPageHandler] procedure ListH(var P: TestPage "OR List")
    begin
        AtOpen := P.NoCtl.Value();
        Rows := '';
        if P.First() then repeat Rows += '[' + P.NoCtl.Value() + ']'; until not P.Next();
    end;
"#;

// misc/W2PageProperties.al:234 SetTableView_DescendingOrder: the page opens `=><` from the blank record, so a descending view
// lands on its last row.
#[test]
fn a_descending_view_handed_by_set_table_view_opens_on_its_last_row() {
    passes(&format!(
        "{LIST_HANDLER}
        [Test] [HandlerFunctions('ListH')]
        procedure Desc() var R: Record \"OR Row\"; Pg: Page \"OR List\"; begin
            Seed(); R.SetRange(Grp, 'x'); R.Ascending(false); Pg.SetTableView(R); Pg.RunModal();
            Check('A', AtOpen); Check('[D][C][A][]', Rows); end;"
    ));
}

// platform/lt-xmlport-session-navapp-cookie/P94PageRun.al:88 Run_WithRecordNotInTable: a record past the last row opens on the last.
#[test]
fn a_record_past_the_last_row_opens_the_card_on_the_last_row() {
    passes(
        "[Test] [HandlerFunctions('CardH')]
        procedure Ghost() var I: Record \"OR Int Row\"; begin
            Seed(); I.Id := 99; Page.Run(Page::\"OR Card\", I); Check('3', AtOpen); end;
        [PageHandler] procedure CardH(var P: TestPage \"OR Card\") begin AtOpen := P.IdCtl.Value(); end;",
    );
}

// misc/lt-protected-var-abs-query-views/P146ViewMarkedTemp.al:304 P146_PageRunModal_MarkedOnly_Real: the page shows the marked rows only.
#[test]
fn page_run_modal_with_a_marked_only_record_shows_the_marked_rows() {
    passes(&format!(
        "{LIST_HANDLER}
        [Test] [HandlerFunctions('ListH')]
        procedure Marks() var R: Record \"OR Row\"; begin
            Seed(); R.Get('A'); R.Mark(true); R.Get('C'); R.Mark(true); R.MarkedOnly(true);
            Page.RunModal(Page::\"OR List\", R); Check('[A][C][]', Rows); end;"
    ));
}

// pages/lt-page-variable-lifecycle/PageVariableLifecycle.al:158 PageVar_MethodCallBeforeRun: SetRecord constructs the page.
#[test]
fn set_record_on_a_page_variable_runs_on_init() {
    passes(
        "[Test] [HandlerFunctions('LifeH')]
        procedure SetRec() var L: Codeunit \"OR Log\"; P: Page \"OR Life\"; I: Record \"OR Int Row\"; begin
            Seed(); L.Reset(); I.Get(1); P.SetRecord(I); Check('I', L.Get()); P.Run(); Check('IO', L.Get()); end;
        [PageHandler] procedure LifeH(var TP: TestPage \"OR Life\") begin end;",
    );
}

// pages/lt-page-variable-lifecycle/PageVariableLifecycle.al:131 PageVar_RunTwiceOnOneVariable: a second non-modal Run runs OnInit again.
#[test]
fn a_second_non_modal_run_of_a_page_variable_runs_on_init_again() {
    passes(
        "[Test] [HandlerFunctions('LifeH')]
        procedure Twice() var L: Codeunit \"OR Log\"; P: Page \"OR Life\"; I: Record \"OR Int Row\"; begin
            Seed(); L.Reset(); I.Get(1); P.SetRecord(I); P.Run(); Check('IO', L.Get()); P.Run(); Check('IOIO', L.Get()); end;
        [PageHandler] procedure LifeH(var TP: TestPage \"OR Life\") begin end;",
    );
}

// errors/lt-ms-refusals-128/P128ModalOnInit.al:210 P128_TypedModalHandler_PageWithOnInit_PageVariable: a page variable keeps its
// own LookupMode (No) in a Page-typed handler; the static RunModal whose answer is used reads Yes (:191; called as a statement
// it is no lookup, pages/lt-be-runmodal-onlookup/BeRunModalOnLookup.al:916).
#[test]
fn a_page_typed_handler_reads_the_lookup_mode_of_the_page_variable() {
    passes(
        "[Test] [HandlerFunctions('TypedH')]
        procedure Typed() var L: Codeunit \"OR Log\"; P: Page \"OR Life\"; A: Action; begin
            L.Reset(); P.RunModal(); Check('Ilookup=No', L.Get());
            L.Reset(); A := Page.RunModal(Page::\"OR Life\"); Check('Ilookup=Yes', L.Get()); end;
        [ModalPageHandler] procedure TypedH(var P: Page \"OR Life\"; var Response: Action)
        var L: Codeunit \"OR Log\"; begin L.Add('lookup=' + Format(P.LookupMode())); Response := Action::LookupOK; end;",
    );
}

// platform/lt-xmlport-session-navapp-cookie/P94PageRun.al:211 Run_UnknownPage and misc/W5MissingObject.al:52: inside a try
// function the error names the test codeunit, not the test runner.
#[test]
fn a_missing_page_run_in_a_try_function_names_the_test_codeunit() {
    passes(
        "[Test]
        procedure Missing() begin
            if TryRun(62999) then Error('ran');
            Check('You tried to invoke the Page object with the ID 62999 from the object T. An object with that ID does not exist in the current application compiled with emit version 29000.', GetLastErrorText());
        end;
        [TryFunction] local procedure TryRun(Id: Integer) begin Page.Run(Id); end;",
    );
}

const TREE: &str = r#"
table 62520 "OR Tree Row" { fields { field(1; Id; Code[10]) { } field(2; Lvl; Integer) { } } keys { key(PK; Id) { } } }
page 62521 "OR Tree"
{
    PageType = List; SourceTable = "OR Tree Row";
    layout { area(Content) { repeater(R) { IndentationColumn = Rec.Lvl; ShowAsTree = true; field(IdCtl; Rec.Id) { } } } }
}
codeunit 62522 T
{
    Subtype = Test;
    procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
    local procedure Add(Id: Code[10]; Lvl: Integer) var R: Record "OR Tree Row"; begin R.Id := Id; R.Lvl := Lvl; R.Insert(); end;
    local procedure Rows(var P: TestPage "OR Tree"): Text
    var Result: Text;
    begin
        if P.First() then repeat Result += P.IdCtl.Value() + ' '; until not P.Next();
        exit(Result);
    end;
    [Test] [HandlerFunctions('TreeH')]
    procedure Tree()
    begin
        Add('T1', 0); Add('T2', 1); Add('T3', 2); Add('T4', 1); Add('T5', 0);
        Page.Run(Page::"OR Tree");
    end;
    [PageHandler] procedure TreeH(var P: TestPage "OR Tree")
    begin
        Check('T1 T5 ', Rows(P));
        P.First(); Check('No', Format(P.IsExpanded()));
        P.Expand(true); Check('Yes', Format(P.IsExpanded()));
        Check('T1 T2 T4 T5 ', Rows(P));
        P.GoToKey('T1'); P.Expand(false);
        Check('T1 T5 ', Rows(P));
        Check('No', Format(P.GoToKey('T2')));
        P.GoToKey('T5'); Check('No', Format(P.IsExpanded()));
    end;
}
"#;

// pages/lt-ah-pages/AhPageTests.al:981 Tree_ThroughPageRun: a tree offers no blank new line; Expand(true) shows the direct
// children of the row, Expand(false) hides them again.
#[test]
fn expand_on_a_tree_shows_and_hides_the_direct_children() {
    let o = al_testkit::outcomes_in(TREE, "T");
    assert_eq!(o, vec![("Tree".to_string(), Outcome::Pass)]);
}

// record/lt-loadfields-pagesource-validate/P81PageTempSource.al:238-242 RunModal_TempRecord_NewRow: a new line saved below a row it
// does not follow in the key order keeps its place in the client; where First goes then is not pinned, so it is refused.
#[test]
fn first_after_a_new_line_saved_out_of_key_order_is_refused() {
    let o = outcomes(
        "[Test] [HandlerFunctions('NewH')]
        procedure NewRow() begin Seed(); Page.RunModal(Page::\"OR List\"); end;
        [ModalPageHandler] procedure NewH(var P: TestPage \"OR List\") begin
            P.First(); P.New(); P.NoCtl.SetValue('0'); P.First(); end;",
    );
    assert!(
        matches!(&o[0].1, Outcome::NotRun(why) if why.contains("client's row order")),
        "{o:?}"
    );
}
