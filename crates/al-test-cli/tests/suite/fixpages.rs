//! Page, part, handler and report contracts pinned by probe tests of the corpus. AL source -> compile -> run as a test codeunit. Each test names the probe it
//! mirrors.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 64000 "FP Row"
{
    fields { field(1; "No."; Integer) { } field(2; Link; Integer) { } field(3; Txt; Text[30]) { } }
    keys { key(PK; "No.") { } }
}
codeunit 64001 "FP Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(E: Text) begin L += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end;
}
page 64010 "FP Part"
{
    PageType = ListPart; SourceTable = "FP Row";
    layout { area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } field(TxtCtl; Rec.Txt) { } } } }
    var Log: Codeunit "FP Log";
    trigger OnInit() begin Log.Add('part.OnInit'); end;
    trigger OnOpenPage() begin Log.Add('part.OnOpenPage'); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin Log.Add('part.Query(' + Format(CloseAction) + ')'); exit(true); end;
    trigger OnClosePage() begin Log.Add('part.Close'); end;
}
page 64011 "FP Host"
{
    PageType = Card; SourceTable = "FP Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { }
        part(Lines; "FP Part") { SubPageLink = Link = field("No."); }
        part(ReadOnly; "FP Part") { Editable = false; SubPageLink = Link = field("No."); } } }
    var Log: Codeunit "FP Log";
    trigger OnInit() begin Log.Add('host.OnInit'); end;
    trigger OnOpenPage() begin Log.Add('host.OnOpenPage'); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin Log.Add('host.Query'); exit(true); end;
    trigger OnClosePage() begin Log.Add('host.Close'); end;
}
page 64012 "FP Dialog"
{
    PageType = StandardDialog;
}
page 64017 "FP Row Dialog"
{
    PageType = StandardDialog; SourceTable = "FP Row";
    layout { area(Content) { field(TxtCtl; Rec.Txt) { } } }
}
page 64013 "FP Hidden Host"
{
    PageType = Card; SourceTable = "FP Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } part(Hidden; "FP Part") { Visible = false; } } }
}
page 64014 "FP Actions"
{
    PageType = Card; SourceTable = "FP Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
    actions { area(Processing) {
        action(WithTrigger) { trigger OnAction() begin end; }
        action(NoTrigger) { }
        action(WithRunObject) { RunObject = page "FP Dialog"; } } }
}
table 64002 "FP Clear"
{
    fields { field(1; Id; Integer) { } field(2; Txt; Text[20]) { } }
    keys { key(PK; Id) { } }
    var TableGlobal: Integer;
    procedure ClearInTable(): Text begin TableGlobal := 7; Id := 3; Txt := 'tbl'; ClearAll(); exit('global=' + Format(TableGlobal) + ' Id=' + Format(Id) + ' Txt=[' + Txt + ']'); end;
}
codeunit 64003 "FP Clear No"
{
    TableNo = "FP Clear";
    var Glob: Integer; Glob2: Text;
    trigger OnRun() var Log: Codeunit "FP Log"; begin
        Glob := 5; Glob2 := 'g'; Rec.Txt := 'before'; ClearAll();
        Log.Add('glob=' + Format(Glob) + ' glob2=[' + Glob2 + '] Rec.Id=' + Format(Rec.Id) + ' Rec.Txt=[' + Rec.Txt + ']'); end;
}
page 64015 "FP Clear Page"
{
    PageType = Card; SourceTable = "FP Clear";
    layout { area(Content) { field(IdCtl; Rec.Id) { } field(TxtCtl; Rec.Txt) { } } }
    actions { area(Processing) { action(ClearIt) { trigger OnAction() var Log: Codeunit "FP Log"; begin
        PageGlobal := 9; ClearAll(); Log.Add('glob=' + Format(PageGlobal) + ' Rec.Id=' + Format(Rec.Id) + ' Rec.Txt=[' + Rec.Txt + ']'); end; } } }
    var PageGlobal: Integer;
}
page 64016 "FP Plain List"
{
    PageType = List; SourceTable = "FP Row";
    layout { area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } } }
    actions { area(Processing) { action(SelectionFilter) { trigger OnAction() var Selected: Record "FP Row"; Log: Codeunit "FP Log"; begin
        CurrPage.SetSelectionFilter(Selected); Log.Add('filters=[' + Selected.GetFilters() + '] count=' + Format(Selected.Count())); end; } } }
}
report 64021 "FP Captioned" { Caption = 'FP Caption'; ProcessingOnly = true; UseRequestPage = false; }
report 64020 "FP Report"
{
    ProcessingOnly = true; UseRequestPage = false;
    dataset
    {
        dataitem(Parent; "FP Row")
        {
            DataItemTableView = sorting("No.") where(Link = const(0));
            dataitem(Child; "FP Row")
            {
                DataItemLink = Link = field("No.");
                trigger OnAfterGetRecord() begin Log.Add('child(' + Format(Child."No.") + ',' + Child.GetFilters() + ')'); end;
            }
            trigger OnAfterGetRecord()
            begin
                Log.Add('parent(' + Format(Parent."No.") + ')');
                if (Mode = 'skip') and (Parent."No." = 1) then CurrReport.Skip();
                if (Mode = 'break') and (Parent."No." = 2) then CurrReport.Break();
                if (Mode = 'quit') and (Parent."No." = 2) then CurrReport.Quit();
            end;
            trigger OnPostDataItem() begin Log.Add('post-item'); end;
        }
    }
    var Log: Codeunit "FP Log"; Mode: Text;
    procedure SetMode(M: Text) begin Mode := M; end;
    trigger OnPreReport() begin Parent.SetRange("No.", 2); Log.Add('pre'); end;
    trigger OnPostReport() begin Log.Add('post-report'); end;
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 64100 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    for (n, r) in &outcomes(tests) {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

const HELPERS: &str = "local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;
local procedure Seed() var R: Record \"FP Row\"; Log: Codeunit \"FP Log\"; begin R.DeleteAll();
  R.\"No.\" := 1; R.Insert(); R.\"No.\" := 2; R.Insert(); R.\"No.\" := 3; R.Insert();
  R.\"No.\" := 11; R.Link := 1; R.Insert(); R.\"No.\" := 12; R.Link := 1; R.Insert(); R.\"No.\" := 31; R.Link := 3; R.Insert();
  Commit(); Log.Take(); end;";

// handlers/TestUnhandledUiAndDialogDefaults.al:68-107 (one space, MissingUIHandler, page ids)
#[test]
fn unhandled_ui_text_and_code() {
    passes(&format!(
        "{HELPERS}
        [Test] procedure Texts() var D: Page \"FP Dialog\"; begin
          asserterror Message('hello %1', 5);
          Check('Unhandled UI: Message hello 5', GetLastErrorText()); Check('MissingUIHandler', GetLastErrorCode());
          asserterror Page.RunModal(Page::\"FP Dialog\");
          Check('Unhandled UI: ModalPage 64012', GetLastErrorText());
          asserterror Page.Run(Page::\"FP Dialog\");
          Check('Unhandled UI: Page 64012', GetLastErrorText()); end;
        [Test] [HandlerFunctions('Boom')] procedure InHandler() begin
          asserterror Message('x'); Check('boom', GetLastErrorText()); Check('TestWrapped:Dialog', GetLastErrorCode()); end;
        [MessageHandler] procedure Boom(M: Text) begin Error('boom'); end;"
    ));
}

// testpart/TestPartTriggerOrderAndRefresh.al:236-247 and the close order of
// testpart/TestPartCloseLinkEditableContracts.al
#[test]
fn part_init_and_close_order() {
    passes(&format!(
        "{HELPERS}
        [Test] procedure Order() var H: TestPage \"FP Host\"; Log: Codeunit \"FP Log\"; begin Seed();
          H.OpenEdit(); Check('host.OnInit;part.OnInit;part.OnInit;host.OnOpenPage;part.OnOpenPage;part.OnOpenPage;', Log.Take());
          H.Close();
          Check('part.Query(None);part.Query(None);host.Query;part.Close;part.Close;host.Close;', Log.Take()); end;"
    ));
}

// testpart/TestPartTriggerOrderAndRefresh.al:316-340 and handlers/TestPageRuntimeContracts.al:245-278
#[test]
fn not_found_ids_and_codes() {
    passes(&format!(
        "{HELPERS}
        [Test] procedure Ids() var H: TestPage \"FP Hidden Host\"; B: Boolean; begin Seed(); H.OpenEdit();
          asserterror B := H.Hidden.Visible();
          Check('The part with ID = 1732295561 was not found on the page.', GetLastErrorText());
          Check('TestPartNotFound', GetLastErrorCode());
          asserterror H.GetField(99999).Value();
          Check('The field with ID = 99999 is not found on the page.', GetLastErrorText());
          Check('TestFieldNotFound', GetLastErrorCode());
          asserterror H.GetValidationError(0);
          Check('Index out of bounds.', GetLastErrorText()); Check('IndexOutOfBounds', GetLastErrorCode()); end;"
    ));
}

// handlers/TestPageLabelControlContracts.al:131-145 (BC 27.5): an action with no OnAction trigger and no RunObject is
// not on the TestPage: naming it raises "The action with ID = <n> is not found on the page." / TestActionNotFound.
#[test]
fn an_action_without_behaviour_is_not_on_the_test_page() {
    passes(&format!(
        "{HELPERS}
        [Test] procedure Actions() var H: TestPage \"FP Actions\"; B: Boolean; begin Seed(); H.OpenView();
          B := H.WithTrigger.Visible(); B := H.WithRunObject.Visible();
          asserterror B := H.NoTrigger.Visible();
          if not GetLastErrorText().StartsWith('The action with ID = ') then Error('start %1', GetLastErrorText());
          if not GetLastErrorText().EndsWith(' is not found on the page.') then Error('end %1', GetLastErrorText());
          Check('TestActionNotFound', GetLastErrorCode()); end;"
    ));
}

// session/TestClearAllTransactionTypeContracts.al:224-285 (BC 27.5): ClearAll resets the globals of a codeunit with a
// TableNo, of a table and of a page, and keeps their record (Rec, the fields).
#[test]
fn clear_all_resets_the_globals_and_keeps_the_record() {
    passes(&format!(
        "{HELPERS}
        [Test] procedure CodeunitWithTableNo() var R: Record \"FP Clear\"; C: Codeunit \"FP Clear No\"; Log: Codeunit \"FP Log\"; begin
          R.DeleteAll(); R.Id := 1; R.Txt := 'a'; R.Insert(); Commit(); Log.Take(); R.Get(1); C.Run(R);
          Check('glob=0 glob2=[] Rec.Id=1 Rec.Txt=[before];', Log.Take()); Check('before', R.Txt); end;
        [Test] procedure Table() var R: Record \"FP Clear\"; begin
          Check('global=0 Id=3 Txt=[tbl]', R.ClearInTable()); end;
        [Test] procedure Page() var R: Record \"FP Clear\"; P: TestPage \"FP Clear Page\"; Log: Codeunit \"FP Log\"; begin
          R.DeleteAll(); R.Id := 1; R.Txt := 'a'; R.Insert(); Commit(); Log.Take(); P.OpenView(); P.ClearIt.Invoke();
          Check('glob=0 Rec.Id=1 Rec.Txt=[a];', Log.Take()); end;"
    ));
}

// handlers/TestPageCloseTriggersSelectionCardPageContracts.al:250-285 (BC 27.5): Page.GetRecord after RunModal gives the
// row the page was positioned on when it closed: the picked row after OK, the last positioned row after Cancel.
#[test]
fn get_record_is_the_row_the_page_closed_on() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('PickTwo')] procedure AfterOk() var P: Page \"FP Plain List\"; Picked: Record \"FP Row\"; A: Action; begin Seed();
          P.LookupMode(true); A := P.RunModal(); P.GetRecord(Picked);
          Check('LookupOK', Format(A)); Check('2', Format(Picked.\"No.\")); Check('', Picked.GetFilters()); end;
        [Test] [HandlerFunctions('CancelOnThree')] procedure AfterCancel() var P: Page \"FP Plain List\"; Picked: Record \"FP Row\"; A: Action; begin Seed();
          P.LookupMode(true); A := P.RunModal(); P.GetRecord(Picked);
          Check('LookupCancel', Format(A)); Check('3', Format(Picked.\"No.\")); end;
        [ModalPageHandler] procedure PickTwo(var L: TestPage \"FP Plain List\") begin L.GoToKey(2); L.OK().Invoke(); end;
        [ModalPageHandler] procedure CancelOnThree(var L: TestPage \"FP Plain List\") begin L.GoToKey(3); L.Cancel().Invoke(); end;"
    ));
}

// handlers/TestPageCloseTriggersSelectionCardPageContracts.al:239-250: SetSelectionFilter on a list positioned on a row
// filters the record to that row.
#[test]
fn set_selection_filter_filters_to_the_row_the_list_stands_on() {
    passes(&format!(
        "{HELPERS}
        [Test] procedure Selection() var L: TestPage \"FP Plain List\"; Log: Codeunit \"FP Log\"; begin Seed();
          L.OpenView(); L.GoToKey(2); L.SelectionFilter.Invoke();
          Check('filters=[No.: 2] count=1;', Log.Take()); L.Close(); end;"
    ));
}

// handlers/PWRefusedBuiltins.al:78-107 (BC 27.5): Report.ObjectId(false) is `Report <id>`, ObjectId(true) and ObjectId()
// `Report <Caption>`, the report's name when it has no Caption.
#[test]
fn report_object_id_reads_the_id_or_the_caption() {
    passes(&format!(
        "{HELPERS}
        [Test] procedure ObjectId() var A: Report \"FP Captioned\"; B: Report \"FP Report\"; begin
          Check('Report 64021', A.ObjectId(false)); Check('Report FP Caption', A.ObjectId(true)); Check('Report FP Caption', A.ObjectId());
          Check('Report 64020', B.ObjectId(false)); Check('Report FP Report', B.ObjectId(true)); end;"
    ));
}

// handlers/TestReportRuntimeContracts.al:322-390 (Skip, Break, Quit, discarded OnPreReport filter,
// DataItemLink hidden from GetFilters)
#[test]
fn report_skip_break_quit_and_filters() {
    passes(&format!(
        "{HELPERS}
        [Test] procedure Flow() var Rep: Report \"FP Report\"; Log: Codeunit \"FP Log\"; begin Seed();
          Rep.Run();
          Check('pre;parent(1);child(11,);child(12,);parent(2);parent(3);child(31,);post-item;post-report;', Log.Take());
          Clear(Rep); Rep.SetMode('skip'); Rep.Run();
          Check('pre;parent(1);parent(2);parent(3);child(31,);post-item;post-report;', Log.Take());
          Clear(Rep); Rep.SetMode('break'); Rep.Run();
          Check('pre;parent(1);child(11,);child(12,);parent(2);post-item;post-report;', Log.Take());
          Clear(Rep); Rep.SetMode('quit'); Rep.Run();
          Check('pre;parent(1);child(11,);child(12,);parent(2);', Log.Take()); end;"
    ));
}

// handlers/TestPageModalResultsAndTriggersContracts.al:
// RunModal_Dialog_StaticVersusVariable_ReportedAction
#[test]
fn static_run_modal_reports_the_lookup_actions() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Ok')] procedure Static() var R: Action; begin
          R := Page.RunModal(Page::\"FP Dialog\"); Check('LookupOK', Format(R)); end;
        [ModalPageHandler] procedure Ok(var D: TestPage \"FP Dialog\") begin D.OK().Invoke(); end;"
    ));
}

// testpart/TestPartCloseLinkEditableContracts.al:
// Parts_EnabledReadsTheControl_EditableFalseDoesNotBlockWrites
#[test]
fn editable_false_part_still_takes_writes() {
    passes(&format!(
        "{HELPERS}
        [Test] procedure Write() var H: TestPage \"FP Host\"; R: Record \"FP Row\"; begin Seed();
          H.OpenEdit(); H.GoToKey(1); H.ReadOnly.First(); H.ReadOnly.TxtCtl.SetValue('ro'); H.ReadOnly.Next();
          R.Get(11); Check('ro', R.Txt); H.Close(); end;"
    ));
}

// Page.RunModal(Id, Rec, Field) with a handler that does not answer LookupOK (Segment Line.RunCreateInteraction, Tests-Marketing, 33 tests):
// the field is kept (assumption `runmodal-field-kept-unless-lookupok`); the unhandled case is pinned by errors/lt-last-error-and-events/P70_06RunModalField.al.
#[test]
fn run_modal_with_a_field_argument_and_a_handler_that_does_not_answer_lookup_ok() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Cancel')] procedure Cancelled() var R: Record \"FP Row\"; A: Action; begin
          R.Txt := 'kept'; A := Page.RunModal(Page::\"FP Row Dialog\", R, R.Txt); Check('LookupCancel', Format(A)); Check('kept', R.Txt); end;
        [Test] [HandlerFunctions('Typed')] procedure TypedHandler() var R: Record \"FP Row\"; A: Action; D: Page \"FP Row Dialog\"; begin
          R.Txt := 'kept'; A := D.RunModal(); Check('kept', R.Txt); end;
        [ModalPageHandler] procedure Cancel(var D: TestPage \"FP Row Dialog\") begin D.Cancel().Invoke(); end;
        [ModalPageHandler] procedure Typed(var D: Page \"FP Row Dialog\"; var Response: Action) begin end;"
    ));
}

// With a TestPage handler that answers OK the field argument changes nothing (pages/lt-page-run-overloads-testpart/
// P106RunStatic.al:141-866; al-runner issue 5753).
#[test]
fn run_modal_with_a_field_argument_and_lookup_ok_runs() {
    let r = outcomes(
        "[Test] [HandlerFunctions('Ok')] procedure Accepted() var R: Record \"FP Row\"; A: Action; begin
          R.Txt := 'x'; A := Page.RunModal(Page::\"FP Row Dialog\", R, R.Txt); end;
        [ModalPageHandler] procedure Ok(var D: TestPage \"FP Row Dialog\") begin D.OK().Invoke(); end;",
    );
    assert!(r.iter().all(|(_, o)| matches!(o, Outcome::Pass)), "{r:?}");
}
