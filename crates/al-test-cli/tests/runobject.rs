//! `RunObject` actions of the headless test client (ADR-012 follow-up): AL source → compile →
//! run as a test codeunit. Each test names the corpus test it mirrors.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 61000 "RO Row"
{
    fields { field(1; "No."; Code[20]) { } field(2; Descr; Text[50]) { } field(3; Grp; Code[10]) { } field(4; Rank; Integer) { } }
    keys { key(PK; "No.") { } key(ByRank; Rank) { } }
}
table 61001 "RO Line"
{
    fields { field(1; "Head No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Descr; Text[50]) { } }
    keys { key(PK; "Head No.", "Line No.") { } }
}
codeunit 61002 "RO Probe"
{
    SingleInstance = true;
    var Trail: Text; Seen: Text; Groups: Text;
    procedure Reset() begin Trail := ''; Seen := ''; Groups := ''; end;
    procedure Note(T: Text) begin if Trail <> '' then Trail += ','; Trail += T; end;
    procedure Trace(): Text begin exit(Trail); end;
    procedure SetSeen(S: Text) begin Seen := S; end;
    procedure GetSeen(): Text begin exit(Seen); end;
    procedure SetGroups(S: Text) begin Groups := S; end;
    procedure GetGroups(): Text begin exit(Groups); end;
}
page 61010 "RO Card"
{
    PageType = Card; SourceTable = "RO Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } field(DescrCtl; Rec.Descr) { } } }
    trigger OnOpenPage() var P: Codeunit "RO Probe"; begin P.Note('open:' + Rec."No."); end;
    trigger OnNewRecord(BelowxRec: Boolean) var P: Codeunit "RO Probe"; begin P.Note('new'); end;
}
page 61011 "RO Dialog"
{
    PageType = StandardDialog; SourceTable = "RO Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } field(DescrCtl; Rec.Descr) { } } }
    trigger OnOpenPage() var P: Codeunit "RO Probe"; begin P.Note('dialog-open:' + Rec."No."); end;
}
page 61012 "RO Lines"
{
    PageType = List; SourceTable = "RO Line"; Editable = false;
    layout { area(Content) { repeater(R) { field(Head; Rec."Head No.") { } field(LineNo; Rec."Line No.") { } field(DescrCtl; Rec.Descr) { } } } }
    trigger OnOpenPage() var P: Codeunit "RO Probe"; begin
        Rec.FilterGroup(0); P.SetGroups('g0=' + Rec.GetFilter("Head No.")); Rec.FilterGroup(0); end;
}
page 61013 "RO Rank List"
{
    PageType = List; SourceTable = "RO Row"; Editable = false;
    layout { area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } field(RankCtl; Rec.Rank) { } } } }
    trigger OnOpenPage() var P: Codeunit "RO Probe"; begin
        Rec.FilterGroup(3); P.SetGroups('g3=' + Rec.GetFilter(Grp) + '|key=' + Rec.CurrentKey()); Rec.FilterGroup(0); end;
}
codeunit 61020 "RO Runner"
{
    TableNo = "RO Row";
    trigger OnRun() var P: Codeunit "RO Probe"; begin
        P.SetSeen(Rec.Descr);
        if Rec.Grp = 'MOVE' then begin Rec.Reset(); Rec.SetRange("No.", 'C'); Rec.FindFirst(); end;
        if Rec.Grp = 'WRITE' then begin Rec.Descr := 'Written'; Rec.Modify(); end;
    end;
}
report 61030 "RO Report" { ProcessingOnly = true; UseRequestPage = false; }
xmlport 61031 "RO XmlPort" { Direction = Export; Format = Xml; UseRequestPage = false; schema { textelement(Root) { } } }
query 61032 "RO Query" { elements { dataitem(Row; "RO Row") { column(No; "No.") { } } } }
page 61040 "RO Host"
{
    PageType = List; SourceTable = "RO Row";
    layout { area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } field(DescrCtl; Rec.Descr) { } field(GrpCtl; Rec.Grp) { } } } }
    actions
    {
        area(Processing)
        {
            action(RunCard) { RunObject = Page "RO Card"; RunPageOnRec = true; }
            action(RunCardPlain) { RunObject = Page "RO Card"; }
            action(RunDialog) { RunObject = Page "RO Dialog"; RunPageOnRec = true; }
            action(RunCardView) { RunObject = Page "RO Card"; RunPageMode = View; }
            action(RunCardCreate) { RunObject = Page "RO Card"; RunPageMode = Create; }
            action(RunDialogCreate) { RunObject = Page "RO Dialog"; RunPageMode = Create; }
            action(RunCodeunit) { RunObject = Codeunit "RO Runner"; }
            action(RunReport) { RunObject = Report "RO Report"; }
            action(RunXmlPort) { RunObject = XmlPort "RO XmlPort"; }
            action(RunQuery) { RunObject = Query "RO Query"; }
            action(RunRanked) { RunObject = Page "RO Rank List"; RunPageView = sorting(Rank) where(Grp = const('KEEP')); }
            action(HasTrigger) { trigger OnAction() var P: Codeunit "RO Probe"; begin P.Note('trigger:' + Rec."No."); end; }
        }
        area(Promoted) { group(Cat) { actionref(RunCard_Promoted; RunCard) { } } }
    }
}
table 61004 "RO Mod"
{
    fields { field(1; "No."; Code[20]) { } field(2; Descr; Text[50]) { } field(3; Grp; Code[10]) { } }
    keys { key(PK; "No.") { } }
    trigger OnModify() var P: Codeunit "RO Probe"; begin P.SetSeen(xRec.Descr + '>' + Rec.Descr); end;
}
page 61050 "RO Mod Card"
{
    PageType = Card; SourceTable = "RO Mod";
    layout { area(Content) { field(NoCtl; Rec."No.") { } field(DescrCtl; Rec.Descr) { } } }
}
page 61051 "RO Mod Host"
{
    PageType = List; SourceTable = "RO Mod";
    layout { area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } field(DescrCtl; Rec.Descr) { } field(GrpCtl; Rec.Grp) { } } } }
    actions { area(Processing) {
        action(OpenTarget) { RunObject = Page "RO Mod Card"; RunPageOnRec = true; }
        action(OpenView) { RunObject = Page "RO Mod Card"; RunPageMode = View; }
    } }
    trigger OnAfterGetRecord() begin Rec.Grp := 'CALC'; end;
}
page 61052 "RO Part Page"
{
    PageType = ListPart; SourceTable = "RO Mod";
    layout { area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } field(DescrCtl; Rec.Descr) { } } } }
}
page 61053 "RO Part Host"
{
    PageType = Card; SourceTable = "RO Mod";
    layout { area(Content) { field(NoCtl; Rec."No.") { } part(Lines; "RO Part Page") { } } }
}
page 61060 "RO Closing Card"
{
    PageType = Card; SourceTable = "RO Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
    trigger OnClosePage() var P: Codeunit "RO Probe"; begin P.Note('closed'); end;
}
page 61061 "RO View Card"
{
    PageType = Card; SourceTable = "RO Row"; SourceTableView = sorting(Rank);
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
}
page 61062 "RO Refused Host"
{
    PageType = List; SourceTable = "RO Row";
    layout { area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } } }
    actions { area(Processing) {
        action(Closing) { RunObject = Page "RO Closing Card"; }
        action(OnRecWithView) { RunObject = Page "RO View Card"; RunPageOnRec = true; }
        action(CreateOnRec) { RunObject = Page "RO Card"; RunPageMode = Create; RunPageOnRec = true; }
        action(Missing) { RunObject = Page "No Such Page"; }
        action(Odd) { RunObject = Page "RO Card"; RunPageMode = Strange; }
    } }
}
page 61063 "RO Both Host"
{
    PageType = Card; SourceTable = "RO Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
    actions { area(Processing) { action(Both) { RunObject = Page "RO Card"; trigger OnAction() begin Error('OnAction ran'); end; } } }
}
table 61003 "RO Head"
{
    fields { field(1; "No."; Code[20]) { } field(2; Descr; Text[50]) { } }
    keys { key(PK; "No.") { } }
}
page 61041 "RO Head Card"
{
    PageType = Card; SourceTable = "RO Head";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
    actions
    {
        area(Processing)
        {
            action(ByField) { RunObject = Page "RO Lines"; RunPageLink = "Head No." = field("No."); }
            action(ByConst) { RunObject = Page "RO Lines"; RunPageLink = "Head No." = const('H1'); }
            action(ByFilter) { RunObject = Page "RO Lines"; RunPageLink = "Head No." = filter('H1'|'H3'); }
        }
    }
}
page 61042 "RO Lines Reader"
{
    PageType = List; SourceTable = "RO Line"; Editable = false;
    layout { area(Content) { repeater(R) { field(Head; Rec."Head No.") { } } } }
    trigger OnOpenPage() var P: Codeunit "RO Probe"; begin P.SetSeen(Rec."Head No."); end;
}
page 61043 "RO Reader Host"
{
    PageType = Card; SourceTable = "RO Head";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
    actions { area(Processing) {
        action(Read) { RunObject = Page "RO Lines Reader"; RunPageLink = "Head No." = field("No."); }
        action(ReadOp) { RunObject = Page "RO Lines Op Reader"; RunPageLink = "Head No." = field("No."); }
    } }
}
page 61044 "RO Lines Op Reader"
{
    PageType = List; SourceTable = "RO Line"; Editable = false;
    layout { area(Content) { repeater(R) { field(Head; Rec."Head No.") { } } } }
    trigger OnOpenPage() var P: Codeunit "RO Probe"; begin
        case P.GetSeen() of
            'GetFilter': P.SetGroups(Rec.GetFilter("Head No."));
            'GetFilters': P.SetGroups(Rec.GetFilters());
            'GetView': P.SetGroups(Rec.GetView());
            'CurrentKey': P.SetGroups(Rec.CurrentKey());
            'HasFilter': P.SetGroups(Format(Rec.HasFilter()));
            'GetRangeMin': P.SetGroups(Rec.GetRangeMin("Head No."));
            'Count': P.SetGroups(Format(Rec.Count()));
            'IsEmpty': P.SetGroups(Format(Rec.IsEmpty()));
            'FindFirst': P.SetGroups(Format(Rec.FindFirst()));
        end;
    end;
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 61100 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

const SEED: &str = "local procedure Seed() var R: Record \"RO Row\"; P: Codeunit \"RO Probe\"; begin
    P.Reset(); R.DeleteAll();
    R.Init(); R.\"No.\" := 'A'; R.Descr := 'Alpha'; R.Grp := 'KEEP'; R.Rank := 30; R.Insert();
    R.Init(); R.\"No.\" := 'B'; R.Descr := 'Bravo'; R.Grp := 'DROP'; R.Rank := 5; R.Insert();
    R.Init(); R.\"No.\" := 'C'; R.Descr := 'Charlie'; R.Grp := 'KEEP'; R.Rank := 10; R.Insert();
    Commit();
end;
procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;";

// handlers/TestPageActionRunObject_Tests.al:RunObjectActionOpensTheTargetOnTheHostsCurrentRow
#[test]
fn a_page_action_opens_its_target_on_the_hosts_current_row_and_a_handler_answers_it() {
    passes(&format!(
        "{SEED}
        [Test] [HandlerFunctions('CardHandler')]
        procedure OnRec() var H: TestPage \"RO Host\"; P: Codeunit \"RO Probe\"; begin
            Seed(); H.OpenEdit(); H.First(); H.Next(); H.RunCard.Invoke(); H.Close();
            Check('open:B', P.Trace()); Check('Bravo', P.GetSeen()); end;
        [PageHandler] procedure CardHandler(var T: TestPage \"RO Card\") var P: Codeunit \"RO Probe\"; begin P.SetSeen(T.DescrCtl.Value()); end;"
    ));
}

// handlers/TestPageActionRunObject_Tests.al:ActionrefToARunObjectActionOpensTheSameTarget
#[test]
fn an_actionref_opens_what_its_action_opens() {
    passes(&format!(
        "{SEED}
        [Test] [HandlerFunctions('CardHandler')]
        procedure Promoted() var H: TestPage \"RO Host\"; P: Codeunit \"RO Probe\"; begin
            Seed(); H.OpenEdit(); H.First(); H.Next(); H.RunCard_Promoted.Invoke(); H.Close();
            Check('Bravo', P.GetSeen()); end;
        [PageHandler] procedure CardHandler(var T: TestPage \"RO Card\") var P: Codeunit \"RO Probe\"; begin P.SetSeen(T.DescrCtl.Value()); end;"
    ));
}

// handlers/TestPageActionRunObject_Tests.al:AnActionWithATriggerStillRunsItsTriggerAndOpensNothing
#[test]
fn an_action_with_a_trigger_runs_it_and_opens_nothing() {
    passes(&format!(
        "{SEED}
        [Test]
        procedure Trigger() var H: TestPage \"RO Host\"; P: Codeunit \"RO Probe\"; begin
            Seed(); H.OpenEdit(); H.First(); H.Next(); H.HasTrigger.Invoke(); H.Close();
            Check('trigger:B', P.Trace()); end;"
    ));
}

// handlers/TestPageActionRunObject_Tests.al:RunObjectActionToAStandardDialogTargetIsAnsweredByTheModalHandler
#[test]
fn a_standard_dialog_target_is_answered_by_the_modal_page_handler() {
    passes(&format!(
        "{SEED}
        [Test] [HandlerFunctions('DialogHandler')]
        procedure Dialog() var H: TestPage \"RO Host\"; P: Codeunit \"RO Probe\"; begin
            Seed(); H.OpenEdit(); H.First(); H.Next(); H.RunDialog.Invoke(); H.Close();
            Check('dialog-open:B', P.Trace()); Check('Bravo', P.GetSeen()); end;
        [ModalPageHandler] procedure DialogHandler(var T: TestPage \"RO Dialog\") var P: Codeunit \"RO Probe\"; begin P.SetSeen(T.DescrCtl.Value()); end;"
    ));
}

// handlers/TestPageActionRunObjectNoHandler_Tests.al:RunObjectActionWithoutAHandlerOpensItsTargetUnattended
#[test]
fn a_card_target_with_no_handler_opens_unattended_and_raises_nothing() {
    passes(&format!(
        "{SEED}
        [Test]
        procedure Unattended() var H: TestPage \"RO Host\"; P: Codeunit \"RO Probe\"; begin
            Seed(); H.OpenEdit(); H.First(); H.Next(); H.RunCard.Invoke();
            Check('open:B', P.Trace()); end;"
    ));
}

// handlers/TestPageActionRunObjectNoHandler_Tests.al:RunObjectActionOnADialogTargetWithoutAHandlerIsRefused / ...OpensItBeforeRefusing
#[test]
fn a_dialog_target_with_no_handler_opens_and_then_is_refused() {
    passes(&format!(
        "{SEED}
        [Test]
        procedure Refused() var H: TestPage \"RO Host\"; P: Codeunit \"RO Probe\"; begin
            Seed(); H.OpenEdit(); H.First(); H.Next();
            asserterror H.RunDialog.Invoke();
            if StrPos(GetLastErrorText(), 'TestPages that are invoked from a RunObject action must have a handler') = 0 then Error('got %1', GetLastErrorText());
            Check('dialog-open:B', P.Trace()); end;"
    ));
}

// handlers/TestPageActionRunObjectKinds_Tests.al:RunObjectNamingAReport/XmlPort/Query... (the refusals of the TestPage surface)
#[test]
fn report_xmlport_and_query_targets_are_refused_with_the_test_surface_texts() {
    passes(&format!(
        "{SEED}
        [Test]
        procedure Report() var H: TestPage \"RO Host\"; begin
            Seed(); H.OpenEdit(); H.First();
            asserterror H.RunReport.Invoke();
            Check('The method RunReport is not supported for TestPages.', GetLastErrorText()); end;
        [Test]
        procedure XmlPort() var H: TestPage \"RO Host\"; begin
            Seed(); H.OpenEdit(); H.First();
            asserterror H.RunXmlPort.Invoke();
            Check('The method RunXmlPort is not supported for TestPages.', GetLastErrorText()); end;
        [Test]
        procedure Query() var H: TestPage \"RO Host\"; begin
            Seed(); H.OpenEdit(); H.First();
            asserterror H.RunQuery.Invoke();
            Check('The method GetQueryTableMetadata is not supported for TestPages.', GetLastErrorText()); end;"
    ));
}

// handlers/TestPageActionRunObjectKinds_Tests.al:RunObjectNamingACodeunitRunsItOnTheHostsRow
// handlers/TestPageActionRunObjectCodeunitRec_Tests.al:CodeunitMovingItsRecMovesTheHost, CodeunitModifyingItsRecIsShownOnTheHost
#[test]
fn a_codeunit_target_runs_on_the_hosts_own_record() {
    passes(&format!(
        "{SEED}
        [Test]
        procedure Reads() var H: TestPage \"RO Host\"; P: Codeunit \"RO Probe\"; begin
            Seed(); H.OpenEdit(); H.First(); H.Next(); H.RunCodeunit.Invoke(); Check('Bravo', P.GetSeen()); end;
        [Test]
        procedure Moves() var H: TestPage \"RO Host\"; R: Record \"RO Row\"; begin
            Seed(); R.Get('B'); R.Grp := 'MOVE'; R.Modify(); Commit();
            H.OpenEdit(); H.First(); H.Next(); H.RunCodeunit.Invoke();
            Check('Charlie', H.DescrCtl.Value()); end;
        [Test]
        procedure Writes() var H: TestPage \"RO Host\"; R: Record \"RO Row\"; begin
            Seed(); R.Get('B'); R.Grp := 'WRITE'; R.Modify(); Commit();
            H.OpenEdit(); H.First(); H.Next(); H.RunCodeunit.Invoke();
            R.Get('B'); Check('Written', R.Descr); Check('Written', H.DescrCtl.Value()); end;"
    ));
}

// handlers/TestPageActionRunPageLink.al:ActionRunPageLink_Field_FiltersTargetToTheHostsCurrentRow, _Const_, _Filter_
// handlers/TestActionRunPageLinkFilterGroup.al:RunPageLink_FilterLandsInGroup0_NotTheLinkGroup
#[test]
fn run_page_link_filters_the_target_in_filter_group_zero() {
    passes(
        "local procedure SeedLines() var Hd: Record \"RO Head\"; L: Record \"RO Line\"; P: Codeunit \"RO Probe\"; begin
            P.Reset(); Hd.DeleteAll(); L.DeleteAll();
            Hd.\"No.\" := 'H1'; Hd.Insert(); Hd.\"No.\" := 'H2'; Hd.Insert(); Hd.\"No.\" := 'H3'; Hd.Insert();
            L.\"Head No.\" := 'H1'; L.\"Line No.\" := 1; L.Descr := 'H1-L1'; L.Insert();
            L.\"Head No.\" := 'H2'; L.\"Line No.\" := 1; L.Descr := 'H2-L1'; L.Insert();
            L.\"Head No.\" := 'H2'; L.\"Line No.\" := 2; L.Descr := 'H2-L2'; L.Insert();
            L.\"Head No.\" := 'H3'; L.\"Line No.\" := 1; L.Descr := 'H3-L1'; L.Insert();
            Commit(); end;
        procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
        local procedure Rows(var T: TestPage \"RO Lines\"): Text var S: Text; begin
            if T.First() then repeat S += T.DescrCtl.Value() + ';'; until not T.Next(); exit(S); end;
        [Test] [HandlerFunctions('LinesHandler')]
        procedure Field() var H: TestPage \"RO Head Card\"; P: Codeunit \"RO Probe\"; R: Record \"RO Head\"; begin
            SeedLines(); R.Get('H2'); H.OpenEdit(); H.GoToRecord(R); H.ByField.Invoke();
            Check('H2-L1;H2-L2;', P.GetSeen()); Check('g0=H2', P.GetGroups()); end;
        [Test] [HandlerFunctions('LinesHandler')]
        procedure Const() var H: TestPage \"RO Head Card\"; P: Codeunit \"RO Probe\"; R: Record \"RO Head\"; begin
            SeedLines(); R.Get('H2'); H.OpenEdit(); H.GoToRecord(R); H.ByConst.Invoke();
            Check('H1-L1;', P.GetSeen()); end;
        [Test] [HandlerFunctions('LinesHandler')]
        procedure FilterExpr() var H: TestPage \"RO Head Card\"; P: Codeunit \"RO Probe\"; R: Record \"RO Head\"; begin
            SeedLines(); R.Get('H2'); H.OpenEdit(); H.GoToRecord(R); H.ByFilter.Invoke();
            Check('H1-L1;H3-L1;', P.GetSeen()); end;
        [PageHandler] procedure LinesHandler(var T: TestPage \"RO Lines\") var P: Codeunit \"RO Probe\"; begin P.SetSeen(Rows(T)); end;",
    );
}

// handlers/TestPageActionRunPageView.al:RunPageView_SortingAndWhere_FiltersAndOrdersTheOpenedPage
#[test]
fn run_page_view_filters_in_group_three_and_orders_the_target() {
    passes(&format!(
        "{SEED}
        local procedure Rows(var T: TestPage \"RO Rank List\"): Text var S: Text; begin
            if T.First() then repeat S += T.NoCtl.Value(); until not T.Next(); exit(S); end;
        [Test] [HandlerFunctions('RankHandler')]
        procedure Ranked() var H: TestPage \"RO Host\"; P: Codeunit \"RO Probe\"; begin
            Seed(); H.OpenEdit(); H.First(); H.RunRanked.Invoke();
            Check('CA', P.GetSeen()); Check('g3=KEEP|key=Rank', P.GetGroups()); end;
        [PageHandler] procedure RankHandler(var T: TestPage \"RO Rank List\") var P: Codeunit \"RO Probe\"; begin P.SetSeen(Rows(T)); end;"
    ));
}

// handlers/TestPageActionRunPageMode.al:RunPageModeView_OpensTheCardReadOnly, RunPageModeCreate_OpensTheCardOnANewRecord,
// RunPageModeCreate_NoHandlerBound_Triggers, RunPageModeCreate_Trapped_Triggers
#[test]
fn run_page_mode_opens_the_target_read_only_or_on_a_new_record() {
    passes(&format!(
        "{SEED}
        [Test] [HandlerFunctions('ViewHandler')]
        procedure View() var H: TestPage \"RO Host\"; P: Codeunit \"RO Probe\"; begin
            Seed(); H.OpenEdit(); H.GoToKey('B'); H.RunCardView.Invoke(); Check('No', P.GetSeen()); end;
        [Test]
        procedure CreateUnattended() var H: TestPage \"RO Host\"; P: Codeunit \"RO Probe\"; R: Record \"RO Row\"; begin
            Seed(); H.OpenEdit(); H.GoToKey('B'); H.RunCardCreate.Invoke();
            Check('open:,new', P.Trace()); Check('3', Format(R.Count())); end;
        [Test]
        procedure CreateTrapped() var H: TestPage \"RO Host\"; C: TestPage \"RO Card\"; P: Codeunit \"RO Probe\"; begin
            Seed(); H.OpenEdit(); H.GoToKey('B'); C.Trap(); H.RunCardCreate.Invoke();
            Check('', C.NoCtl.Value()); C.Close(); Check('open:,new', P.Trace()); end;
        [PageHandler] procedure ViewHandler(var T: TestPage \"RO Card\") var P: Codeunit \"RO Probe\"; begin P.SetSeen(Format(T.Editable())); end;"
    ));
}

// handlers/TestPageActionRunObject_Tests.al:RunObjectActionOpensTheTargetOnTheHostsCurrentRow (RunPageOnRec absent opens the first row)
#[test]
fn without_run_page_on_rec_the_target_opens_on_its_first_row() {
    passes(&format!(
        "{SEED}
        [Test] [HandlerFunctions('CardHandler')]
        procedure Plain() var H: TestPage \"RO Host\"; P: Codeunit \"RO Probe\"; begin
            Seed(); H.OpenEdit(); H.First(); H.Next(); H.RunCardPlain.Invoke();
            Check('Alpha', P.GetSeen()); end;
        [PageHandler] procedure CardHandler(var T: TestPage \"RO Card\") var P: Codeunit \"RO Probe\"; begin P.SetSeen(T.DescrCtl.Value()); end;"
    ));
}

// handlers/TestPageActionRunObjectPageRec_Tests.al:HostOnAfterGetRecordAfterTheTargetWroteTheRow, TargetEditingTheRowAndTheHostsValue
#[test]
fn the_host_shows_the_row_a_page_target_wrote() {
    passes(&format!(
        "{SEED}
        [Test] [HandlerFunctions('WriteHandler')]
        procedure Written() var H: TestPage \"RO Host\"; begin
            Seed(); H.OpenEdit(); H.First(); H.Next(); H.RunCard.Invoke();
            Check('B', H.NoCtl.Value()); Check('Written', H.DescrCtl.Value()); end;
        [PageHandler] procedure WriteHandler(var T: TestPage \"RO Card\") begin T.DescrCtl.SetValue('Written'); T.Close(); end;"
    ));
}

const MOD_SEED: &str = "local procedure SeedMod() var R: Record \"RO Mod\"; P: Codeunit \"RO Probe\"; begin
    P.Reset(); R.DeleteAll();
    R.\"No.\" := 'A'; R.Descr := 'Alpha'; R.Grp := 'G1'; R.Insert();
    R.\"No.\" := 'B'; R.Descr := 'Bravo'; R.Grp := 'G2'; R.Insert();
    R.\"No.\" := 'C'; R.Descr := 'Charlie'; R.Grp := 'G1'; R.Insert();
    Commit();
end;
procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;";

// handlers/TestPageActionRunObjectPageStoredRow_Tests.al:OpenHostOnB (the precondition), TestPageActionRunObjectPageRec_Tests.al:HostOnAfterGetRecordValueAfterPlainRunObject
#[test]
fn a_list_row_keeps_what_on_after_get_record_put_into_rec() {
    passes(&format!(
        "{MOD_SEED}
        [Test]
        procedure First() var H: TestPage \"RO Mod Host\"; begin
            SeedMod(); H.OpenEdit(); H.First(); Check('CALC', H.GrpCtl.Value()); end;
        [Test]
        procedure Goto() var H: TestPage \"RO Mod Host\"; R: Record \"RO Mod\"; begin
            SeedMod(); H.OpenEdit(); R.Get('B'); H.GoToRecord(R); Check('CALC', H.GrpCtl.Value()); Check('B', H.NoCtl.Value()); end;
        [Test]
        procedure Next() var H: TestPage \"RO Mod Host\"; begin
            SeedMod(); H.OpenEdit(); H.First(); H.Next(); Check('CALC', H.GrpCtl.Value()); Check('B', H.NoCtl.Value()); end;"
    ));
}

// handlers/TestPageActionRunObjectPageStoredRow_Tests.al:TargetOnOpenPageSeesHostRow, TargetSaveDoesNotStoreHostsComputedValue
#[test]
fn the_target_opens_on_the_stored_row_not_on_what_the_hosts_page_code_left_in_rec() {
    passes(&format!(
        "{MOD_SEED}
        [Test] [HandlerFunctions('GrpHandler')]
        procedure Stored() var H: TestPage \"RO Mod Host\"; P: Codeunit \"RO Probe\"; R: Record \"RO Mod\"; begin
            SeedMod(); H.OpenEdit(); R.Get('B'); H.GoToRecord(R); Check('CALC', H.GrpCtl.Value());
            H.OpenTarget.Invoke(); Check('G2', P.GetGroups()); end;
        [PageHandler] procedure GrpHandler(var T: TestPage \"RO Mod Card\") var P: Codeunit \"RO Probe\"; begin P.SetGroups(T.NoCtl.Value() + T.DescrCtl.Value()); end;"
    ).replace("Check('G2', P.GetGroups())", "Check('BBravo', P.GetGroups())"));
}

// handlers/TestPageActionRunObjectPageRec_Tests.al:HostSaveAfterTheTargetEditedTheRowCarriesXRec
#[test]
fn the_host_saves_with_the_xrec_it_loaded_after_a_target_wrote_the_row() {
    passes(&format!(
        "{MOD_SEED}
        [Test] [HandlerFunctions('WriteHandler')]
        procedure XRec() var H: TestPage \"RO Mod Host\"; P: Codeunit \"RO Probe\"; R: Record \"RO Mod\"; begin
            SeedMod(); H.OpenEdit(); R.Get('B'); H.GoToRecord(R); H.OpenTarget.Invoke();
            Check('Bravo>Written', P.GetSeen());
            H.GrpCtl.SetValue('G9'); H.Next();
            Check('Bravo>Written', P.GetSeen()); R.Get('B'); Check('G9', R.Grp); Check('Written', R.Descr); end;
        [PageHandler] procedure WriteHandler(var T: TestPage \"RO Mod Card\") begin T.DescrCtl.SetValue('Written'); T.Close(); end;"
    ));
}

// handlers/TestPageActionRunPageMode.al: the host has actions named OpenView and OpenEdit, which are
// not the TestPage methods when a member of theirs is used
#[test]
fn an_action_named_like_a_test_page_method_is_the_control_when_a_member_follows() {
    passes(&format!(
        "{MOD_SEED}
        [Test] [HandlerFunctions('ViewHandler')]
        procedure Named() var H: TestPage \"RO Mod Host\"; P: Codeunit \"RO Probe\"; begin
            SeedMod(); H.OpenEdit(); H.First(); H.OpenView.Invoke(); Check('No', P.GetSeen()); end;
        [PageHandler] procedure ViewHandler(var T: TestPage \"RO Mod Card\") var P: Codeunit \"RO Probe\"; begin P.SetSeen(Format(T.Editable())); end;"
    ));
}

// handlers/TestPageControlEditableRoutes.al:Part_HostOpenEdit_ThenViewAction
#[test]
fn the_built_in_view_action_of_a_host_makes_its_parts_read_only() {
    passes(&format!(
        "{MOD_SEED}
        [Test]
        procedure PartsFollow() var H: TestPage \"RO Part Host\"; begin
            SeedMod(); H.OpenEdit(); H.First(); Check('Yes', Format(H.Lines.Editable()));
            H.View().Invoke(); Check('No', Format(H.Editable())); Check('No', Format(H.Lines.Editable())); end;"
    ));
}

fn not_run(tests: &str) -> Vec<(String, String)> {
    outcomes(tests)
        .into_iter()
        .map(|(n, o)| match o {
            Outcome::NotRun(why) => (n, why),
            other => panic!("{n}: {other:?}"),
        })
        .collect()
}

// Refused, never guessed (hard rule 4): what no corpus test pins.
#[test]
fn what_the_corpus_does_not_pin_is_refused_when_the_action_is_invoked() {
    let got = not_run(&format!(
        "{SEED}
        [Test] procedure OnRecWithSourceTableView() var H: TestPage \"RO Refused Host\"; begin Seed(); H.OpenEdit(); H.First(); H.OnRecWithView.Invoke(); end;
        [Test] procedure CreateOnRec() var H: TestPage \"RO Refused Host\"; begin Seed(); H.OpenEdit(); H.First(); H.CreateOnRec.Invoke(); end;
        [Test] procedure MissingTarget() var H: TestPage \"RO Refused Host\"; begin Seed(); H.OpenEdit(); H.First(); H.Missing.Invoke(); end;
        [Test] procedure UnknownMode() var H: TestPage \"RO Refused Host\"; begin Seed(); H.OpenEdit(); H.First(); H.Odd.Invoke(); end;"
    ));
    let why = |name: &str| got.iter().find(|(n, _)| n == name).unwrap().1.clone();
    assert!(why("OnRecWithSourceTableView").contains("SourceTableView"), "{got:?}");
    assert!(why("CreateOnRec").contains("RunPageMode = Create"), "{got:?}");
    assert!(why("MissingTarget").contains("outside the workspace"), "{got:?}");
    assert!(why("UnknownMode").contains("RunPageMode = strange"), "{got:?}");
}

// What `Rec` holds in the OnOpenPage of a page opened through `RunPageLink` is not pinned (only the filter groups are,
// handlers/TestActionRunPageLinkFilterGroup.al:119): a read of a field of Rec there refuses; reading the filters runs
// (run_page_link_filters_the_target_in_filter_group_zero). Probe queued in the corpus repo (issue 712).
#[test]
fn a_field_read_of_rec_in_onopenpage_of_a_run_page_link_target_is_refused() {
    let got = not_run(
        "[Test] [HandlerFunctions('Handler')] procedure Read() var H: Record \"RO Head\"; L: Record \"RO Line\"; Host: TestPage \"RO Reader Host\"; begin
            H.\"No.\" := 'H1'; H.Insert(); L.\"Head No.\" := 'H1'; L.\"Line No.\" := 1; L.Insert();
            Host.OpenEdit(); Host.GoToRecord(H); Host.Read.Invoke(); end;
        [PageHandler] procedure Handler(var P: TestPage \"RO Lines Reader\") begin P.Close(); end;",
    );
    assert!(got[0].1.contains("RunPageLink"), "{got:?}");
}

// Real BC fills the linked fields of Rec in that OnOpenPage, and GetFilters() of a copy of Rec reads '' (corpus issue 712, writer
// QZ). The runtime holds the link as a filter, so the reads of Rec's filters, key, view and rows refuse there. GetFilter of one
// field runs: the corpus pins it per filter group (handlers/TestActionRunPageLinkFilterGroup.al:119-131, baseline 60941).
#[test]
fn filter_key_and_view_reads_of_rec_in_onopenpage_of_a_run_page_link_target_are_refused() {
    let ops = ["GetFilters", "GetView", "CurrentKey", "HasFilter", "GetRangeMin", "Count", "IsEmpty", "FindFirst"];
    let mut tests = String::from(
        "[PageHandler] procedure Handler(var P: TestPage \"RO Lines Op Reader\") begin P.Close(); end;
        local procedure Read(Op: Text) var H: Record \"RO Head\"; L: Record \"RO Line\"; P: Codeunit \"RO Probe\"; Host: TestPage \"RO Reader Host\"; begin
            P.Reset(); H.DeleteAll(); L.DeleteAll(); P.SetSeen(Op);
            H.\"No.\" := 'H1'; H.Insert(); L.\"Head No.\" := 'H1'; L.\"Line No.\" := 1; L.Insert();
            Host.OpenEdit(); Host.GoToRecord(H); Host.ReadOp.Invoke(); end;",
    );
    for op in ops {
        tests += &format!("[Test] [HandlerFunctions('Handler')] procedure {op}x() begin Read('{op}'); end;");
    }
    let got = not_run(&tests);
    assert_eq!(got.len(), ops.len(), "{got:?}");
    for (n, why) in &got {
        assert!(why.contains("RunPageLink") && why.contains("712"), "{n}: {why}");
    }
    passes(
        "[PageHandler] procedure Handler(var P: TestPage \"RO Lines Op Reader\") begin P.Close(); end;
        [Test] [HandlerFunctions('Handler')] procedure GetFilterRuns() var H: Record \"RO Head\"; L: Record \"RO Line\"; P: Codeunit \"RO Probe\"; Host: TestPage \"RO Reader Host\"; begin
            P.Reset(); H.DeleteAll(); L.DeleteAll(); P.SetSeen('GetFilter');
            H.\"No.\" := 'H1'; H.Insert(); L.\"Head No.\" := 'H1'; L.\"Line No.\" := 1; L.Insert();
            Host.OpenEdit(); Host.GoToRecord(H); Host.ReadOp.Invoke();
            if P.GetGroups() <> 'H1' then Error('got <%1>', P.GetGroups()); end;",
    );
}

// An action that declares both runs only its RunObject
// (handlers/TestPageRunObjectContracts.al:371-390 RunObject_ActionWithBothRunObjectAndOnAction_OnlyTheTargetRuns).
#[test]
fn an_action_with_run_object_and_on_action_runs_only_the_target() {
    passes(&format!(
        "{SEED}
        [Test] [HandlerFunctions('CardH')] procedure Both() var H: TestPage \"RO Both Host\"; begin Seed(); H.OpenEdit(); H.First(); H.Both.Invoke(); end;
        [PageHandler] procedure CardH(var C: TestPage \"RO Card\") begin end;"
    ));
}

// A target with no handler is dropped without its close triggers
// (handlers/TestPageRunObjectContracts.al:RunObject_CardAndListTargets_NoHandler_OpenAndAreDroppedWithoutCloseTriggers).
#[test]
fn an_unattended_page_with_close_triggers_is_dropped_without_running_them() {
    passes(&format!(
        "{SEED}
        [Test] procedure ClosingTrigger() var H: TestPage \"RO Refused Host\"; begin Seed(); H.OpenEdit(); H.First(); H.Closing.Invoke(); end;"
    ));
}
