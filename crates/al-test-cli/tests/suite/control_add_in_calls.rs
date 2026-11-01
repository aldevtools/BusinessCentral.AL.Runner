//! Control add-ins in a test session (ADR-027 section 3 row 18). BC never creates the add-in of a `usercontrol` there:
//! from BC 28 every call of one of its procedures raises 'The control add-in on control <ctl> on page <page> has not
//! been instantiated.' (code DotNetControlAddInNotCreated, wrapped by the caller's context), its event triggers never run,
//! and the page works as usual (corpus lt-ac/ACControlAddIn.al TZ_*). Code that does not reach a call still runs (issue 34:
//! Base Application's "Item Tracking Lines" calls its barcode add-in only after the client said it is supported).

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

/// Runs codeunit `T` of `src` as BC `bc` (`None`: the newest) and answers each test's outcome.
fn run(src: &str, bc: Option<&str>) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let target = bc.map(|v| al_version::Target::for_bc(al_version::BcVersion::parse(v).unwrap()));
    let engine = Engine::with_target(l.program.clone(), blocked, target);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    run_codeunit(&engine, &l, oi)
}

fn by(o: &[(String, Outcome)], n: &str) -> Outcome {
    o.iter()
        .find(|(t, _)| t == n)
        .map(|(_, r)| r.clone())
        .unwrap_or_else(|| panic!("no test {n}"))
}

const SRC: &str = r#"
controladdin "CA Scanner" { event ControlAddInReady(IsSupported: Boolean); procedure RequestBarcodeAsync(); }
table 63500 "CA Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
page 63501 "CA Lines"
{
    PageType = List; SourceTable = "CA Row";
    layout { area(Content) {
        usercontrol(BarcodeControl; "CA Scanner") { trigger ControlAddInReady(IsSupported: Boolean) begin Available := IsSupported; end; }
        repeater(Rows) { field(Id; Rec.Id) { } } } }
    actions { area(Processing) { action(Scan) { trigger OnAction() begin if Available then CurrPage.BarcodeControl.RequestBarcodeAsync(); end; }
                                 action(ScanNow) { trigger OnAction() begin CurrPage.BarcodeControl.RequestBarcodeAsync(); end; } } }
    var Available: Boolean;
}
codeunit 63502 T
{
    Subtype = Test;
    [Test] [HandlerFunctions('Lines')] procedure OpensThePage() begin Page.RunModal(Page::"CA Lines"); end;
    [Test] procedure ScanWithoutTheAddIn() var P: TestPage "CA Lines"; begin P.OpenView(); P.Scan.Invoke(); P.Close(); end;
    [Test] procedure ScanNow() var P: TestPage "CA Lines"; begin P.OpenView(); P.ScanNow.Invoke(); end;
    [ModalPageHandler] procedure Lines(var P: TestPage "CA Lines") begin P.Scan.Invoke(); end;
}
"#;

/// A call that does not run changes nothing; one that runs raises the not-instantiated error out of the action
/// (corpus pages/lt-ao-testpage/AoTests.al:793 AddinProcedureFromAction, BC28PLUS).
#[test]
fn a_control_add_in_call_raises_the_not_instantiated_error_only_when_it_runs() {
    let o = run(SRC, None);
    assert_eq!(by(&o, "OpensThePage"), Outcome::Pass);
    assert_eq!(by(&o, "ScanWithoutTheAddIn"), Outcome::Pass);
    match by(&o, "ScanNow") {
        Outcome::Fail(why) => assert!(
            why.contains("The control add-in on control BarcodeControl on page CA Lines has not been instantiated."),
            "{why}"
        ),
        other => panic!("{other:?}"),
    }
}

/// BC 27 is not pinned (Windows BC 27.5 gave a client callback error through the test runner hub): the call refuses
/// when it runs, and the code around it still runs.
#[test]
fn before_bc_28_a_control_add_in_call_refuses_when_it_runs() {
    let o = run(SRC, Some("27.5"));
    assert_eq!(by(&o, "ScanWithoutTheAddIn"), Outcome::Pass);
    match by(&o, "ScanNow") {
        Outcome::NotRun(why) => assert!(why.contains("BarcodeControl"), "{why}"),
        other => panic!("{other:?}"),
    }
}

/// The error and its code in each place the corpus pins (lt-ac/ACControlAddIn.al, TZ fixtures in TzAddinFixtures.al):
/// a TryFunction (plain code), an action (TestWrapped:), OnOpenPage and OnAfterGetRecord during OpenEdit (plain),
/// OnAfterGetCurrRecord during OpenEdit (:TargetInvocation), OnClosePage (the closing text, TestWrapped), a field's
/// OnValidate (NSValidationFailed:), a usercontrol with Visible = false, one a page extension adds, and a page variable never
/// opened. No add-in event trigger runs.
const PLACES: &str = r#"
controladdin "TZ Addin" { procedure Say(Message: Text); event ControlAddInReady(); }
table 63520 "TZ Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
codeunit 63521 "TZ Log"
{
    SingleInstance = true;
    var Entries: Text; CallIn: Text;
    procedure Reset(NewCallIn: Text) begin Entries := ''; CallIn := NewCallIn; end;
    procedure ShouldCall(Where: Text): Boolean begin exit(CallIn = Where); end;
    procedure Add(Entry: Text) begin if Entries <> '' then Entries += ' | '; Entries += Entry; end;
    procedure Dump(): Text begin exit(Entries); end;
}
page 63522 "TZ Card"
{
    PageType = Card; SourceTable = "TZ Row";
    layout { area(Content) {
        field(NameF; Rec.Name) { trigger OnValidate() begin CurrPage.Ctl.Say(Rec.Name); end; }
        usercontrol(Ctl; "TZ Addin") { trigger ControlAddInReady() begin Log.Add('Ctl ControlAddInReady'); end; }
        usercontrol(HiddenCtl; "TZ Addin") { Visible = false; } } }
    actions { area(Processing) {
        action(CallAct) { trigger OnAction() begin Log.Add('CallAct before'); CurrPage.Ctl.Say('hello'); Log.Add('CallAct after'); end; }
        action(HiddenCallAct) { trigger OnAction() begin CurrPage.HiddenCtl.Say('hello'); end; }
        action(TryCallAct) { trigger OnAction() var Ok: Boolean; begin ClearLastError(); Ok := TrySay(); Log.Add('ok=' + Format(Ok) + ' code=' + GetLastErrorCode()); end; } } }
    trigger OnOpenPage() begin Log.Add('OnOpenPage'); CallIfAsked('OnOpenPage'); end;
    trigger OnAfterGetRecord() begin Log.Add('OnAfterGetRecord'); CallIfAsked('OnAfterGetRecord'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('OnAfterGetCurrRecord'); CallIfAsked('OnAfterGetCurrRecord'); end;
    trigger OnClosePage() begin Log.Add('OnClosePage'); CallIfAsked('OnClosePage'); end;
    local procedure CallIfAsked(Where: Text) begin if not Log.ShouldCall(Where) then exit; CurrPage.Ctl.Say(Where); Log.Add(Where + ' after call'); end;
    [TryFunction] local procedure TrySay() begin CurrPage.Ctl.Say('try'); end;
    procedure CallFromCode() begin CurrPage.Ctl.Say('code'); end;
    var Log: Codeunit "TZ Log";
}
pageextension 63523 "TZ Card Ext" extends "TZ Card"
{
    layout { addlast(Content) { usercontrol(ExtCtl; "TZ Addin") { } } }
    actions { addlast(Processing) { action(ExtCallAct) { trigger OnAction() begin CurrPage.ExtCtl.Say('ext'); end; } } }
}
codeunit 63524 T
{
    Subtype = Test;
    var Log: Codeunit "TZ Log";
    local procedure Seed(Mode: Text) var Row: Record "TZ Row"; begin Row.DeleteAll(); Row.Id := 1; Row.Name := 'one'; Row.Insert(); Log.Reset(Mode); end;
    local procedure Check(Text: Text; Code: Text)
    begin
        if GetLastErrorText() <> Text then Error('text: %1', GetLastErrorText());
        if GetLastErrorCode() <> Code then Error('code: %1', GetLastErrorCode());
    end;
    [Test] procedure OpenNavigateClose() var TP: TestPage "TZ Card"; begin
        Seed(''); TP.OpenEdit(); TP.Close();
        if Log.Dump() <> 'OnOpenPage | OnAfterGetRecord | OnAfterGetCurrRecord | OnClosePage' then Error('log: %1', Log.Dump()); end;
    [Test] procedure InOnOpenPage() var TP: TestPage "TZ Card"; begin
        Seed('OnOpenPage'); asserterror TP.OpenEdit();
        Check('The control add-in on control Ctl on page TZ Card has not been instantiated.', 'DotNetControlAddInNotCreated');
        if Log.Dump() <> 'OnOpenPage' then Error('log: %1', Log.Dump()); end;
    [Test] procedure InOnAfterGetRecord() var TP: TestPage "TZ Card"; begin
        Seed('OnAfterGetRecord'); asserterror TP.OpenEdit();
        Check('The control add-in on control Ctl on page TZ Card has not been instantiated.', 'DotNetControlAddInNotCreated'); end;
    [Test] procedure InOnAfterGetCurrRecord() var TP: TestPage "TZ Card"; begin
        Seed('OnAfterGetCurrRecord'); asserterror TP.OpenEdit();
        Check('The control add-in on control Ctl on page TZ Card has not been instantiated.', 'DotNetControlAddInNotCreated:TargetInvocation'); end;
    [Test] procedure InOnClosePage() var TP: TestPage "TZ Card"; begin
        Seed('OnClosePage'); TP.OpenEdit(); asserterror TP.Close();
        if not GetLastErrorText().StartsWith('One or more errors occurred while closing the page.') then Error('text: %1', GetLastErrorText());
        if not GetLastErrorText().EndsWith('The control add-in on control Ctl on page TZ Card has not been instantiated.') then Error('text: %1', GetLastErrorText());
        if GetLastErrorCode() <> 'TestWrapped' then Error('code: %1', GetLastErrorCode()); end;
    [Test] procedure FromAction() var TP: TestPage "TZ Card"; begin
        Seed(''); TP.OpenEdit(); ClearLastError();
        if TryInvoke(TP, 1) then Error('no error');
        Check('The control add-in on control Ctl on page TZ Card has not been instantiated.', 'TestWrapped:DotNetControlAddInNotCreated');
        if TP.NameF.Value() <> 'one' then Error('page'); end;
    [Test] procedure FromHiddenControl() var TP: TestPage "TZ Card"; begin
        Seed(''); TP.OpenEdit(); asserterror TP.HiddenCallAct.Invoke();
        Check('The control add-in on control HiddenCtl on page TZ Card has not been instantiated.', 'TestWrapped:DotNetControlAddInNotCreated'); end;
    [Test] procedure FromExtensionControl() var TP: TestPage "TZ Card"; begin
        Seed(''); TP.OpenEdit(); asserterror TP.ExtCallAct.Invoke();
        Check('The control add-in on control ExtCtl on page TZ Card has not been instantiated.', 'TestWrapped:DotNetControlAddInNotCreated'); end;
    [Test] procedure TryFunctionInsideAction() var TP: TestPage "TZ Card"; begin
        Seed(''); TP.OpenEdit(); TP.TryCallAct.Invoke();
        if not Log.Dump().Contains('ok=No code=DotNetControlAddInNotCreated |') then Error('log: %1', Log.Dump()); end;
    [Test] procedure FromFieldValidate() var TP: TestPage "TZ Card"; begin
        Seed(''); TP.OpenEdit(); asserterror TP.NameF.SetValue('changed');
        Check('The control add-in on control Ctl on page TZ Card has not been instantiated.', 'NSValidationFailed:DotNetControlAddInNotCreated'); end;
    [Test] procedure PageVariableNeverOpened() var P: Page "TZ Card"; begin
        asserterror P.CallFromCode();
        Check('The control add-in on control Ctl on page TZ Card has not been instantiated.', 'DotNetControlAddInNotCreated'); end;
    [TryFunction] local procedure TryInvoke(var TP: TestPage "TZ Card"; Which: Integer) begin TP.CallAct.Invoke(); end;
}
"#;

#[test]
fn the_not_instantiated_error_in_each_pinned_place() {
    let o = run(PLACES, None);
    let bad: Vec<_> = o.iter().filter(|(_, r)| *r != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:#?}");
    assert_eq!(o.len(), 11);
}

/// al-runner issue 6154: a page part keeps a callback add-in in a `ControlAddIn` variable and a flag guards every call. The
/// routine lowers, code that does not reach the call runs, and a call that runs refuses (what it does is not pinned).
#[test]
fn a_control_add_in_variable_call_refuses_only_when_it_runs() {
    let src = r#"
controladdin "Repro Callback" { RequestedHeight = 0; RequestedWidth = 0; procedure StatusChanged(Connected: Boolean) event StatusChangedEvent(Connected: Boolean); }
page 50100 "Repro Part"
{
    PageType = CardPart;
    var CallbackControl: ControlAddIn "Repro Callback"; CallbackControlIsSet: Boolean; Connected: Boolean;
    procedure SetCallbackControl(Control: ControlAddIn "Repro Callback") begin CallbackControlIsSet := true; CallbackControl := Control; end;
    procedure SetConnected(NewConnected: Boolean) begin
        if NewConnected = Connected then exit;
        Connected := NewConnected;
        if CallbackControlIsSet then CallbackControl.StatusChanged(Connected);
    end;
    procedure ForceCall() begin CallbackControl.StatusChanged(true); end;
    procedure IsConnected(): Boolean begin exit(Connected); end;
}
codeunit 50101 T
{
    Subtype = Test;
    [Test] procedure NotSet() var ReproPart: Page "Repro Part"; begin ReproPart.SetConnected(true); if not ReproPart.IsConnected() then Error('not connected'); end;
    [Test] procedure Called() var ReproPart: Page "Repro Part"; begin ReproPart.ForceCall(); end;
}
"#;
    let o = run(src, None);
    assert_eq!(by(&o, "NotSet"), Outcome::Pass);
    match by(&o, "Called") {
        Outcome::NotRun(why) => assert!(why.contains("ControlAddIn variable"), "{why}"),
        other => panic!("{other:?}"),
    }
}

/// The error names the page; only pages without a caption are pinned, so on a page with one the call refuses when it
/// runs.
#[test]
fn a_control_add_in_call_on_a_page_with_a_caption_refuses_when_it_runs() {
    let src = r#"
controladdin "CA Addin" { procedure Say(Message: Text); }
page 63530 "CA Captioned" { Caption = 'Something'; layout { area(Content) { usercontrol(Ctl; "CA Addin") { } } }
    procedure CallIt() begin CurrPage.Ctl.Say('x'); end; }
codeunit 63531 T { Subtype = Test; [Test] procedure Call() var P: Page "CA Captioned"; begin P.CallIt(); end; }
"#;
    match by(&run(src, None), "Call") {
        Outcome::NotRun(why) => assert!(why.contains("caption"), "{why}"),
        other => panic!("{other:?}"),
    }
}

// `CurrPage.Update(true)` on a page that has an action named Update is the page's method: an action is not called
// (System Application's "Schedule Feature Data Update" does it; real BC compiles it). Issue 34.
#[test]
fn curr_page_update_is_the_method_when_an_action_has_the_same_name() {
    let src = r#"
table 63510 "CU Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
page 63511 "CU Card"
{
    PageType = Card; SourceTable = "CU Row";
    layout { area(Content) { field(Id; Rec.Id) { } } }
    actions { area(Processing) { action(Update) { trigger OnAction() begin Done := true; CurrPage.Update(true); end; } } }
    var Done: Boolean;
}
codeunit 63512 T
{
    Subtype = Test;
    [Test] procedure UpdateAction() var R: Record "CU Row"; P: TestPage "CU Card"; begin R.Id := 1; R.Insert(); P.OpenEdit(); P.GoToRecord(R); P.Update.Invoke(); P.Close(); end;
}
"#;
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let o = run_codeunit(&engine, &l, oi);
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0]);
}
