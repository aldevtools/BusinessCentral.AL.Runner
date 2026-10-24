//! Page extensions in the test client end to end: AL source → compile → run (ADR-012 follow-up).
//! Each test names the corpus test it mirrors.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_testkit::text;

const OBJECTS: &str = r#"
table 52000 "PX Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { }
        field(3; Other; Text[30]) { }
        field(4; Extra; Text[30]) { }
        field(5; Trace; Text[250]) { }
        field(6; Flag; Boolean) { }
        field(7; Descr; Text[50]) { Caption = 'Severity'; }
    }
    keys { key(PK; "No.") { } }
}
table 52001 "PX Lock" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { } } }

codeunit 52100 "PX Trace"
{
    SingleInstance = true;
    var Log: Text;
    procedure Add(T: Text) begin Log += T; end;
    procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end;
}

page 52001 "PX List"
{
    PageType = List;
    SourceTable = "PX Row";
    layout { area(Content) { repeater(Rows) {
        field("No."; Rec."No.") { }
        field(Name; Rec.Name) { }
    } } }
    actions { area(Processing) { } }
}

pageextension 52001 "PX List Ext" extends "PX List"
{
    layout
    {
        addafter(Name)
        {
            field(ExtExpression; Format(Rec."No.") + ':' + Rec.Name) { }
            field(ExtGlobalField; ExtGlobal)
            {
                trigger OnValidate() begin ExtGlobal := UpperCase(ExtGlobal) + '!'; end;
            }
        }
        addlast(Rows)
        {
            field(ExtRecField; Rec.Other) { Caption = 'Ext Other'; }
        }
    }
    actions
    {
        addlast(Processing)
        {
            action("Stamp Ext")
            {
                trigger OnAction() var T: Codeunit "PX Trace"; begin T.Add('stamp:' + Rec."No." + ';'); end;
            }
            action(Fails) { trigger OnAction() begin Error('ext action refused'); end; }
        }
    }
    trigger OnOpenPage() var T: Codeunit "PX Trace"; begin ExtGlobal := 'EXTOPENED'; T.Add('ext-open;'); end;
    trigger OnAfterGetRecord() var T: Codeunit "PX Trace"; begin T.Add('ext-agr:' + Rec."No." + ';'); end;
    var ExtGlobal: Text[20];
}

page 52002 "PX Card"
{
    PageType = Card;
    SourceTable = "PX Row";
    layout { area(Content) {
        field(NoCtl; Rec."No.") { }
        field(Name; Rec.Name)
        {
            trigger OnValidate() begin Rec.Trace := Rec.Trace + 'page;'; end;
            trigger OnAssistEdit() begin Rec.Trace := Rec.Trace + 'baseassist;'; end;
        }
        field(Other; Rec.Other) { trigger OnValidate() begin Rec.Trace := Rec.Trace + 'otherpage;'; end; }
        field(Extra; Rec.Extra) { }
        field(Trace; Rec.Trace) { Editable = false; }
        field(Plain; Rec.Descr) { }
        field(FlagCtl; Rec.Flag) { Editable = BaseEditable; }
        field(Gate; Rec.Flag) { Editable = BaseGate; }
    } }
    actions { area(Processing) {
        action(DoIt) { trigger OnAction() var T: Codeunit "PX Trace"; begin T.Add('doit;'); end; }
        action(GateAct) { Enabled = BaseGate; trigger OnAction() begin end; }
        action(ForExtRef) { trigger OnAction() var T: Codeunit "PX Trace"; begin T.Add('base-via-ext;'); end; }
    } area(Promoted) {
        actionref(DoIt_Promoted; DoIt) { }
        group(Category_Process) { actionref(DoIt_Grouped; DoIt) { } }
    } }
    var BaseEditable: Boolean; BaseGate: Boolean;
    trigger OnOpenPage() var T: Codeunit "PX Trace"; begin BaseEditable := true; T.Add('base-open;'); end;
}

pageextension 52002 "PX Card Ext" extends "PX Card"
{
    layout
    {
        modify(Name)
        {
            trigger OnBeforeValidate() begin
                Rec.Trace := Rec.Trace + 'before;';
                if Rec.Name = 'stop' then Error('stopped in OnBeforeValidate');
            end;
            trigger OnAfterValidate() begin Rec.Trace := Rec.Trace + 'after;'; end;
        }
        modify(Plain) { Caption = 'Modified Plain'; }
        modify(FlagCtl) { Editable = not Locked; }
        modify(Gate) { Editable = not Locked; }
        modify(Extra)
        {
            trigger OnDrillDown() begin Rec.Trace := Rec.Trace + 'extdrill;'; end;
            trigger OnAssistEdit() begin Rec.Trace := Rec.Trace + 'extassist;'; end;
            trigger OnLookup(var Text: Text): Boolean begin Rec.Trace := Rec.Trace + 'extlookup;'; exit(false); end;
        }
    }
    actions
    {
        modify(GateAct) { Enabled = not Locked; }
        addlast(Processing) { action(StampExt) { trigger OnAction() var T: Codeunit "PX Trace"; begin T.Add('ext:' + Rec."No." + ';'); end; } }
        addlast(Promoted)
        {
            actionref(StampExt_Promoted; StampExt) { }
            actionref(StampBase_Promoted; ForExtRef) { }
        }
    }
    var Locked: Boolean;
    trigger OnAfterGetCurrRecord()
    var L: Record "PX Lock";
    begin
        Locked := L.Get(Rec."No.");
    end;
}

pageextension 52003 "PX Card Other Ext" extends "PX Card"
{
    layout
    {
        modify(Other)
        {
            trigger OnBeforeValidate() begin Rec.Trace := Rec.Trace + 'otherbefore;'; end;
            trigger OnAfterValidate() begin Rec.Trace := Rec.Trace + 'otherafter;'; end;
        }
    }
}

page 52010 "PX Part"
{
    PageType = CardPart;
    layout { area(Content) { field(Msg; Message) { } } }
    var Message: Text;
    trigger OnOpenPage() var T: Codeunit "PX Trace"; begin T.Add('part-open;'); end;
}
page 52011 "PX Host"
{
    PageType = Card;
    SourceTable = "PX Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
}
pageextension 52011 "PX Host Ext" extends "PX Host"
{
    layout { addlast(Content) {
        part(ExtPart; "PX Part") { }
        part(ExtHiddenPart; "PX Part") { Visible = false; }
    } }
}

page 52012 "PX Host Two"
{
    PageType = Card;
    SourceTable = "PX Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } part(Sub; "PX Sub") { } } }
    trigger OnOpenPage() begin CurrPage.Sub.Page.Mark('base'); end;
}
page 52013 "PX Sub"
{
    PageType = CardPart;
    layout { area(Content) { field(Msg; Message) { } } }
    var Message: Text;
    procedure Mark(What: Text) var T: Codeunit "PX Trace"; begin T.Add('sub-' + What + ';'); end;
}
pageextension 52013 "PX Sub Ext" extends "PX Sub"
{
    layout { addlast(Content) { field(OwnCtl; Own) { } } }
    var Own: Text;
    procedure MarkExt(What: Text) var T: Codeunit "PX Trace"; begin Own := What; T.Add('subext-' + Own + ';'); end;
}
pageextension 52014 "PX Host Two Ext" extends "PX Host Two"
{
    trigger OnOpenPage() begin CurrPage.Sub.Page.MarkExt('ext'); end;
}
page 52020 "PX Twice"
{
    PageType = Card;
    SourceTable = "PX Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
}
pageextension 52021 "PX Twice A" extends "PX Twice" { trigger OnOpenPage() var T: Codeunit "PX Trace"; begin T.Add('twice-a;'); end; }
pageextension 52022 "PX Twice B" extends "PX Twice" { trigger OnOpenPage() var T: Codeunit "PX Trace"; begin T.Add('twice-b;'); end; }
page 52025 "PX Closing"
{
    PageType = Card;
    SourceTable = "PX Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
}
pageextension 52026 "PX Closing Ext" extends "PX Closing" { trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin exit(true); end; }
page 52030 "PX Curr"
{
    PageType = Card;
    SourceTable = "PX Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
}
pageextension 52030 "PX Curr Ext" extends "PX Curr" { trigger OnOpenPage() begin CurrPage.Editable(false); end; }
table 52040 "PX Area Row" { fields { field(1; "Code"; Code[20]) { } field(2; Basic; Text[30]) { } field(3; Service; Text[30]) { } field(4; Ext; Text[30]) { } } keys { key(PK; "Code") { } } }
page 52040 "PX Area Card"
{
    PageType = Card;
    SourceTable = "PX Area Row";
    ApplicationArea = Basic;
    layout { area(Content) {
        field(CodeCtl; Rec."Code") { }
        field(BasicCtl; Rec.Basic) { ApplicationArea = Basic; }
        field(ServiceCtl; Rec.Service) { ApplicationArea = Service; }
        field(EveryCtl; Rec.Service) { ApplicationArea = All; }
        field(ModCtl; Rec.Service) { ApplicationArea = Service; }
    } }
    actions { area(Processing) {
        action(BasicAct) { trigger OnAction() var T: Codeunit "PX Trace"; begin T.Add('basic;'); end; }
        action(ServiceAct) { ApplicationArea = Service; trigger OnAction() var T: Codeunit "PX Trace"; begin T.Add('service;'); end; }
    } area(Promoted) {
        actionref(BasicRef; BasicAct) { }
        actionref(ServiceRef; ServiceAct) { }
    } }
}
pageextension 52040 "PX Area Ext" extends "PX Area Card"
{
    layout
    {
        addlast(Content)
        {
            field(ExtBasicCtl; Rec.Ext) { ApplicationArea = Basic; }
            field(ExtNoAreaCtl; Rec.Ext) { }
            part(ExtServicePart; "PX Part") { ApplicationArea = Service; }
            part(ExtBasicPart; "PX Part") { ApplicationArea = Basic; }
        }
        modify(ModCtl) { ApplicationArea = Suite; }
    }
}
page 52024 "PX Missing"
{
    PageType = Card;
    SourceTable = "PX Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
}
pageextension 52023 "PX Missing Target" extends "PX Missing" { layout { addlast(NoSuchGroup) { field(X; Rec.Name) { } } } }
"#;

fn compile(apps: Vec<Vec<(String, String)>>) -> (al_sema::workspace::Workspace, al_sema::lower::Lowered) {
    let n = apps.len();
    let apps = apps
        .into_iter()
        .enumerate()
        .map(|(i, files)| AppSource {
            id: format!("app{i}"),
            name: format!("app{i}"),
            // Each app depends on the one before it.
            dependencies: if i == 0 { vec![] } else { vec![format!("app{}", i - 1)] },
            files,
        })
        .collect::<Vec<_>>();
    assert_eq!(apps.len(), n);
    al_sema::lower::compile(apps, &Default::default())
}

fn run_apps(apps: Vec<Vec<(String, String)>>, routine: &str) -> Result<Value, Signal> {
    let (ws, l) = compile(apps);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![(oi, ri)];
    while let Some((o, r)) = stack.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        if let Some(why) = l.unsupported[o][r].first() {
            panic!(
                "{}.{} is blocked: {why}",
                l.program.objects[o].name, l.program.objects[o].routines[r].name
            );
        }
        stack.extend(l.calls[o][r].iter().copied());
    }
    let mut s = al_testkit::session(l);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn run(body: &str, routine: &str) -> Result<Value, Signal> {
    let src = format!("{OBJECTS}\ncodeunit 52200 C {{ {body} }}");
    run_apps(vec![vec![("f.al".into(), src)]], routine)
}

/// The reason the first routine of codeunit C that declares a TestPage of `page` is refused.
fn refusal(body: &str) -> String {
    let src = format!("{OBJECTS}\ncodeunit 52200 C {{ {body} }}");
    let (_, l) = compile(vec![vec![("f.al".into(), src)]]);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    l.unsupported[oi]
        .iter()
        .find_map(|r| r.first().cloned())
        .expect("a refusal")
}

const SEED: &str = "R.\"No.\" := 'A'; R.Name := 'Alpha'; R.Other := 'o1'; R.Insert();
    R.\"No.\" := 'B'; R.Name := 'Bravo'; R.Other := 'o2'; R.Insert();";

// pageextensiontrigger/TestPageExtensionPageTriggers_Tests.al:
// PageextensionOnOpenPageRunsOnceWhenTheExtendedPageOpens, ...OnAfterGetRecordRunsForEachRowTheTestPageVisits,
// ...OnOpenPageIsRaisedBeforeTheFirstOnAfterGetRecord
#[test]
fn an_extension_trigger_runs_in_the_pages_context_for_each_row() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PX Row\"; L: TestPage \"PX List\"; T: Codeunit \"PX Trace\"; begin
                {SEED}
                L.OpenView(); L.First(); L.Next(); L.Close();
                exit(T.Take());
            end;"
        ),
        "F",
    );
    // A list with OnAfterGetRecord alone reads its window and then the current row once more, whether the view ends inside the window
    // or not (handlers/TestPageRuntimeContracts.al:121-136; pages/lt-testpage-list-rows/ListRowsOpenAG.al:OpenView_OnlyRec5Rows_ReadsAtOpen).
    assert_eq!(text(r), "ext-open;ext-agr:A;ext-agr:B;ext-agr:A;");
}

// pageextensioncontrol/TestPageExtensionControlTrapped_Tests.al: TrappedPageRun_ExtExpressionControl_ReadsTheCurrentRow,
// _ExtGlobalControl_ReadsTheValueItsOnOpenPageAssigned, _ExtControlOnValidate_UpdatesTheGlobalTheControlShows;
// pageextensioncontrol/TestPageExtensionControlBinding_Tests.al (a control bound to Rec from the extension)
#[test]
fn extension_controls_are_bound_to_rec_to_an_expression_and_to_the_extensions_own_global() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PX Row\"; L: TestPage \"PX List\"; X: Text; begin
                {SEED}
                L.OpenView();
                L.First();
                X := L.ExtExpression.Value() + '|' + L.ExtRecField.Value() + '|' + L.ExtGlobalField.Value() + '|';
                L.Next();
                X += L.ExtRecField.Value();
                L.Close();
                exit(X);
            end;"
        ),
        "F",
    );
    assert_eq!(text(r), "A:Alpha|o1|EXTOPENED|o2");
}

// The corpus pins the expression after First() (TrappedPageRun_ExtExpressionControl_ReadsTheCurrentRow); whether Next() on a
// list computes it again is not pinned (al-vm pages/shown.rs).
#[test]
fn an_extension_expression_control_after_next_is_refused() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PX Row\"; L: TestPage \"PX List\"; begin
                {SEED}
                L.OpenView();
                L.First();
                L.Next();
                exit(L.ExtExpression.Value());
            end;"
        ),
        "F",
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn an_extension_controls_on_validate_runs_on_the_extensions_instance() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PX Row\"; L: TestPage \"PX List\"; X: Text; begin
                {SEED}
                L.OpenEdit();
                L.ExtGlobalField.SetValue('typed');
                X := L.ExtGlobalField.Value();
                L.Close();
                exit(X);
            end;"
        ),
        "F",
    );
    assert_eq!(text(r), "TYPED!");
}

// handlers/TestPageExtensionActionInvoke_Tests.al: ExtActionInvokeRunsAgainstThePagesCurrentRow,
// ExtActionInvokePropagatesAnErrorRaisedInsideOnAction; handlers/TestPageSpacedActionName_Tests.al: SpacedExtActionInvokeRunsItsTrigger
#[test]
fn an_extension_action_runs_its_trigger_against_the_current_row() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PX Row\"; L: TestPage \"PX List\"; T: Codeunit \"PX Trace\"; X: Text; begin
                {SEED}
                L.OpenEdit();
                L.First(); L.Next();
                L.\"Stamp Ext\".Invoke();
                X := T.Take();
                asserterror L.Fails.Invoke();
                exit(X + '|' + GetLastErrorText());
            end;"
        ),
        "F",
    );
    assert_eq!(
        text(r),
        // The page reads the current row again after an action (corpus handlers/
        // TestPageExtensionBaseMembersContracts.al:195-218), so the extension's OnAfterGetRecord runs.
        "ext-open;ext-agr:A;ext-agr:B;ext-agr:A;stamp:B;ext-agr:B;|ext action refused"
    );
}

// pageextensiontrigger/TestModifiedControlValidateTriggers_Tests.al:
// ModifiedControlBeforeAndAfterValidateWrapTheBaseControlsOwnOnValidate, TheOtherControlsOwnModifyBlockWrapsItsOwnBaseTrigger,
// AModifyBlockOnAnotherControlIsNotRaisedForThisOne
#[test]
fn modify_before_and_after_validate_wrap_the_base_controls_on_validate() {
    let r = run(
        "procedure F(): Text var C: TestPage \"PX Card\"; X: Text; begin
            C.OpenNew();
            C.NoCtl.SetValue('N1');
            C.Name.SetValue('value');
            X := C.Trace.Value() + '|';
            C.Other.SetValue('v');
            X += C.Trace.Value();
            C.Close();
            exit(X);
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "before;page;after;|before;page;after;otherbefore;otherpage;otherafter;"
    );
}

// TestModifiedControlValidateTriggers_Tests.al: AnErrorInOnBeforeValidateDiscardsThatTriggersOwnRecMutation,
// TheRaisingTriggerDidRunEvenThoughItsMutationIsDiscarded, AWriteAfterARefusedOneTracesFromTheRestoredBuffer
#[test]
fn an_error_in_on_before_validate_discards_its_mutation_and_the_next_write_traces_cleanly() {
    let r = run(
        "procedure F(): Text var C: TestPage \"PX Card\"; X: Text; begin
            C.OpenNew();
            C.NoCtl.SetValue('N2');
            asserterror C.Name.SetValue('stop');
            X := '[' + C.Trace.Value() + ']' + GetLastErrorText() + '|';
            C.Name.SetValue('value');
            X += C.Trace.Value();
            exit(X);
        end;",
        "F",
    );
    let t = text(r);
    assert!(t.starts_with("[]"), "{t}");
    assert!(t.contains("stopped in OnBeforeValidate"), "{t}");
    assert!(t.ends_with("|before;page;after;"), "{t}");
}

// TestModifiedControlValidateTriggers_Tests.al: AModifyBlocksOnDrillDownRunsForTheModifiedControl,
// AModifyBlocksOnLookupRunsForTheModifiedControl, ABaseControlsOwnOnAssistEditRuns, AModifyBlocksOnAssistEditRunsForTheModifiedControl,
// OnAssistEditIsRaisedOnlyForTheControlItWasCalledOn, OnAssistEditRunsOncePerCall
#[test]
fn drill_down_lookup_and_assist_edit_run_the_triggers_of_the_control_they_are_called_on() {
    let r = run(
        "procedure F(): Text var C: TestPage \"PX Card\"; X: Text; begin
            C.OpenNew();
            C.NoCtl.SetValue('N3');
            C.Extra.DrillDown();
            C.Extra.Lookup();
            C.Name.AssistEdit();
            C.Name.AssistEdit();
            C.Extra.AssistEdit();
            X := C.Trace.Value();
            C.Close();
            exit(X);
        end;",
        "F",
    );
    assert_eq!(text(r), "extdrill;extlookup;baseassist;baseassist;extassist;");
}

// handlers/TestPageExtensionModifyCaption_Tests.al: SamePage_ModifiedCaption_OnRecordControl_WinsOverFieldCaption;
// handlers/TestPageExtensionControlCaption_Tests.al: ExtRecordFieldControl_DeclaredCaption_WinsOverFieldCaption
#[test]
fn a_modified_or_declared_caption_wins_over_the_field_caption() {
    let r = run(
        "procedure F(): Text var C: TestPage \"PX Card\"; L: TestPage \"PX List\"; X: Text; begin
                C.OpenNew();
                X := C.Plain.Caption() + '|';
                C.Close();
                L.OpenView();
                X += L.ExtRecField.Caption();
                L.Close();
                exit(X);
            end;",
        "F",
    );
    assert_eq!(text(r), "Modified Plain|Ext Other");
}

// pageextensioncontrol/TestPageExtensionModifyEditable_Tests.al:
// SourcePageExtModifyEditable_SourcePage_FollowsTheExtensionsVariablePerRow, ..._ExtensionReplacesTheBase
#[test]
fn a_modified_editable_or_enabled_follows_the_extensions_variable_and_replaces_the_base() {
    let r = run(
        "procedure F(): Text var R: Record \"PX Row\"; K: Record \"PX Lock\"; C: TestPage \"PX Card\"; X: Text; begin
            R.\"No.\" := 'LOCKED'; R.Insert(); R.\"No.\" := 'OPEN'; R.Insert();
            K.Code := 'LOCKED'; K.Insert();
            C.OpenEdit();
            R.Get('LOCKED'); C.GoToRecord(R);
            X := Format(C.Plain.Editable()) + Format(C.FlagCtl.Editable()) + Format(C.GateAct.Enabled()) + '|';
            R.Get('OPEN'); C.GoToRecord(R);
            X += Format(C.Plain.Editable()) + Format(C.FlagCtl.Editable()) + Format(C.Gate.Editable()) + Format(C.GateAct.Enabled());
            C.Close();
            exit(X);
        end;",
        "F",
    );
    assert_eq!(text(r), "YesNoNo|YesYesYesYes");
}

// handlers/TestPagePromotedActionref_Tests.al: PromotedActionrefInvokeRunsItsTargetsTrigger, PromotedActionrefInsideAGroupRunsItsTargetsTrigger,
// PageExtensionPromotedActionrefRunsItsOwnExtensionActionsTrigger, PageExtensionPromotedActionrefRunsABasePageActionsTrigger
#[test]
fn an_actionref_invokes_its_targets_trigger_whether_base_or_extension_declares_either() {
    let r = run(
        "procedure F(): Text var C: TestPage \"PX Card\"; T: Codeunit \"PX Trace\"; begin
            C.OpenNew();
            C.DoIt_Promoted.Invoke();
            C.DoIt_Grouped.Invoke();
            C.StampExt_Promoted.Invoke();
            C.StampBase_Promoted.Invoke();
            C.Close();
            exit(T.Take());
        end;",
        "F",
    );
    assert_eq!(text(r), "base-open;doit;doit;ext:;base-via-ext;");
}

// pageextensionpart/TestPageExtensionPart.al: SourceHost_ExtensionPart_OpensWithTheHost, SourceHost_ExtensionPartVisibleFalse_IsNotOnThePage
#[test]
fn a_part_an_extension_adds_opens_with_its_host_and_hides_when_not_visible() {
    let r = run(
        "procedure F(): Text var H: TestPage \"PX Host\"; T: Codeunit \"PX Trace\"; X: Text; B: Boolean; begin
            H.OpenView();
            X := Format(H.ExtPart.Visible()) + '|' + T.Take() + '|';
            asserterror B := H.ExtHiddenPart.Visible();
            X += GetLastErrorText();
            H.Close();
            exit(X);
        end;",
        "F",
    );
    let t = text(r);
    assert!(t.starts_with("Yes|part-open;|"), "{t}");
    assert!(t.contains("was not found on the page"), "{t}");
}

// pageextensiontrigger/TestPageExtQualifiedExtends_Tests.al: ImportedAndQualifiedExtendsBothRunForAUniquelyNamedPage
// (two extensions declare OnOpenPage; both run once, the test states no order between them).
#[test]
fn two_extensions_declaring_the_same_page_trigger_both_run_in_application_order() {
    let r = run(
        "procedure F(): Text var P: TestPage \"PX Twice\"; T: Codeunit \"PX Trace\"; begin P.OpenView(); P.Close(); exit(T.Take()); end;",
        "F",
    );
    assert_eq!(text(r), "twice-a;twice-b;");
}

// A trigger that answers a Boolean cannot be combined with the page's own answer: not pinned, so refused. (OnQueryClosePage is
// pinned: handlers/TestPageCloseTriggersSelectionCardPageContracts.al; OnInsertRecord is not.)
#[test]
fn an_extension_trigger_that_answers_a_boolean_is_refused() {
    let why = refusal("procedure F() var P: TestPage \"PX Closing\"; begin P.OpenView(); end;");
    assert!(why.contains("OnInsertRecord") && why.contains("not supported"), "{why}");
}

#[test]
fn an_extension_target_that_the_page_does_not_have_is_refused() {
    let why = refusal("procedure F() var P: TestPage \"PX Missing\"; begin P.OpenView(); end;");
    assert!(why.contains("NoSuchGroup"), "{why}");
}

const TWO_APPS_BASE: &str = r#"
table 52300 "PX Dep Row" { fields { field(1; "No."; Code[20]) { } field(2; Name; Text[30]) { } } keys { key(PK; "No.") { } } }
page 52300 "PX Dep Page"
{
    PageType = Card;
    SourceTable = "PX Dep Row";
    layout { area(Content) {
        field(Both; Rec.Name) { Caption = 'Host Both'; }
        field(Twice; Rec.Name) { Caption = 'Host Twice'; }
        field(DepOnly; Rec.Name) { Caption = 'Host Dep'; }
    } }
}
pageextension 52301 "PX Dep Ext" extends "PX Dep Page"
{
    layout { modify(Both) { Caption = 'Dependency Both'; } modify(DepOnly) { Caption = 'Dependency Own'; } }
}
"#;

const TWO_APPS_MAIN: &str = r#"
// A lower object id than the dependency's extension, and still the one that wins.
pageextension 52300 "PX Main Low Ext" extends "PX Dep Page"
{
    layout { modify(Both) { Caption = 'Main Both'; } }
}
// Two extensions of this app modify one control: the higher object id wins, whatever the order written.
pageextension 52399 "PX Main High" extends "PX Dep Page"
{
    layout { modify(Twice) { Caption = 'High Id Twice'; } }
}
pageextension 52302 "PX Main Low" extends "PX Dep Page"
{
    layout { modify(Twice) { Caption = 'Low Id Twice'; } }
}
codeunit 52301 C
{
    procedure F(): Text
    var P: TestPage "PX Dep Page";
    begin
        P.OpenNew();
        exit(P.Both.Caption() + '|' + P.Twice.Caption() + '|' + P.DepOnly.Caption());
    end;
}
"#;

// handlers/TestPageExtensionModifyCaption_Tests.al: DependencyPage_ModifiedByBothApps_ThisAppWins_EvenWithTheLowerId (FxBoth2Ctl),
// DependencyPage_ModifiedCaption_FromItsOwnApp, SamePage_TwoExtensionsOfOneApp_HigherIdWins_WhateverTheOrder (Twice2Ctl)
#[test]
fn the_app_that_depends_on_the_other_wins_a_modify_and_within_an_app_the_higher_id_does() {
    let r = run_apps(
        vec![
            vec![("dep.al".into(), TWO_APPS_BASE.into())],
            vec![("main.al".into(), TWO_APPS_MAIN.into())],
        ],
        "F",
    );
    assert_eq!(text(r), "Main Both|High Id Twice|Dependency Own");
}

// pageextensioncontrol/TestPageExtensionControlTrapped_Tests.al: TrappedPageRun_UndeclaredControlId_IsNotFound;
// autoformat/TestAutoFormat.al:135 AutoFormat_GetFieldByTableFieldNo_Errors
#[test]
fn get_field_of_an_id_the_page_does_not_declare_is_not_found() {
    let r = run(
        "procedure F(): Text var L: TestPage \"PX List\"; X: Text; begin
            L.OpenView();
            asserterror X := L.GetField(12345).Value();
            exit(GetLastErrorText());
        end;",
        "F",
    );
    assert_eq!(text(r), "The field with ID = 12345 is not found on the page.");
}

// handlers/TestPageExtensionActionInvoke_Tests.al (the extension's code runs in the page's context) with
// handlers/TestPageControlEditableRoutes.al (CurrPage.Editable(false) in OnOpenPage): an extension reaches the page through CurrPage.
#[test]
fn an_extensions_code_reaches_the_page_through_curr_page() {
    let r = run(
        "procedure F(): Text var R: Record \"PX Row\"; H: TestPage \"PX Curr\"; X: Text; begin
            R.\"No.\" := 'A'; R.Insert();
            H.OpenEdit();
            X := Format(H.NoCtl.Editable());
            H.Close();
            exit(X);
        end;",
        "F",
    );
    assert_eq!(text(r), "No");
}

fn found(ctl: &str, areas_set: &str) -> bool {
    let probe = format!(
        "procedure F(): Text var C: TestPage \"PX Area Card\"; begin
            ApplicationArea('{areas_set}');
            C.OpenView();
            exit(C.{ctl}.Value());
        end;"
    );
    match run(&probe, "F") {
        Ok(_) => true,
        Err(Signal::Error(e)) => {
            assert!(e.message.contains("is not found on the page."), "{}", e.message);
            false
        }
        other => panic!("{other:?}"),
    }
}

// pageapplicationarea/TestPageApplicationAreaControlRemoval.al: AreaNotEnabled_ControlIsNotFound,
// AreaNotEnabled_EnabledControlIsStillFoundAndWrites, AreaEnabled_ControlIsFoundAndWrites, EmptyAreaString_EveryControlIsFound
#[test]
fn a_control_whose_area_is_not_enabled_is_not_on_the_page() {
    assert!(!found("ServiceCtl", "#Basic,#Suite"));
    assert!(found("BasicCtl", "#Basic,#Suite"));
    assert!(found("ServiceCtl", "#Basic,#Suite,#Service"));
    assert!(found("ServiceCtl", ""));
}

// pageapplicationarea/TestPageApplicationAreaExtensionControlRemoval.al: ExtNoArea_DoesNotInheritPageArea_IsNotFound
// (the page's own control without an area inherits the page's), ExtOwnArea_NotEnabled_IsNotFound,
// ModifiedArea_ModifyValueEnabled_IsFoundAndWrites, ModifiedArea_BaseValueNoLongerApplies_IsNotFound,
// ExtNoArea_EmptyAreaString_IsFoundAndWrites
#[test]
fn extension_controls_inherit_no_page_area_and_a_modify_replaces_the_area() {
    assert!(found("CodeCtl", "#Basic"));
    assert!(!found("ExtNoAreaCtl", "#Basic"));
    assert!(found("ExtNoAreaCtl", ""));
    assert!(found("ExtBasicCtl", "#Basic,#Suite"));
    // The modify moved ModCtl from #Service to #Suite.
    assert!(found("ModCtl", "#Basic,#Suite"));
    assert!(!found("ModCtl", "#Basic,#Service"));
    // `ApplicationArea = All` is on every page.
    assert!(found("EveryCtl", "#Basic"));
}

// pageapplicationarea/TestPageApplicationAreaActionRemoval.al, ...ExtensionActionPartRemoval.al:
// an action is removed like a control; an actionref follows its target unless it states an area of its own.
#[test]
fn actions_and_actionrefs_follow_the_areas_and_parts_are_removed() {
    let r = run(
        "procedure F(): Text var C: TestPage \"PX Area Card\"; T: Codeunit \"PX Trace\"; X: Text; B: Boolean; begin
            ApplicationArea('#Basic,#Suite');
            C.OpenView();
            C.BasicAct.Invoke();
            C.BasicRef.Invoke();
            asserterror C.ServiceAct.Invoke();
            X := GetLastErrorText() + '|';
            asserterror C.ServiceRef.Invoke();
            X += GetLastErrorText() + '|' + Format(C.BasicRef.Visible()) + '|';
            asserterror B := C.ExtServicePart.Visible();
            X += GetLastErrorText() + '|' + Format(C.ExtBasicPart.Visible()) + '|' + T.Take();
            ApplicationArea('');
            exit(X);
        end;",
        "F",
    );
    let t = text(r);
    assert!(
        t.starts_with(
            // The ids are FNV-1a of the declaring object id and the control name (pages.rs `control_id`).
            "The action with ID = 16714197 is not found on the page.|The action with ID = 681589962 is not found on the page.|"
        ),
        "{t}"
    );
    assert!(
        t.contains("was not found on the page.|Yes|part-open;basic;basic;"),
        "{t}"
    );
}

// pageapplicationarea/TestPageDiscardedFieldValueCall.al: RemovedControl_DiscardedValueCall_RaisesNothing,
// FoundControl_DiscardedValueCall_RaisesNothing
#[test]
fn a_discarded_value_call_is_not_evaluated() {
    let r = run(
        "procedure F(): Text var C: TestPage \"PX Area Card\"; begin
            ApplicationArea('#Basic');
            C.OpenView();
            C.ServiceCtl.Value();
            C.BasicCtl.Value();
            ApplicationArea('');
            exit('ok');
        end;",
        "F",
    );
    assert_eq!(text(r), "ok");
}

// The areas changing under an open page is not pinned: refused.
#[test]
fn changing_the_areas_while_a_page_is_open_is_refused() {
    let r = run(
        "procedure F(): Text var C: TestPage \"PX Area Card\"; begin
            C.OpenView();
            ApplicationArea('#Basic');
            exit(C.BasicCtl.Value());
        end;",
        "F",
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

// handlers/TestPagePartAgcr_Tests.al (a host reaches its part's page through CurrPage.<Part>.Page) with
// pageextensioncontrol/TestPageExtensionControlBinding_Page.al (a procedure of the part page's extension runs on the
// extension's own instance, where its variables live).
#[test]
fn a_procedure_of_a_part_pages_extension_runs_on_the_extensions_instance() {
    let r = run(
        "procedure F(): Text var H: TestPage \"PX Host Two\"; T: Codeunit \"PX Trace\"; X: Text; begin
            H.OpenView();
            X := H.Sub.OwnCtl.Value() + '|';
            H.Close();
            exit(X + T.Take());
        end;",
        "F",
    );
    assert_eq!(text(r), "ext|sub-base;subext-ext;");
}
