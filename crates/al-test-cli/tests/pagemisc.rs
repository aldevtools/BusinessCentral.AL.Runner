//! Smaller gaps of the headless test client (ADR-012 follow-up): procedures on a `Page`
//! variable, `Clear` of a TestPage, and further. AL source → compile → run as a test codeunit.
//! Each test names the corpus test it mirrors.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62000 "PM Row"
{
    fields { field(1; "No."; Code[20]) { } field(2; Descr; Text[50]) { } field(3; Rank; Integer) { } }
    keys { key(PK; "No.") { } }
    procedure PreviousNo(): Code[20] begin exit(xRec."No."); end;
}
codeunit 62002 "PM Probe"
{
    SingleInstance = true;
    var Trail: Text;
    procedure Reset() begin Trail := ''; end;
    procedure Note(T: Text) begin Trail += T; end;
    procedure Trace(): Text begin exit(Trail); end;
}
page 62010 "PM Wizard"
{
    PageType = Card;
    layout { area(Content) { field(StepCtl; Step) { } } }
    var Step: Integer;
    trigger OnInit() var P: Codeunit "PM Probe"; begin P.Note('I'); Step := 0; end;
    trigger OnOpenPage() var P: Codeunit "PM Probe"; begin P.Note('O'); end;
    procedure SetStep(S: Integer) var P: Codeunit "PM Probe"; begin P.Note('S'); Step := S; end;
    procedure GetStep(): Integer begin exit(Step); end;
}
page 62019 "PM Second"
{
    PageType = Card;
    layout { area(Content) { field(Label2; Txt) { } } }
    var Txt: Text[10];
    trigger OnOpenPage() begin Txt := 'second'; end;
}
page 62011 "PM Persist"
{
    PageType = Worksheet;
    layout { area(Content) { field(Dummy; Dummy) { } } }
    var Target: Record "PM Row"; Dummy: Text[10];
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        if (CloseAction <> CloseAction::OK) and (CloseAction <> CloseAction::LookupOK) then exit(true);
        Target.Rank := 0; Target.Modify(); exit(true);
    end;
    procedure SetTarget(var Row: Record "PM Row") begin Target := Row; end;
}
table 62090 "PM Opt Row"
{
    fields { field(1; Id; Integer) { } field(2; Opt; Option) { OptionMembers = " ",A,B; OptionCaption = ' ,A,B'; } }
    keys { key(PK; Id) { } }
}
page 62092 "PM Opt Card"
{
    PageType = Card; SourceTable = "PM Opt Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(Opt; Rec.Opt) { } } }
}
page 62012 "PM Card"
{
    PageType = Card; SourceTable = "PM Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } field(DescrCtl; Rec.Descr) { } } }
}

table 62001 "PM Look"
{
    fields {
        field(1; "No."; Code[20]) { }
        field(2; "Table Only"; Code[20]) { trigger OnLookup() begin "Table Only" := 'FROM-TABLE'; end; }
        field(3; Both; Code[20]) { trigger OnLookup() begin Both := 'FROM-TABLE'; end; }
        field(4; Related; Code[20]) { TableRelation = "PM Related"; }
        field(5; Plain; Code[20]) { }
        field(6; Pageless; Code[20]) { TableRelation = "PM Pageless"; }
    }
    keys { key(PK; "No.") { } }
}
table 62003 "PM Related" { LookupPageId = "PM Related List"; fields { field(1; "Code"; Code[20]) { } field(2; Descr; Text[50]) { } } keys { key(PK; "Code") { } } }
table 62004 "PM Pageless" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { } } }
page 62014 "PM Related List"
{
    PageType = List; SourceTable = "PM Related";
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } field(Descr; Rec.Descr) { } } } }
}
page 62015 "PM Look Card"
{
    PageType = Card; SourceTable = "PM Look";
    layout { area(Content) {
        field("No."; Rec."No.") { }
        field("Table Only"; Rec."Table Only") { }
        field(Both; Rec.Both) { trigger OnLookup(var Text: Text): Boolean begin Text := 'FROM-CONTROL'; exit(true); end; }
        field(Related; Rec.Related) { }
        field(Plain; Rec.Plain) { }
        field(Pageless; Rec.Pageless) { }
    } }
}

codeunit 62020 "PM Worker"
{
    trigger OnRun() var P: Dictionary of [Text, Text]; R: Dictionary of [Text, Text]; N: Text; V: Text; Row: Record "PM Row"; begin
        P := Page.GetBackgroundParameters(); P.Get('No', N);
        if N.StartsWith('FAIL') then Error('worker failed for %1', N);
        if N = 'WRITE' then begin Row."No." := 'W'; Row.Insert(); end;
        if N = 'COMMIT' then Commit();
        if N = 'ISOSET' then IsolatedStorage.Set('PMbg', 'x');
        if N = 'ISODEL' then IsolatedStorage.Delete('PMbg');
        if N = 'ISOGET' then begin R.Add('Ret', Format(IsolatedStorage.Get('PMbg', V), 0, 9)); R.Add('Value', V); end;
        R.Add('Out', 'BG:' + N); Page.SetBackgroundTaskResult(R);
    end;
}
page 62021 "PM Bg Card"
{
    PageType = Card; SourceTable = "PM Row";
    layout { area(Content) { field("No."; Rec."No.") { } field(OutCtl; OutText) { } field(ErrCtl; ErrText) { } } }
    trigger OnAfterGetCurrRecord() var A: Dictionary of [Text, Text]; begin A.Add('No', Rec."No."); CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"PM Worker", A); end;
    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text]) begin Results.Get('Out', OutText); end;
    trigger OnPageBackgroundTaskError(TaskId: Integer; ErrorCode: Text; ErrorText: Text; ErrorCallStack: Text; var IsHandled: Boolean) begin ErrText := ErrorText; IsHandled := Rec.Descr = 'handled'; end;
    var TaskId: Integer; OutText: Text; ErrText: Text;
}

page 62013 "PM Rowset"
{
    PageType = List; SourceTable = "PM Row"; SourceTableView = sorting("No.") order(descending); Editable = false;
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } field(Descr; Rec.Descr) { } } } }
    actions { area(Processing) {
        action(FillEveryOther) { trigger OnAction() var Src: Record "PM Row"; Take: Boolean; begin
            TempBuf.Reset(); TempBuf.DeleteAll(); Src.SetCurrentKey("No."); Src.Ascending(true); Take := true;
            if Src.FindSet() then repeat if Take then begin TempBuf := Src; TempBuf.Insert(); end; Take := not Take; until Src.Next() = 0;
            Activate(); end; }
        action(FillAll) { trigger OnAction() var Src: Record "PM Row"; begin
            TempBuf.Reset(); TempBuf.DeleteAll();
            if Src.FindSet() then repeat TempBuf := Src; TempBuf.Insert(); until Src.Next() = 0;
            Activate(); end; }
    } }
    var TempBuf: Record "PM Row" temporary; RunOnTemp: Boolean;
    local procedure Activate() begin RunOnTemp := true; Rec.FilterGroup(0); Rec.SetRange("No."); CurrPage.Update(false); end;
    trigger OnFindRecord(Which: Text): Boolean
    var P: Codeunit "PM Probe"; Found: Boolean;
    begin
        P.Note('F');
        if not RunOnTemp then exit(Rec.Find(Which));
        TempBuf.Copy(Rec); Found := TempBuf.Find(Which); if Found then Rec := TempBuf; exit(Found);
    end;
    trigger OnNextRecord(Steps: Integer): Integer
    var P: Codeunit "PM Probe"; Moved: Integer;
    begin
        P.Note('N');
        if not RunOnTemp then exit(Rec.Next(Steps));
        TempBuf.Copy(Rec); Moved := TempBuf.Next(Steps); if Moved <> 0 then Rec := TempBuf; exit(Moved);
    end;
}
page 62090 "PM Dialog" { PageType = StandardDialog; }
page 62096 "PM Close List"
{
    PageType = List; SourceTable = "PM Row";
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var P: Codeunit "PM Probe"; begin P.Note('List ' + Format(CloseAction) + ';'); exit(true); end;
}
page 62099 "PM Event Card"
{
    PageType = Card; SourceTable = "PM Row";
    layout { area(Content) { field("No."; Rec."No.") { } field(Descr; Rec.Descr) { trigger OnValidate() var P: Codeunit "PM Probe"; begin P.Note('V;'); end; } } }
}
codeunit 62097 "PM Event Subs"
{
    [EventSubscriber(ObjectType::Page, Page::"PM Event Card", 'OnBeforeValidateEvent', 'Descr', false, false)]
    local procedure Before(var Rec: Record "PM Row") var P: Codeunit "PM Probe"; begin P.Note('B(' + Rec.Descr + ');'); end;
    [EventSubscriber(ObjectType::Page, Page::"PM Event Card", 'OnAfterValidateEvent', 'Descr', false, false)]
    local procedure After(var Rec: Record "PM Row") var P: Codeunit "PM Probe"; begin P.Note('A(' + Rec.Descr + ');'); end;
}
page 62191 "PM SetRec List"
{
    PageType = List; SourceTable = "PM Row";
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } field(Descr; Rec.Descr) { } } } }
    trigger OnOpenPage() var P: Codeunit "PM Probe"; begin P.Note('open=' + Rec."No." + ';'); end;
}
table 62300 "PM Host" { fields { field(1; "No."; Code[20]) { } field(2; Name; Text[30]) { } } keys { key(PK; "No.") { } } }
table 62301 "PM Host Line" { fields { field(1; "Host No."; Code[20]) { TableRelation = "PM Host"; } field(2; "Line No."; Integer) { } field(3; Info; Text[30]) { } } keys { key(PK; "Host No.", "Line No.") { } } }
page 62302 "PM Host Lines" { PageType = ListPart; SourceTable = "PM Host Line"; AutoSplitKey = true; DelayedInsert = true;
  layout { area(Content) { repeater(R) { field(Info; Rec.Info) { } } } } }
page 62303 "PM Host Card" { PageType = ListPlus; SourceTable = "PM Host";
  layout { area(Content) { group(G) { field("No."; Rec."No.") { } field(Name; Rec.Name) { } }
    part(Lines; "PM Host Lines") { SubPageLink = "Host No." = field("No."); } } } }
table 62340 "PM SH" { fields { field(0; "Document Type"; Option) { OptionMembers = Quote,"Order"; } field(1; "No."; Code[20]) { } field(2; "Customer No."; Code[20]) { } }
  keys { key(PK; "Document Type", "No.") { } }
  trigger OnInsert() begin if "No." = '' then "No." := 'SVO001'; end; }
table 62341 "PM SIL" { fields {
    field(1; "Document No."; Code[20]) { TableRelation = "PM SH"."No." where("Document Type" = field("Document Type")); }
    field(2; "Line No."; Integer) { } field(3; Info; Text[30]) { }
    field(43; "Document Type"; Option) { OptionMembers = Quote,"Order"; } }
  keys { key(PK; "Document Type", "Document No.", "Line No.") { } } }
page 62342 "PM SIL Part" { PageType = ListPart; SourceTable = "PM SIL"; SourceTableView = where("Document Type" = const("Order")); AutoSplitKey = true; DelayedInsert = true;
  layout { area(Content) { repeater(R) { field(Info; Rec.Info) { } } } } }
page 62343 "PM SH Card" { PageType = Document; SourceTable = "PM SH"; SourceTableView = where("Document Type" = filter("Order"));
  layout { area(Content) { group(G) { field("No."; Rec."No.") { } field("Customer No."; Rec."Customer No.") { } }
    part(Lines; "PM SIL Part") { SubPageLink = "Document No." = field("No."); } } }
  trigger OnNewRecord(BelowxRec: Boolean) begin Rec."Document Type" := Rec."Document Type"::"Order"; end; }
page 62344 "PM OpenNew Find Card" { PageType = Card; SourceTable = "PM Row";
  layout { area(Content) { field("No."; Rec."No.") { } } }
  trigger OnFindRecord(Which: Text): Boolean begin if Rec.Find(Which) then exit(true); Rec.SetRange("No."); exit(Rec.Find(Which)); end; }
table 62304 "PM Pick" { LookupPageId = "PM Pick List"; fields { field(1; Id; Code[10]) { } field(2; Grp; Integer) { } } keys { key(PK; Id) { } } }
page 62305 "PM Pick List" { PageType = List; SourceTable = "PM Pick"; Editable = false;
  layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } } }
table 62306 "PM Cat" { fields { field(1; Item; Code[10]) { } field(2; Vend; Code[10]) { } field(3; Info; Code[10]) { } } keys { key(PK; Item, Vend) { } } }
page 62307 "PM Cat List" { PageType = List; SourceTable = "PM Cat";
  layout { area(Content) { repeater(R) { field(Item; Rec.Item) { } field(Vend; Rec.Vend) { } field(Info; Rec.Info) { } } } } }
page 62308 "PM Selection Own"
{
    PageType = Worksheet; SourceTable = "PM Row";
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } } } }
    actions { area(Processing) { action(Pick) { trigger OnAction() var S: Record "PM Row"; begin CurrPage.SetSelectionFilter(S); if S.FindFirst() then; end; } } }
    procedure SetSelectionFilter() begin end;
}
page 62320 "PM Hidden Part" { PageType = ListPart; SourceTable = "PM Row";
  layout { area(Content) { repeater(R) { field("No."; Rec."No.") { } } } }
  procedure Mark() begin end;
  procedure Ping(Key: Code[20]) begin Rec.FilterGroup(2); Rec.SetRange("No.", Key); Rec.FilterGroup(0); if Rec.FindLast() then; CurrPage.Update(false); end; }
page 62321 "PM Hidden Host" { PageType = Card; SourceTable = "PM Row";
  layout { area(Content) { field("No."; Rec."No.") { } part(Hid; "PM Hidden Part") { Visible = false; } } }
  trigger OnAfterGetCurrRecord() var P: Codeunit "PM Probe"; begin CurrPage.Hid.PAGE.Ping(Rec."No."); P.Note('host;'); end; }
page 62323 "PM Find List" { PageType = List; SourceTable = "PM Row";
  layout { area(Content) { repeater(R) { field("No."; Rec."No.") { } } } }
  actions { area(Processing) { action(Refresh) { trigger OnAction() begin CurrPage.Update(false); end; } } }
  trigger OnFindRecord(Which: Text): Boolean var P: Codeunit "PM Probe"; begin P.Note('F' + Which + ';'); exit(Rec.Find(Which)); end;
  trigger OnAfterGetRecord() var P: Codeunit "PM Probe"; begin P.Note('R' + Rec."No." + ';'); end; }
page 62324 "PM Caption Before" { PageType = Card; SourceTable = "PM Row"; Caption = 'Static Caption'; layout { area(Content) { field("No."; Rec."No.") { } } }
  procedure Read(): Text begin exit(CurrPage.Caption); end; }
page 62326 "PM Never Ran Host" { PageType = Card; SourceTable = "PM Row"; layout { area(Content) { field("No."; Rec."No.") { } part(Sub; "PM Hidden Part") { } } }
  procedure Reach(): Text begin CurrPage.Sub.PAGE.Mark(); exit('reached'); end; }
page 62325 "PM Empty Class" { PageType = Card; SourceTable = "PM Row"; layout { area(Content) { field("No."; Rec."No.") { CaptionClass = '3,'; } } } }
page 62322 "PM Selection Own Update" { PageType = Card; SourceTable = "PM Row"; layout { area(Content) { field("No."; Rec."No.") { } } }
  procedure UseAfterClose() begin CurrPage.Update(true); end; }
page 62327 "PM Editable Flag" { PageType = Card; SourceTable = "PM Row";
  layout { area(Content) { field("No."; Rec."No.") { } field(Descr; Rec.Descr) { trigger OnValidate() begin Flag := Rec.Descr = ''; end; } field(Other; Rec.Rank) { Editable = not Flag; } } }
  var Flag: Boolean; }
page 62328 "PM Xrec Card" { PageType = Card; SourceTable = "PM Row"; layout { area(Content) { field("No."; Rec."No.") { } } }
  trigger OnNewRecord(BelowxRec: Boolean) var P: Codeunit "PM Probe"; begin P.Note('x=' + Rec.PreviousNo() + ';'); end; }
page 62329 "PM Temp Part" { PageType = ListPart; SourceTable = "PM Row"; SourceTableTemporary = true;
  layout { area(Content) { repeater(R) { field("No."; Rec."No.") { } field(Descr; Rec.Descr) { } } } }
  procedure SetRows(var Src: Record "PM Row" temporary) begin if Src.FindFirst() then Rec.Copy(Src, true); end; }
page 62330 "PM Temp Host" { PageType = Card; SourceTable = "PM Row"; layout { area(Content) { field("No."; Rec."No.") { } part(Sub; "PM Temp Part") { } } }
  procedure Fill(var Src: Record "PM Row" temporary) begin CurrPage.Sub.PAGE.SetRows(Src); end; }
page 62331 "PM Temp Var" { PageType = List; SourceTable = "PM Row"; SourceTableTemporary = true;
  layout { area(Content) { repeater(R) { field("No."; Rec."No.") { } field(Descr; Rec.Descr) { } } } }
  var Max: Decimal;
  procedure SetMax(M: Decimal) begin Max := M; end;
  procedure GetMax(): Decimal begin exit(Max); end;
  procedure Fill() begin Rec."No." := 'T1'; Rec.Descr := 'before'; Rec.Insert(); end;
  procedure Mark() begin Rec.FindFirst(); Rec.Descr := 'marked'; Rec.Modify(); end;
  procedure Count(): Integer begin Rec.Reset(); exit(Rec.Count()); end;
  procedure First(): Text begin Rec.Reset(); if Rec.FindFirst() then exit(Rec.Descr); end; }
page 62332 "PM Open Modify" { PageType = List; SourceTable = "PM Row"; SourceTableTemporary = true;
  layout { area(Content) { repeater(R) { field("No."; Rec."No.") { } field(Descr; Rec.Descr) { } } } }
  procedure Fill() begin Rec."No." := 'T1'; Rec.Rank := 5; Rec.Insert(); Rec."No." := 'T2'; Rec.Rank := 6; Rec.Insert(); end;
  procedure FillView(var Src: Record "PM Row" temporary) var X: Record "PM Row" temporary; begin X.Copy(Rec); Rec.Reset(); Rec.DeleteAll(); if Src.FindSet() then repeat Rec := Src; Rec.Insert(); until Src.Next() = 0; Rec.SetView(X.GetView()); end;
  procedure Mark() begin if Rec.FindSet() then begin Rec.Descr := 'marked'; Rec.Modify(); if Rec.Next() = 0 then; end; end;
  trigger OnOpenPage() var P: Codeunit "PM Probe"; X: Record "PM Row"; begin if Rec.Modify() then; X := Rec; Rec.CalcSums(Rank); P.Note('sum=' + Format(Rec.Rank) + ';'); Rec := X; end; }
page 62098 "PM Find Card"
{
    PageType = Card; SourceTable = "PM Row";
    layout { area(Content) { field("No."; Rec."No.") { } } }
    actions { area(Processing) { action(Refresh) { trigger OnAction() begin CurrPage.Update(false); end; } } }
    trigger OnFindRecord(Which: Text): Boolean var P: Codeunit "PM Probe"; begin P.Note('F' + Which + ';'); exit(Rec.Find(Which)); end;
    trigger OnAfterGetCurrRecord() var P: Codeunit "PM Probe"; begin P.Note('G;'); end;
}
page 62097 "PM Close Document"
{
    PageType = Document; SourceTable = "PM Row";
    layout { area(Content) { field("No."; Rec."No.") { } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var P: Codeunit "PM Probe"; begin P.Note('Document ' + Format(CloseAction) + ';'); exit(true); end;
}
report 62092 "PM Scheduled"
{
    ProcessingOnly = true; AllowScheduling = true;
    dataset { dataitem(Row; "PM Row") { } }
}
page 62093 "PM Runs Scheduled"
{
    PageType = Card;
    layout { area(Content) { field(Dummy; Dummy) { } } }
    actions { area(Processing) { action(Print) { trigger OnAction() var R: Report "PM Scheduled"; begin R.Run(); end; } } }
    var Dummy: Text[10];
}
page 62094 "PM Selection"
{
    PageType = List; SourceTable = "PM Row";
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } } } }
    procedure Selected(var Out: Record "PM Row") begin CurrPage.SetSelectionFilter(Out); end;
    procedure Picked(var Out: Record "PM Row") var Other: Page "PM Selection"; begin Other.GetRecord(Out); end;
}
page 62095 "PM Bounded"
{
    PageType = Card;
    layout { area(Content) { field(Tol; Tol) { MinValue = 0; MaxValue = 100; } } }
    var Tol: Decimal;
}
page 62091 "PM Kinds"
{
    PageType = Card; SourceTable = "PM Row";
    layout {
        area(Content) { label(Hint) { Caption = 'x'; } field(NoCtl; Rec."No.") { } }
        area(FactBoxes) { systempart(NotesBox; Notes) { } systempart(LinksBox; Links) { } usercontrol(Addin; "Some Add-in") { } }
    }
    actions { area(Processing) { group(Flow) { customaction(MakeFlow) { CustomActionType = FlowTemplateGallery; } } action(Act) { trigger OnAction() begin end; } fileuploadaction(Upload) { trigger OnAction(Files: List of [FileUpload]) begin end; } } }
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62100 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

const HELPERS: &str = "procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;";

// handlers/TestPageOnInit_Tests.al:188 POI_SetterBeforeRunModal_RunsAfterOnInitAndSurvives
#[test]
fn a_procedure_on_a_page_variable_runs_after_on_init_and_before_the_open() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('WizardHandler')]
        procedure SetterBeforeRunModal() var W: Page \"PM Wizard\"; P: Codeunit \"PM Probe\"; begin
            P.Reset(); W.SetStep(2); W.RunModal();
            Check('ISO', P.Trace()); end;
        [ModalPageHandler] procedure WizardHandler(var T: TestPage \"PM Wizard\") begin Check('2', T.StepCtl.Value()); end;"
    ));
}

// handlers/TestPageOnInit_Tests.al:206 POI_SetterBeforeRun_RunsAfterOnInitAndSurvives
#[test]
fn the_non_modal_run_of_a_page_variable_keeps_what_the_setter_chose() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('WizardHandler')]
        procedure SetterBeforeRun() var W: Page \"PM Wizard\"; P: Codeunit \"PM Probe\"; begin
            P.Reset(); W.SetStep(1); W.Run();
            Check('ISO', P.Trace()); end;
        [PageHandler] procedure WizardHandler(var T: TestPage \"PM Wizard\") begin Check('1', T.StepCtl.Value()); end;"
    ));
}

// handlers/TestPageOnInit_Tests.al:223 POI_SetterBeforeTrappedRun_RunsAfterOnInitAndSurvives
#[test]
fn a_trapped_run_receives_the_page_variables_instance() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure SetterBeforeTrappedRun() var W: Page \"PM Wizard\"; T: TestPage \"PM Wizard\"; P: Codeunit \"PM Probe\"; begin
            P.Reset(); T.Trap(); W.SetStep(1); W.Run();
            Check('ISO', P.Trace()); Check('1', T.StepCtl.Value()); T.Close(); end;"
    ));
}

// Microsoft's Test Posting Preview traps the entries page and, in a callee, the posting preview page, then runs the
// preview and the page its action opens: two Trap()s wait at once, one per page (Tests-ERM, 17 tests).
#[test]
fn two_traps_wait_at_once_one_per_page() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure TwoTraps() var A: TestPage \"PM Wizard\"; B: TestPage \"PM Second\"; begin
            A.Trap(); B.Trap();
            Page.Run(Page::\"PM Second\");
            Page.Run(Page::\"PM Wizard\");
            Check('0', A.StepCtl.Value()); Check('second', B.Label2.Value()); A.Close(); B.Close(); end;"
    ));
}

// handlers/TestPageOnInit_Tests.al GetSelectedKindOrdinal_WithoutRunModal_ReadsTheDeclaredDefault (handlers/TestPageEnumGlobal*)
#[test]
fn a_getter_on_a_page_variable_that_never_ran_reads_the_declared_default() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure GetterWithoutRun() var W: Page \"PM Wizard\"; P: Codeunit \"PM Probe\"; begin
            P.Reset(); if W.GetStep() <> 0 then Error('step'); Check('I', P.Trace()); end;"
    ));
}

// handlers/TestPageModalQueryClose_Tests.al:186 PersistInQueryClosePage_HandlerOk_WriteReachesTheTable
#[test]
fn a_page_variable_handed_a_record_writes_it_back_in_on_query_close_page() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('OkHandler')]
        procedure PersistOk() var R: Record \"PM Row\"; M: Page \"PM Persist\"; begin
            R.DeleteAll(); R.\"No.\" := 'F'; R.Rank := 999; R.Insert();
            M.SetTarget(R); M.LookupMode(true); M.RunModal();
            R.Get('F'); if R.Rank <> 0 then Error('rank %1', R.Rank); end;
        [ModalPageHandler] procedure OkHandler(var T: TestPage \"PM Persist\") begin T.OK().Invoke(); end;"
    ));
}

// handlers/TestPageEditable_Tests.al:93 EditableBoundToPageVariable_FollowsTheRow
#[test]
fn clear_of_a_closed_test_page_gives_a_fresh_variable() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure ClearClosed() var R: Record \"PM Row\"; C: TestPage \"PM Card\"; begin
            R.DeleteAll(); R.\"No.\" := 'A'; R.Descr := 'x'; R.Insert();
            C.OpenView(); Check('A', C.NoCtl.Value()); C.Close();
            Clear(C);
            C.OpenView(); Check('A', C.NoCtl.Value()); C.Close(); end;"
    ));
}

const EIGHT: &str = "procedure Seed(Alternate: Boolean) var R: Record \"PM Row\"; I: Integer; begin
    R.DeleteAll();
    for I := 1 to 8 do begin R.Init(); R.\"No.\" := 'L000' + Format(I); R.Descr := 'Row'; if Alternate then if I mod 2 = 1 then R.Descr := 'Included' else R.Descr := 'Excluded'; R.Insert(); end;
end;";

// pagefindrecord/TestPageFindRecord.al:112 OnFindRecord_FirstAnswersTheFirstRowOfTheTriggerRowset,
// :129 OnNextRecord_NextWalksTheTriggerRowsetAndStopsAtItsEnd
#[test]
fn first_and_next_walk_the_rows_the_triggers_serve() {
    passes(&format!(
        "{HELPERS} {EIGHT}
        [Test]
        procedure Walk() var L: TestPage \"PM Rowset\"; begin
            Seed(false); L.OpenView(); L.FillEveryOther.Invoke();
            if not L.First() then Error('first'); Check('L0007', L.\"No.\".Value);
            if not L.Next() then Error('n2'); Check('L0005', L.\"No.\".Value);
            if not L.Next() then Error('n3'); Check('L0003', L.\"No.\".Value);
            if not L.Next() then Error('n4'); Check('L0001', L.\"No.\".Value);
            if L.Next() then Error('past the end');
            L.Close(); end;"
    ));
}

// :160 OnFindRecord_LastAndPreviousWalkTheTriggerRowset, :186 OnFindRecord_GoToKeyRefusesARowTheTriggerRowsetDoesNotServe
#[test]
fn last_previous_and_go_to_key_use_the_trigger_rowset() {
    passes(&format!(
        "{HELPERS} {EIGHT}
        [Test]
        procedure LastPrev() var L: TestPage \"PM Rowset\"; begin
            Seed(false); L.OpenView(); L.FillEveryOther.Invoke();
            if not L.Last() then Error('last'); Check('L0001', L.\"No.\".Value);
            if not L.Previous() then Error('prev'); Check('L0003', L.\"No.\".Value);
            L.Close(); end;
        [Test]
        procedure GoTo() var L: TestPage \"PM Rowset\"; begin
            Seed(false); L.OpenView(); L.FillEveryOther.Invoke();
            if not L.GoToKey('L0003') then Error('served'); Check('L0003', L.\"No.\".Value);
            if L.GoToKey('L0004') then Error('not served');
            L.Close(); end;"
    ));
}

// :205 PlainPage_WithoutTheTriggers..., :222 OnFindRecord_AManualFilterStillNarrowsTheTriggerRowset
#[test]
fn a_manual_filter_narrows_the_trigger_rowset_and_both_triggers_are_reached() {
    passes(&format!(
        "{HELPERS} {EIGHT}
        [Test]
        procedure Filtered() var L: TestPage \"PM Rowset\"; P: Codeunit \"PM Probe\"; begin
            Seed(true); L.OpenView(); L.Filter.SetFilter(Descr, 'Included'); L.FillAll.Invoke();
            Check('', L.Filter.GetFilter(\"No.\")); Check('Included', L.Filter.GetFilter(Descr));
            P.Reset();
            if not L.First() then Error('first'); Check('L0007', L.\"No.\".Value);
            if not L.Next() then Error('n2'); Check('L0005', L.\"No.\".Value);
            if not L.Next() then Error('n3'); Check('L0003', L.\"No.\".Value);
            if not L.Next() then Error('n4'); Check('L0001', L.\"No.\".Value);
            if L.Next() then Error('past');
            if StrPos(P.Trace(), 'F') = 0 then Error('find'); if StrPos(P.Trace(), 'N') = 0 then Error('next');
            L.Close(); end;"
    ));
}

// A control read straight after a refresh of a rowset page shows a row the re-anchoring picks, which
// the corpus does not pin beyond the Which strings of a deleted row (pageupdate/TestPageUpdateGoneRow.al).
#[test]
fn reading_a_rec_bound_control_after_a_refresh_of_a_rowset_page_is_refused() {
    let o = outcomes(&format!(
        "{HELPERS} {EIGHT}
        [Test]
        procedure Stale() var L: TestPage \"PM Rowset\"; begin
            Seed(false); L.OpenView(); L.FillEveryOther.Invoke();
            Check('L0008', L.\"No.\".Value); end;"
    ));
    assert!(
        matches!(&o[0].1, Outcome::NotRun(why) if why.contains("after CurrPage.Update")),
        "{o:?}"
    );
}

const LOOK: &str = "procedure OpenOn(var C: TestPage \"PM Look Card\") var L: Record \"PM Look\"; R: Record \"PM Related\"; X: Record \"PM Pageless\"; begin
    L.DeleteAll(); L.\"No.\" := 'R1'; L.Insert();
    R.DeleteAll(); R.Code := 'REL-A'; R.Descr := 'Alpha'; R.Insert(); R.Code := 'REL-B'; R.Descr := 'Beta'; R.Insert();
    X.DeleteAll(); X.Code := 'PL-A'; X.Insert();
    C.OpenEdit(); C.GoToRecord(L); end;";

// handlers/TestPageFieldTableLookup.al:55 Lookup_ControlWithoutTrigger_RunsTheTableFieldsOnLookup,
// :72 Lookup_ControlWithTrigger_WinsOverTheTableFieldsOnLookup
#[test]
fn lookup_runs_the_controls_trigger_else_the_table_fields() {
    passes(&format!(
        "{HELPERS} {LOOK}
        [Test]
        procedure TableOnly() var C: TestPage \"PM Look Card\"; begin
            OpenOn(C); C.\"Table Only\".Lookup(); Check('FROM-TABLE', C.\"Table Only\".Value); C.Close(); end;
        [Test]
        procedure ControlWins() var C: TestPage \"PM Look Card\"; begin
            OpenOn(C); C.Both.Lookup(); Check('FROM-CONTROL', C.Both.Value); C.Close(); end;"
    ));
}

// handlers/TestPageFieldRelationLookup_Tests.al:104 ...OpensTheRelatedTablesLookupPage, :122 ...WritesTheSelectedRowBack,
// :142 ...CancelLeavesTheFieldUnchanged, :168 Lookup_NoTriggerAndNoTableRelation_DoesNothing
#[test]
fn lookup_over_a_table_relation_opens_the_related_lookup_page_and_writes_the_pick_back() {
    passes(&format!(
        "{HELPERS} {LOOK}
        [Test] [HandlerFunctions('PicksB')]
        procedure Picks() var C: TestPage \"PM Look Card\"; begin
            OpenOn(C); C.Related.Lookup(); Check('REL-B', C.Related.Value); C.Close(); end;
        [Test] [HandlerFunctions('Cancels')]
        procedure Cancel() var C: TestPage \"PM Look Card\"; begin
            OpenOn(C); C.Related.Lookup(); Check('', C.Related.Value); C.Close(); end;
        [Test]
        procedure NothingToResolve() var C: TestPage \"PM Look Card\"; begin
            OpenOn(C); C.Plain.SetValue('KEEP'); C.Plain.Lookup(); Check('KEEP', C.Plain.Value); C.Close(); end;
        [ModalPageHandler] procedure PicksB(var M: TestPage \"PM Related List\") begin M.GoToKey('REL-B'); M.OK().Invoke(); end;
        [ModalPageHandler] procedure Cancels(var M: TestPage \"PM Related List\") begin M.GoToKey('REL-B'); M.Cancel().Invoke(); end;"
    ));
}

// handlers/TestPageFieldRelationLookup_Tests.al:253 Lookup_RelationToTableWithNoLookupPage_RaisesTheUnhandledUiSignal,
// :273 ..._LeavesTheFieldUnchanged
#[test]
fn lookup_over_a_relation_to_a_table_without_a_lookup_page_raises_unhandled_ui() {
    passes(&format!(
        "{HELPERS} {LOOK}
        [Test]
        procedure Pageless() var C: TestPage \"PM Look Card\"; begin
            OpenOn(C); Commit(); C.Pageless.SetValue('PL-A');
            asserterror C.Pageless.Lookup();
            if StrPos(GetLastErrorText(), 'Unhandled UI: ModalPage') = 0 then Error('got %1', GetLastErrorText());
            Check('PL-A', C.Pageless.Value); C.Close(); end;"
    ));
}

// With the host row still uncommitted, how far the asserterror rolls back is not pinned: refused.
#[test]
fn an_unhandled_lookup_with_writes_pending_is_refused() {
    let o = outcomes(&format!(
        "{HELPERS} {LOOK}
        [Test]
        procedure Pending() var C: TestPage \"PM Look Card\"; begin
            OpenOn(C); C.Pageless.SetValue('PL-A'); asserterror C.Pageless.Lookup(); end;"
    ));
    assert!(
        matches!(&o[0].1, Outcome::NotRun(why) if why.contains("writes pending")),
        "{o:?}"
    );
}

const BG: &str = "procedure SeedBg() var R: Record \"PM Row\"; begin R.DeleteAll();
    R.\"No.\" := 'A'; R.Insert(); R.\"No.\" := 'FAIL-H'; R.Descr := 'handled'; R.Insert(); R.\"No.\" := 'FAIL-U'; R.Descr := ''; R.Insert(); end;";

// handlers/TestPageBgTask_Tests.al:61 EnqueueBackgroundTask_CompletesBeforeOpenViewReturns, :146 ..._HandledErrorDoesNotPropagate
#[test]
fn an_enqueued_task_has_completed_when_the_call_returns_and_a_handled_error_is_swallowed() {
    passes(&format!(
        "{HELPERS} {BG}
        [Test]
        procedure Completes() var C: TestPage \"PM Bg Card\"; begin
            SeedBg(); C.OpenView(); Check('BG:A', C.OutCtl.Value); C.Close(); end;
        [Test]
        procedure Handled() var C: TestPage \"PM Bg Card\"; R: Record \"PM Row\"; begin
            SeedBg(); R.Get('FAIL-H'); C.OpenView(); if not C.GoToRecord(R) then Error('goto');
            Check('worker failed for FAIL-H', C.ErrCtl.Value); C.Close(); end;"
    ));
}

// handlers/TestPageBgTask_Tests.al:170 EnqueueBackgroundTask_UnhandledErrorPropagates
#[test]
fn an_unhandled_task_error_tears_the_page_down() {
    passes(&format!(
        "{HELPERS} {BG}
        [Test]
        procedure Unhandled() var C: TestPage \"PM Bg Card\"; R: Record \"PM Row\"; begin
            SeedBg(); R.Get('FAIL-U'); C.OpenView();
            asserterror C.GoToRecord(R);
            if StrPos(GetLastErrorText(), 'The TestPage is not open') = 0 then Error('got %1', GetLastErrorText()); end;"
    ));
}

// handlers/TestPageBgTask_Tests.al:104 RunPageBackgroundTask_ReturnsWorkerResult, :210 ..._WorkerInsert_RefusedByReadOnlySession
#[test]
fn run_page_background_task_returns_the_result_and_the_worker_cannot_write() {
    passes(&format!(
        "{HELPERS} {BG}
        [Test]
        procedure Result() var C: TestPage \"PM Bg Card\"; P: Dictionary of [Text, Text]; R: Dictionary of [Text, Text]; V: Text; begin
            SeedBg(); C.OpenView(); P.Add('No', 'RPT'); R := C.RunPageBackgroundTask(Codeunit::\"PM Worker\", P, false);
            R.Get('Out', V); Check('BG:RPT', V); C.Close(); end;
        [Test]
        procedure NoWrite() var C: TestPage \"PM Bg Card\"; P: Dictionary of [Text, Text]; Row: Record \"PM Row\"; begin
            SeedBg(); C.OpenView(); P.Add('No', 'WRITE');
            if TryRun(C, P) then Error('must fail');
            if StrPos(GetLastErrorText(), 'Sorry, the current permissions prevented the action. (TableData 62000 PM Row Insert: ') = 0 then Error('got %1', GetLastErrorText());
            if GetLastErrorCode() <> 'DB:ReadOnlySession' then Error('code %1', GetLastErrorCode());
            if Row.Get('W') then Error('landed'); C.Close(); end;
        [TryFunction] local procedure TryRun(var C: TestPage \"PM Bg Card\"; P: Dictionary of [Text, Text]) var R: Dictionary of [Text, Text]; begin R := C.RunPageBackgroundTask(Codeunit::\"PM Worker\", P, false); end;"
    ));
}

// session/lt-cn-session-leads/CnIsolatedStorageLeads.al:104-134: IsolatedStorage.Set and Delete in a worker are refused like a
// table write, on table 2000000107.
#[test]
fn a_worker_cannot_write_to_isolated_storage() {
    passes(&format!(
        "{HELPERS} {BG}
        [Test]
        procedure SetIso() var C: TestPage \"PM Bg Card\"; P: Dictionary of [Text, Text]; begin
            SeedBg(); if IsolatedStorage.Contains('PMbg') then IsolatedStorage.Delete('PMbg');
            C.OpenView(); P.Add('No', 'ISOSET');
            if TryRun(C, P) then Error('must fail');
            if StrPos(GetLastErrorText(), 'Sorry, the current permissions prevented the action. (TableData 2000000107 Isolated Storage Insert: ') = 0 then Error('got %1', GetLastErrorText());
            if GetLastErrorCode() <> 'DB:ReadOnlySession' then Error('code %1', GetLastErrorCode());
            if IsolatedStorage.Contains('PMbg') then Error('landed'); C.Close(); end;
        [Test]
        procedure DelIso() var C: TestPage \"PM Bg Card\"; P: Dictionary of [Text, Text]; begin
            SeedBg(); IsolatedStorage.Set('PMbg', 'seed'); Commit();
            C.OpenView(); P.Add('No', 'ISODEL');
            if TryRun(C, P) then Error('must fail');
            if StrPos(GetLastErrorText(), 'Sorry, the current permissions prevented the action. (TableData 2000000107 Isolated Storage Delete: ') = 0 then Error('got %1', GetLastErrorText());
            if GetLastErrorCode() <> 'DB:ReadOnlySession' then Error('code %1', GetLastErrorCode());
            if not IsolatedStorage.Contains('PMbg') then Error('gone'); C.Close(); end;
        [TryFunction] local procedure TryRun(var C: TestPage \"PM Bg Card\"; P: Dictionary of [Text, Text]) var R: Dictionary of [Text, Text]; begin R := C.RunPageBackgroundTask(Codeunit::\"PM Worker\", P, false); end;"
    ));
}

// session/lt-cn-session-leads/CnIsolatedStorageLeads.al:445-522: a worker's Get of a key the caller wrote and has not committed
// reads no key; after a Commit of the caller it fails with the lock timeout of the Isolated Storage table.
#[test]
fn a_worker_get_of_a_key_the_caller_wrote_misses_it_or_waits_on_the_lock() {
    passes(&format!(
        "{HELPERS} {BG}
        [Test]
        procedure Uncommitted() var C: TestPage \"PM Bg Card\"; P: Dictionary of [Text, Text]; R: Dictionary of [Text, Text]; V: Text; begin
            IsolatedStorage.Set('PMbg', 'fromcaller');
            C.OpenView(); P.Add('No', 'ISOGET'); R := C.RunPageBackgroundTask(Codeunit::\"PM Worker\", P, false);
            R.Get('Ret', V); Check('false', V); R.Get('Value', V); Check('', V); C.Close(); end;"
    ));
    passes(&format!(
        "{HELPERS} {BG}
        [Test]
        procedure Committed() var C: TestPage \"PM Bg Card\"; P: Dictionary of [Text, Text]; begin
            IsolatedStorage.Set('PMbg', 'fromcaller'); Commit();
            C.OpenView(); P.Add('No', 'ISOGET');
            if TryRun(C, P) then Error('must fail');
            Check('We can''t save your changes right now, because a record in table ''Isolated Storage'' is being updated in a transaction done by another session.\\\\You''ll have to wait until the other transaction has completed, which may take a while. Please try again later.', GetLastErrorText());
            Check('NDBCS:LockTimeout', GetLastErrorCode()); C.Close(); end;
        [TryFunction] local procedure TryRun(var C: TestPage \"PM Bg Card\"; P: Dictionary of [Text, Text]) var R: Dictionary of [Text, Text]; begin R := C.RunPageBackgroundTask(Codeunit::\"PM Worker\", P, false); end;"
    ));
}

// A worker's transaction is not modelled: Commit in it is refused.
#[test]
fn a_commit_in_a_worker_is_refused() {
    let o = outcomes(&format!(
        "{HELPERS} {BG}
        [Test]
        procedure Commits() var C: TestPage \"PM Bg Card\"; P: Dictionary of [Text, Text]; R: Dictionary of [Text, Text]; begin
            SeedBg(); C.OpenView(); P.Add('No', 'COMMIT'); R := C.RunPageBackgroundTask(Codeunit::\"PM Worker\", P, false); end;"
    ));
    assert!(
        matches!(&o[0].1, Outcome::NotRun(why) if why.contains("Commit in a page background task")),
        "{o:?}"
    );
}

// record/TestValidateRaisesAndHandlerAsserterrorContracts.al:122-143
// (Asserterror_InAModalHandler_RollsBackToTheLastCommit). An asserterror inside a
// modal handler rolls back everything since the last Commit: the write pending before the page
// opened, the handler's own earlier write and the write inside the asserterror.
#[test]
fn an_asserterror_in_a_modal_handler_rolls_back_to_the_last_commit() {
    passes(&format!(
        "{HELPERS}
        local procedure Has(No: Code[20]): Text var R: Record \"PM Row\"; begin exit(Format(R.Get(No))); end;
        local procedure Put(No: Code[20]) var R: Record \"PM Row\"; begin R.Init(); R.\"No.\" := No; R.Insert(); end;
        [Test] [HandlerFunctions('Dialog')]
        procedure InHandler() var R: Record \"PM Row\"; Result: Action; begin
            R.DeleteAll(); Put('1'); Commit(); Put('2');
            Result := Page.RunModal(Page::\"PM Dialog\");
            Check('Yes', Has('1')); Check('No', Has('2')); Check('No', Has('3')); Check('No', Has('4'));
            Check('No', Format(Database.IsInWriteTransaction())); end;
        [ModalPageHandler] procedure Dialog(var D: TestPage \"PM Dialog\") begin
            Put('3'); asserterror begin Put('4'); Error('in handler'); end; end;"
    ));
}

// handlers/TestPagePlatformPageSourceTable_Tests.al:77, handlers/TestReportRunFromPrecompiledCaller.al:31,
// handlers/TestPrecompiledOptionCaption.al:81, pageextensioncontrol/TestPageExtensionModifyPrecompiled_Tests.al:69:
// a page with a systempart, a label and a customaction opens, and its other controls work.
#[test]
fn a_page_with_a_systempart_a_label_and_a_customaction_opens() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Opens() var R: Record \"PM Row\"; K: TestPage \"PM Kinds\"; begin
            R.DeleteAll(); R.\"No.\" := 'K1'; R.Insert(); K.OpenView(); K.GoToRecord(R);
            Check('K1', K.NoCtl.Value); Check('Yes', Format(K.Act.Enabled())); K.Close(); end;"
    ));
}

// A usercontrol (control add-in) is not a control of the TestPage either.
#[test]
fn naming_a_usercontrol_on_a_test_page_is_refused() {
    let o = outcomes(&format!(
        "{HELPERS}
        [Test]
        procedure Names() var K: TestPage \"PM Kinds\"; begin
            K.OpenView(); Check('Yes', Format(K.Addin.Visible())); end;"
    ));
    assert!(matches!(&o[0].1, Outcome::NotRun(_)), "{o:?}");
}

// The TestPage has no member for these controls, so a reference is refused, not answered.
#[test]
fn naming_a_systempart_on_a_test_page_is_refused() {
    let o = outcomes(&format!(
        "{HELPERS}
        [Test]
        procedure Names() var K: TestPage \"PM Kinds\"; begin
            K.OpenView(); Check('Yes', Format(K.NotesBox.Visible())); end;"
    ));
    assert!(matches!(&o[0].1, Outcome::NotRun(_)), "{o:?}");
}

// handlers/TestPrecompiledOptionCaption.al:81: a page whose action names a report with AllowScheduling opens.
#[test]
fn a_page_naming_a_report_with_allow_scheduling_opens() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Opens() var P: TestPage \"PM Runs Scheduled\"; begin
            P.OpenView(); Check('Yes', Format(P.Print.Enabled())); P.Close(); end;"
    ));
}

// Running a processing-only report with AllowScheduling is an ordinary run (report/TestProbeReportContracts.al:
// AllowScheduling_TrueAndFalse_Run).
#[test]
fn running_a_report_with_allow_scheduling_runs_it() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Runs() begin Report.Run(Report::\"PM Scheduled\", false); end;"
    ));
}

// SetSelectionFilter and GetRecord are in no corpus test: a page that declares a routine calling them
// opens (handlers/TestPrecompiledOptionCaption.al:81 over Base App's Item Tracing, pageapplicationarea/
// TestPageApplicationAreaPrecompiledActionRemoval.al:39 over Package No. Information List), reaching them is refused.
#[test]
fn a_page_declaring_selection_routines_opens() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Opens() var P: TestPage \"PM Selection\"; begin P.OpenView(); P.Close(); end;"
    ));
}

#[test]
fn set_selection_filter_of_a_page_that_never_ran_runs_and_get_record_is_refused() {
    let o = outcomes(&format!(
        "{HELPERS}
        [Test]
        procedure Selects() var P: Page \"PM Selection\"; R: Record \"PM Row\"; begin P.Selected(R); end;
        [Test]
        procedure Gets() var P: Page \"PM Selection\"; R: Record \"PM Row\"; begin P.Picked(R); end;"
    ));
    // A Page variable that never ran and has no OnInit filters the record on its blank key (pages/lt-page-runtime/P82PageVar.al:86,
    // pages/lt-bl-unopened-currpage/UnopenedCurrPage.al:159).
    assert!(matches!(&o[0].1, Outcome::Pass), "{o:?}");
    assert!(
        matches!(&o[1].1, Outcome::NotRun(why) if why.contains("GetRecord")),
        "{o:?}"
    );
}

// `P.Editable := false` and `P.Editable(false)` on a Page variable that has not run are accepted without an error; the
// variable reads what it was set to, and true before (handlers/TestPageVariableEditableContracts.al:
// EditableBeforeRun_IsTrue_AndAssignmentIsAccepted).
#[test]
fn editable_on_a_page_variable_that_has_not_run_is_accepted_and_read_back() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Assigns() var P: Page \"PM Selection\"; begin
            Check('Yes', Format(P.Editable())); P.Editable := false; Check('No', Format(P.Editable()));
            P.Editable(true); Check('Yes', Format(P.Editable())); P.Editable(false); Check('No', Format(P.Editable())); end;"
    ));
}

// The value set before the run is what the page opens with, for RunModal (handlers/TestPageVariableEditableContracts.al:
// RunModal_AfterEditableFalse_OpensANonEditablePage, :RunModal_AfterAssigningEditableTrue_OpensAnEditablePage).
#[test]
fn a_page_variable_set_non_editable_opens_the_page_non_editable() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Handler')]
        procedure Off() var P: Page \"PM Selection\"; begin P.Editable(false); P.RunModal(); end;
        [ModalPageHandler] procedure Handler(var T: TestPage \"PM Selection\") begin Check('No', Format(T.Editable())); end;"
    ));
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Handler')]
        procedure On() var P: Page \"PM Selection\"; begin P.Editable := true; P.RunModal(); end;
        [ModalPageHandler] procedure Handler(var T: TestPage \"PM Selection\") begin Check('Yes', Format(T.Editable())); end;"
    ));
}

// A control bound to a variable with its own MinValue/MaxValue: a value inside the bounds is written, one outside raises
// the TestValidation error with the pinned text and keeps the old value (handlers/TestPageVariableMinMaxContracts.al:
// IntegerVariable_OutsideTheBounds_RaisesATestValidationError, DecimalVariable_OutsideTheBounds_...).
#[test]
fn a_variable_control_with_bounds_takes_inside_values_and_raises_a_validation_error_for_the_others() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Inside() var P: TestPage \"PM Bounded\"; begin
            P.OpenEdit(); P.Tol.SetValue(50); Check('50.00', P.Tol.Value());
            asserterror P.Tol.SetValue(150);
            Check('Validation error for Field: Tol,  Message = ''The value must be less than or equal to 100. Value: 150.00.''', GetLastErrorText());
            Check('TestValidation', GetLastErrorCode());
            Check('50.00', P.Tol.Value());
            asserterror P.Tol.SetValue(-1);
            Check('Validation error for Field: Tol,  Message = ''The value must be greater than or equal to 0. Value: -1.00.''', GetLastErrorText());
            P.Close(); end;"
    ));
}

// handlers/TestPageCloseActionByPageTypeContracts.al:TestPageClose_IsOk_ForEveryPageType: TestPage.Close() reports OK to
// OnQueryClosePage for a list and a document, not only a card; a dialog still reports Cancel.
#[test]
fn test_page_close_reports_ok_for_list_and_document_pages() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure CloseIsOk() var L: TestPage \"PM Close List\"; D: TestPage \"PM Close Document\"; P: Codeunit \"PM Probe\"; begin
            P.Reset(); L.OpenView(); L.Close(); D.OpenView(); D.Close();
            Check('List OK;Document OK;', P.Trace()); end;"
    ));
}

// Issue 33: Base Application's Tests-ERM declare `[ModalPageHandler] procedure H(var P: Page "X"; var
// Response: Action)` (22 tests). No corpus test has a handler whose parameter is a Page rather than a
// TestPage, and none has a page with triggers. Microsoft's `PaymentToleranceWarning` handler (page 591, whose OnOpenPage and
// OnQueryClosePage overwrite what `InitializeOption` sets) needs the page's triggers not to run: pinned by
// errors/lt-ms-refusals-136/P136PageTyped.al (OnInit runs before the handler, nothing else does).
#[test]
fn a_modal_page_handler_with_a_page_parameter_stands_in_for_the_page_and_runs_only_its_oninit() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('PageHandler')]
        procedure WithPage() var W: Page \"PM Wizard\"; P: Codeunit \"PM Probe\"; begin P.Reset(); W.RunModal(); Check('I', P.Trace()); end;
        [ModalPageHandler] procedure PageHandler(var P: Page \"PM Wizard\"; var Response: Action) begin Response := Action::OK; end;"
    ));
}

// Base Application's Financial Report sets `RequestOptionsPage.Caption := Description` while its request page opens
// (86 Tests-ERM tests were refused with "is not assignable"). The corpus pins `CurrPage.Caption := x` on a page
// (handlers/TestPageCaption_Page.al:44); for a request page the assignment is accepted and not shown to the handler
// (request-page-caption-set, docs/probes-needed.md).
#[test]
fn a_request_page_caption_can_be_set_from_the_code_of_the_report() {
    let src = format!(
        "{OBJECTS}
report 62200 \"PM Req Report\"
{{
    ProcessingOnly = true;
    requestpage
    {{
        layout {{ area(Content) {{ field(Dummy; DummyValue) {{ ApplicationArea = All; }} }} }}
        trigger OnOpenPage() begin SetCaption(); end;
    }}
    var DummyValue: Integer;
    local procedure SetCaption() begin RequestOptionsPage.Caption := 'Custom caption'; end;
}}
codeunit 62201 T {{ Subtype = Test;
    [Test] [HandlerFunctions('Handler')]
    procedure CaptionIsSet() begin Report.Run(Report::\"PM Req Report\"); end;
    [RequestPageHandler] procedure Handler(var R: TestRequestPage \"PM Req Report\") begin
        R.Cancel().Invoke(); end;
}}"
    );
    let o = al_testkit::outcomes_in(&src, "T");
    assert_eq!(o.len(), 1, "{o:?}");
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// Base Application's document pages declare OnFindRecord and OnAfterGetCurrRecord and call CurrPage.Update (88 Tests-ERM tests
// were refused). The corpus pins the Which strings only for a deleted row (pageupdate/TestPageUpdateGoneRow.al:390); that
// the refresh asks OnFindRecord for the current row with `=` and then raises the row triggers is assumed
// (update-refresh-onfindrecord, docs/probes-needed.md).
#[test]
fn current_page_update_on_a_page_with_on_find_record_and_row_triggers_finds_the_current_row_again() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Refreshes() var C: TestPage \"PM Find Card\"; R: Record \"PM Row\"; P: Codeunit \"PM Probe\"; begin
            R.DeleteAll(); R.\"No.\" := 'F1'; R.Insert();
            C.OpenEdit(); C.GoToRecord(R); P.Reset(); C.Refresh.Invoke();
            Check('F=;G;', P.Trace()); Check('F1', C.\"No.\".Value); C.Close(); end;"
    ));
}

// record/lt-loadfields-pagesource-validate/P81PageValidateEvents.al: SetValue on a control raises the page's
// OnBeforeValidateEvent and OnAfterValidateEvent for that control around the control's own OnValidate (80 Tests-ERM tests
// were refused).
#[test]
fn set_value_raises_the_page_validate_events_around_the_control_trigger() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Events() var C: TestPage \"PM Event Card\"; R: Record \"PM Row\"; P: Codeunit \"PM Probe\"; begin
            R.DeleteAll(); R.\"No.\" := 'E1'; R.Insert();
            C.OpenEdit(); C.GoToRecord(R); P.Reset(); C.Descr.SetValue('hello');
            Check('B(hello);V;A(hello);', P.Trace()); C.Close(); end;"
    ));
}

// Base Application's VendEntry-Apply Posted Entries hands "Apply Vendor Entries" `SetRecord(Entry)` and then `SetTableView(Entry)`;
// the page reads `Rec."Vendor No."` in OnOpenPage (39 Tests-ERM tests failed with "The Vendor does not exist ... No.=''").
// SetRecord gives the position and SetTableView the filters, so the second does not blank the record of the first
// (handlers/TestPageModalNewRowLine_Tests.al pins SetTableView alone: no position; docs/probes-needed.md for the pair).
#[test]
fn set_table_view_after_set_record_keeps_the_record_the_page_opens_on() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('H')]
        procedure Pair() var R: Record \"PM Row\"; V: Record \"PM Row\"; W: Page \"PM SetRec List\"; P: Codeunit \"PM Probe\"; begin
            R.DeleteAll(); R.\"No.\" := 'S1'; R.Descr := 'x'; R.Insert(); R.\"No.\" := 'S2'; R.Descr := 'y'; R.Insert();
            V.SetRange(Descr, 'y'); V.FindFirst(); P.Reset();
            W.SetRecord(V); W.SetTableView(V); W.RunModal();
            Check('open=S2;', P.Trace()); end;
        [ModalPageHandler] procedure H(var T: TestPage \"PM SetRec List\") begin end;"
    ));
}

// Base Application's Allocation Account page is a ListPlus card with a part linked on its key. A test opens it with OpenNew and types the
// key: while the host row is not saved, the part's blank line links to a blank value, so its table relation finds nothing to
// refuse (34 Tests-ERM tests failed with "contains a value that cannot be found in the related table"). Pinned by
// pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:398, :400.
#[test]
fn a_part_linked_to_a_new_unsaved_host_links_to_a_blank_value_when_the_key_is_typed() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure KeyThenName() var C: TestPage \"PM Host Card\"; H: Record \"PM Host\"; begin
            H.DeleteAll(); C.OpenNew(); C.\"No.\".SetValue('NEWHOST'); C.Name.SetValue('n');
            if not H.Get('NEWHOST') then Error('host not saved'); C.Close(); end;"
    ));
}

// Base Application's lookups run `Page.RunModal(0, Rec) = Action::LookupOK` and read the record afterwards (Gen. Journal
// Management's template selection: 10 Tests-ERM tests failed with an empty template name). On LookupOK the record holds
// the row the page was left on. Pinned by pages/lt-page-runmodal-rec/PageRunModalRecAfterClose.al.
#[test]
fn page_run_modal_with_a_record_hands_back_the_selected_row_on_lookup_ok() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('Pick')]
        procedure PicksB() var R: Record \"PM Pick\"; T: Record \"PM Pick\"; A: Action; begin
            R.DeleteAll(); R.Id := 'A'; R.Grp := 1; R.Insert(); R.Id := 'B'; R.Grp := 1; R.Insert(); R.Id := 'C'; R.Grp := 2; R.Insert();
            T.SetRange(Grp, 1); A := Page.RunModal(0, T); Check('LookupOK', Format(A)); Check('B', T.Id); end;
        [ModalPageHandler] procedure Pick(var L: TestPage \"PM Pick List\") begin L.GoToKey('B'); L.OK().Invoke(); end;"
    ));
}

// Microsoft's "Item Vendor Catalog" tests open a list with OpenNew, set a filter on the first key field and type the other key
// field: the list shows the blank line and the filter value is on it (2 Tests-ERM tests failed with "The row does not exist on the
// TestPage"). The corpus pins the blank line of an editable list (handlers/TestPageNewRowLine_Tests.al) and that a new row takes
// a single-valued filter (handlers/TestPageNewRowFilters_Tests.al); that a filter change brings the blank line back is pinned by
// pages/lt-filter-change-new-line/FilterChangeNewLine.al.
#[test]
fn a_filter_that_leaves_an_editable_list_empty_lands_on_the_blank_line_carrying_the_filter_value() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure AfterOpenNew() var P: TestPage \"PM Cat List\"; R: Record \"PM Cat\"; begin
            R.DeleteAll(); P.OpenNew(); P.Filter.SetFilter(Item, 'X1'); P.Vend.SetValue('V1'); P.Info.SetValue('i');
            if not R.Get('X1', 'V1') then Error('not stored'); P.Close(); end;
        [Test]
        procedure AfterOpenEdit() var P: TestPage \"PM Cat List\"; R: Record \"PM Cat\"; begin
            R.DeleteAll(); P.OpenEdit(); P.Filter.SetFilter(Item, 'X2'); P.Vend.SetValue('V2'); P.Info.SetValue('i');
            if not R.Get('X2', 'V2') then Error('not stored'); P.Close(); end;"
    ));
}

// Base Application's Service Lines page declares `SetSelectionFilter()` and calls `CurrPage.SetSelectionFilter(ServiceLine)`:
// the platform method of another arity is still a candidate (154 Tests-SCM-Service tests were refused with "has no callee").
#[test]
fn a_page_procedure_named_like_a_platform_method_leaves_the_other_overload_to_the_platform() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Selection() var C: TestPage \"PM Selection Own\"; R: Record \"PM Row\"; begin
            R.DeleteAll(); R.\"No.\" := 'S1'; R.Insert(); C.OpenEdit(); C.Pick.Invoke(); Check('S1', C.\"No.\".Value); C.Close(); end;"
    ));
}

// Base Application's Sales Invoice calls `CurrPage.ApprovalFactBox.PAGE.UpdateApprovalEntriesFromSourceRecord(Rec.RecordId)` in OnAfterGetCurrRecord
// behind `Visible = false`; that procedure ends in `CurrPage.Update(false)` on the part page, which is created and not opened. 926 Tests-ERM
// tests failed with "The TestPage is not open." CurrPage.Update on such a part does nothing (pages/lt-hidden-part-currpage/HiddenPartCurrPageUpdateFalse.al:UpdateFalse_LiteralInvisible_OpenEdit).
#[test]
fn current_page_update_inside_a_procedure_of_a_hidden_part_does_nothing() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Hidden() var C: TestPage \"PM Hidden Host\"; R: Record \"PM Row\"; P: Codeunit \"PM Probe\"; begin
            R.DeleteAll(); R.\"No.\" := 'H1'; R.Insert(); P.Reset(); C.OpenEdit(); C.GoToRecord(R);
            if StrPos(P.Trace(), 'host;') = 0 then Error('host trigger did not finish'); C.Close(); end;"
    ));
}

// A procedure of a `Page` variable that uses `CurrPage` after the page closed is refused, not failed with the TestPage error
// (Posted Sales Document Lines.CopyLineToDoc after RunModal: 35 Tests-ERM, SCM and SMB tests). `Update(false)` there is pinned
// to do nothing (pages/lt-closed-page-variable-currpage/ClosedPageVariableCurrPageUpdate.al:70); `Update(true)` is not.
#[test]
fn a_page_variable_procedure_using_curr_page_after_the_page_closed_is_refused_not_failed() {
    let o = outcomes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('H')]
        procedure Closed() var W: Page \"PM Selection Own Update\"; begin W.RunModal(); W.UseAfterClose(); end;
        [ModalPageHandler] procedure H(var T: TestPage \"PM Selection Own Update\") begin T.OK().Invoke(); end;"
    ));
    assert!(
        matches!(&o[0].1, Outcome::NotRun(why) if why.contains("not open") && why.contains("not pinned")),
        "{o:?}"
    );
}

#[test]
fn a_control_editable_expression_follows_a_variable_set_by_another_controls_on_validate() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Follows() var C: TestPage \"PM Editable Flag\"; begin
            C.OpenNew(); C.Descr.SetValue('x');
            if not C.Other.Editable() then Error('should be editable'); C.Descr.SetValue('');
            if C.Other.Editable() then Error('should not be editable'); C.Close(); end;"
    ));
}

// handlers/TestPageCreateModeNewRecordArgs.al:OpenNew_TableWithRows pins xRec `B` in OnNewRecord; a table procedure the page's code calls
// on Rec sees that xRec (Sales Invoice Subform.OnNewRecord -> Rec.InitType; pinned by pages/lt-xrec-table-procedure/XRecTableProcedure.al).
#[test]
fn a_table_procedure_called_on_rec_in_a_page_trigger_sees_the_pages_xrec() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Sees() var C: TestPage \"PM Xrec Card\"; R: Record \"PM Row\"; P: Codeunit \"PM Probe\"; begin
            R.DeleteAll(); R.\"No.\" := 'A'; R.Insert(); R.\"No.\" := 'B'; R.Insert();
            P.Reset(); C.OpenNew(); Check('x=B;', P.Trace()); C.Close(); end;"
    ));
}

// misc/W2FindRecordUpdate.al:Update_False_OnOpenedPage, Update_True_OnOpenedPage (3 rows, the cursor on the middle one): a list refresh
// re-reads the row, asks `=>`, reads on to the end, steps back to the start and asks `=` (Base Application's sales subforms have
// OnFindRecord alone; there Next moves the record: assumption update-refresh-list-onfindrecord, docs/probes-needed.md).
// Without OnAfterGetCurrRecord the refresh ends after the step back and `=` is not asked (pages/lt-ai-list-order/
// AiListOrderTests.al:301 DropCurrent_RowsAfter_FindOnly, :467 DropCurrent_NoRowsAfter_FindOnly); the row is read twice before
// the action after a GoToKey (:39).
#[test]
fn current_page_update_on_a_list_with_on_find_record_alone_refreshes_it_instead_of_being_refused() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Refreshes() var C: TestPage \"PM Find List\"; R: Record \"PM Row\"; P: Codeunit \"PM Probe\"; begin
            R.DeleteAll(); R.\"No.\" := 'A'; R.Insert(); R.\"No.\" := 'B'; R.Insert(); R.\"No.\" := 'C'; R.Insert();
            C.OpenEdit(); C.GoToKey('B'); P.Reset(); C.Refresh.Invoke();
            Check('RB;RB;RB;F=>;RB;RC;RB;RA;', P.Trace());
            Check('B', C.\"No.\".Value); C.Close(); end;"
    ));
}

// A CaptionClass of '3,' with nothing after the comma (a matrix column nobody captioned) raises no event, so the page opens;
// reading that caption stays refused (P84Captions.al Page_NoSubscriber c06 shows the caption is not the field's own).
#[test]
fn an_empty_caption_class_of_area_3_does_not_stop_the_page_from_opening() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Opens() var C: TestPage \"PM Empty Class\"; R: Record \"PM Row\"; begin
            R.DeleteAll(); R.\"No.\" := 'A'; R.Insert(); C.OpenView(); Check('A', C.\"No.\".Value); C.Close(); end;"
    ));
}

// Edit Dimension Set Entries.SetFormCaption reads CurrPage.Caption on a Page variable before Dimension Management runs it
//.
#[test]
fn a_page_variable_reads_its_caption_before_it_runs() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Reads() var W: Page \"PM Caption Before\"; begin Check('Static Caption', W.Read()); end;"
    ));
}

// Check Credit Limit.PopulateDataOnNotification reaches `CurrPage.CreditLimitDetails.PAGE` on a Page variable that never ran
// (pages/lt-part-never-ran/NeverRanReadRec.al); after the page ran and closed, the same reach stays refused.
#[test]
fn a_page_variable_reaches_the_part_of_a_page_that_never_ran() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Reaches() var W: Page \"PM Never Ran Host\"; begin Check('reached', W.Reach()); end;"
    ));
}

// Posted Purchase Document Lines.CopyLineToDoc reads a part after RunModal returned: refused, not failed with the TestPage error.
#[test]
fn a_page_variable_reaching_a_part_after_the_page_closed_is_refused_not_failed() {
    let o = outcomes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('H')]
        procedure Closed() var W: Page \"PM Never Ran Host\"; begin W.RunModal(); W.Reach(); end;
        [ModalPageHandler] procedure H(var T: TestPage \"PM Never Ran Host\") begin T.OK().Invoke(); end;"
    ));
    assert!(
        matches!(&o[0].1, Outcome::NotRun(why) if why.contains("not open") && why.contains("not pinned")),
        "{o:?}"
    );
}

// Warehouse Pick Summary.SetRecords hands its part (a temporary-table page) the rows before the page runs; the part keeps them when it opens.
#[test]
fn rows_handed_to_a_part_before_the_host_runs_are_there_when_it_opens() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('H')]
        procedure Keeps() var W: Page \"PM Temp Host\"; T: Record \"PM Row\" temporary; begin
            T.\"No.\" := 'T1'; T.Descr := 'kept'; T.Insert(); W.Fill(T); W.RunModal(); end;
        [ModalPageHandler] procedure H(var P: TestPage \"PM Temp Host\") begin Check('kept', P.Sub.Descr.Value()); P.OK().Invoke(); end;"
    ));
}

// Item Tracking Summary: SetSources fills the temporary Rec of a Page variable, AutoSelectTrackingNo modifies it, RunModal runs, GetSelected reads Rec.
#[test]
fn the_temporary_rows_of_a_page_variable_survive_its_run() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('H')]
        procedure Survives() var W: Page \"PM Temp Var\"; begin
            W.Fill(); W.Mark(); W.RunModal(); Check('1', Format(W.Count())); Check('marked', W.First()); end;
        [ModalPageHandler] procedure H(var P: TestPage \"PM Temp Var\") begin Check('marked', P.Descr.Value()); P.OK().Invoke(); end;"
    ));
}

// Warehouse Pick Summary.SetRecords hands rows to its part, then Page.Run opens the page a test trapped: the trapped page has those rows.
#[test]
fn rows_handed_to_a_part_before_a_trapped_run_are_on_the_trapped_page() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Keeps() var W: Page \"PM Temp Host\"; T: Record \"PM Row\" temporary; P: TestPage \"PM Temp Host\"; begin
            T.\"No.\" := 'T1'; T.Descr := 'kept'; T.Insert(); P.Trap(); W.Fill(T); W.Run();
            Check('kept', P.Sub.Descr.Value()); P.Close(); end;"
    ));
}

// Item Tracking Summary.SetMaxQuantity stores a global that AutoSelectTrackingNo reads in a later call on the same Page variable.
#[test]
fn a_page_variable_keeps_its_globals_between_calls_before_it_runs() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Keeps() var W: Page \"PM Temp Var\"; begin W.SetMax(7); Check('7', Format(W.GetMax())); end;"
    ));
}

// Item Tracking Summary: AutoSelectTrackingNo modifies the first row and steps on; OnOpenPage then calls Rec.Modify() on the row the
// buffer holds, which must not write the first row's old values back.
#[test]
fn a_modify_in_on_open_page_writes_the_row_the_buffer_holds() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('H')]
        procedure Keeps() var W: Page \"PM Open Modify\"; begin W.Fill(); W.Mark(); W.RunModal(); end;
        [ModalPageHandler] procedure H(var P: TestPage \"PM Open Modify\") begin P.First(); Check('marked', P.Descr.Value()); P.OK().Invoke(); end;"
    ));
}

#[test]
fn calc_sums_on_the_temporary_rec_of_a_page_variable_in_on_open_page_sums_its_rows() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('H')]
        procedure Sums() var W: Page \"PM Open Modify\"; P: Codeunit \"PM Probe\"; begin P.Reset(); W.Fill(); W.Mark(); W.RunModal(); Check('sum=11;', P.Trace()); end;
        [ModalPageHandler] procedure H(var P: TestPage \"PM Open Modify\") begin P.OK().Invoke(); end;"
    ));
}

#[test]
fn calc_sums_after_set_table_view_and_set_view_in_on_open_page_sums_the_rows() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('H')]
        procedure Sums() var W: Page \"PM Open Modify\"; P: Codeunit \"PM Probe\"; S: Record \"PM Row\" temporary; begin
            P.Reset(); S.\"No.\" := 'A'; S.Rank := 1; S.Insert(); S.\"No.\" := 'B'; S.Rank := 2; S.Insert();
            W.SetTableView(S); W.FillView(S); W.RunModal(); Check('sum=3;', P.Trace()); end;
        [ModalPageHandler] procedure H(var P: TestPage \"PM Open Modify\") begin P.OK().Invoke(); end;"
    ));
}

// Item Tracking Summary: `F.SetTableView(TempSrc)` hands the page a view; the page's own temporary Rec (filled by SetSources, marked by
// AutoSelectTrackingNo) is what it opens with, not the rows of TempSrc.
#[test]
fn set_table_view_of_a_temporary_record_does_not_replace_the_rows_of_a_temporary_page() {
    passes(&format!(
        "{HELPERS}
        [Test] [HandlerFunctions('H')]
        procedure Keeps() var W: Page \"PM Open Modify\"; S: Record \"PM Row\" temporary; begin
            S.\"No.\" := 'S1'; S.Descr := 'source'; S.Insert();
            W.SetTableView(S); W.Fill(); W.Mark(); W.RunModal(); end;
        [ModalPageHandler] procedure H(var P: TestPage \"PM Open Modify\") begin P.First(); Check('T1', P.\"No.\".Value()); Check('marked', P.Descr.Value()); P.OK().Invoke(); end;"
    ));
}

// SCM RTAM Item Tracking-II sets a Code control to a Decimal (assumption setvalue-whole-number-into-text-control).
#[test]
fn set_value_of_a_whole_decimal_on_a_text_control_enters_its_digits() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Digits() var C: TestPage \"PM Card\"; D: Decimal; begin
            D := 14; C.OpenNew(); C.NoCtl.SetValue(D); Check('14', C.NoCtl.Value()); C.Close(); end;"
    ));
}

// Library - Service.CreateServiceOrderHeaderUsingPage (Microsoft's Tests-TestLibraries): OpenNew a Service Order, Activate Customer No.,
// read No., OK. About 9 Tests-SCM-Service tests failed with "The field Document No. of table Service Item Line contains a value ... that
// cannot be found in the related table (Service Header)".
#[test]
fn a_new_document_with_a_part_linked_on_its_number_opens_activates_a_field_and_closes() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure OpenNewActivateOk() var C: TestPage \"PM SH Card\"; H: Record \"PM SH\"; begin
            H.DeleteAll(); C.OpenNew(); C.\"Customer No.\".Activate(); Check('SVO001', C.\"No.\".Value()); C.OK().Invoke();
            if not H.Get(H.\"Document Type\"::\"Order\", 'SVO001') then Error('host not saved'); end;"
    ));
}

// Microsoft's Service Credit Memo, Invoice and Order pages declare OnFindRecord, and their tests open them with OpenNew
// (Tests-SCM-Service ServiceCreditMemoByPage and about 30 more were refused). The page is opened as one without the trigger.
// Pinned by pages/lt-opennew-findrecord/OpenNewFindRecord.al:150-230.
#[test]
fn open_new_on_a_page_with_on_find_record_gets_to_the_new_row() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure OpenNewWithFind() var C: TestPage \"PM OpenNew Find Card\"; R: Record \"PM Row\"; begin
            R.DeleteAll(); R.\"No.\" := 'F1'; R.Insert();
            C.OpenNew(); C.\"No.\".SetValue('F2'); C.Close();
            if not R.Get('F2') then Error('new row not stored'); end;"
    ));
}

// pages/lt-testpage-assertequals-option/TestPageAssertEqualsOption.al:AssertEquals_TextControl_NumbersAndText,
// AssertEquals_CodeControl_NumbersAndText: an Integer, a BigInteger and a Decimal compare with the control's text.
#[test]
fn assert_equals_of_numbers_on_a_code_control() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Numbers() var C: TestPage \"PM Card\"; B: BigInteger; D: Decimal; begin
            B := 5; D := 5.5; C.OpenNew(); C.NoCtl.SetValue('5');
            C.NoCtl.AssertEquals(5); C.NoCtl.AssertEquals(B);
            asserterror C.NoCtl.AssertEquals(6);
            Check('AssertEquals for Field: NoCtl Expected = ''6'', Actual = ''5''', GetLastErrorText());
            C.NoCtl.SetValue('5.5'); C.NoCtl.AssertEquals(D); end;"
    ));
}

// pages/lt-testpage-assertequals-option/TestPageAssertEqualsOption.al:SetValue_BlankOption_AsSpaceText: a text of spaces selects the
// blank member of an option control with an OptionCaption.
#[test]
fn set_value_of_a_space_on_an_option_control_selects_the_blank_member() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure SpaceText() var C: TestPage \"PM Opt Card\"; R: Record \"PM Opt Row\"; N: Integer; begin
            R.DeleteAll(); R.Id := 1; R.Opt := R.Opt::A; R.Insert();
            C.OpenEdit(); C.Opt.SetValue(' '); Check('[ ]', '[' + C.Opt.Value() + ']'); C.Close();
            R.Get(1); N := R.Opt; Check('0', Format(N)); end;"
    ));
}

// SetValue_BlankOption_AsEmptyText: the empty text is refused with TestValidation and the control then shows the ordinal.
#[test]
fn set_value_of_the_empty_text_on_an_option_control_is_refused() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure EmptyText() var C: TestPage \"PM Opt Card\"; R: Record \"PM Opt Row\"; begin
            R.DeleteAll(); R.Id := 1; R.Opt := R.Opt::A; R.Insert();
            C.OpenEdit();
            asserterror C.Opt.SetValue('');
            Check('TestValidation', GetLastErrorCode());
            Check('Validation error for Field: Opt,  Message = ''Your entry of '''' is not an acceptable value for ''Opt''. (Select Refresh to discard errors)''', GetLastErrorText());
            Check('[1]', '[' + C.Opt.Value() + ']'); end;"
    ));
}
