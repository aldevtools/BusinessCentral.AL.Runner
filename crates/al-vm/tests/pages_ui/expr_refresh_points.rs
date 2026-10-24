//! A main card, two list parts and a FactBox that show one codeunit's state through an expression control: each page shows
//! the value it computed when it last refreshed (pages/lt-qj-part-propagation/QjPartPropagation.al:4629-4710, codeunit 86889;
//! al-runner issue 5765). A read after an event whose refresh is not pinned, of a value that changed since, is refused.

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 69800 "XR State"
{
    var Value: Integer;
    procedure SetValue(NewValue: Integer) begin Value := NewValue; end;
    procedure GetValue(): Integer begin exit(Value); end;
}
table 69801 "XR Param"
{
    TableType = Temporary;
    fields { field(1; "Code"; Code[20]) { } field(2; Amount; Decimal) { } }
    keys { key(PK; "Code") { Clustered = true; } }
}
page 69802 "XR Main"
{
    PageType = Card; SourceTable = "XR Param"; SourceTableTemporary = true; UsageCategory = None; ApplicationArea = All;
    layout
    {
        area(Content)
        {
            group(General) { field("Code"; Rec."Code") { } field(StateValue; State.GetValue()) { } }
            part(SubA; "XR Sub") { }
            part(SubB; "XR Sub Plain") { }
        }
        area(FactBoxes) { part(Factbox; "XR Factbox") { } }
    }
    actions
    {
        area(Processing)
        {
            action(UpdatePage) { trigger OnAction() begin CurrPage.Update(false); end; }
            action(SetState5) { trigger OnAction() begin State.SetValue(5); end; }
        }
    }
    trigger OnOpenPage()
    begin
        Rec.Copy(TempParamSource, true);
        CurrPage.SubA.Page.SetSource(Rec);
        CurrPage.SubA.Page.SetState(State);
        CurrPage.SubB.Page.SetSource(Rec);
        CurrPage.SubB.Page.SetState(State);
        CurrPage.Factbox.Page.SetSource(Rec);
        CurrPage.Factbox.Page.SetState(State);
    end;
    var TempParamSource: Record "XR Param" temporary; State: Codeunit "XR State";
    procedure SetParam(var TempParam: Record "XR Param" temporary) begin TempParamSource.Copy(TempParam, true); end;
    procedure SetState(NewState: Codeunit "XR State") begin State := NewState; end;
}
page 69803 "XR Sub"
{
    PageType = ListPart; SourceTable = "XR Param"; SourceTableTemporary = true; ApplicationArea = All;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } field(StateValue; State.GetValue()) { } } } }
    actions { area(Processing) { action(SubSet8) { trigger OnAction() begin State.SetValue(8); end; } } }
    trigger OnAfterGetCurrRecord() begin end;
    var State: Codeunit "XR State";
    procedure SetSource(var TempParam: Record "XR Param" temporary) begin Rec.Copy(TempParam, true); end;
    procedure SetState(NewState: Codeunit "XR State") begin State := NewState; end;
}
page 69804 "XR Sub Plain"
{
    PageType = ListPart; SourceTable = "XR Param"; SourceTableTemporary = true; ApplicationArea = All;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } field(StateValue; State.GetValue()) { } } } }
    var State: Codeunit "XR State";
    procedure SetSource(var TempParam: Record "XR Param" temporary) begin Rec.Copy(TempParam, true); end;
    procedure SetState(NewState: Codeunit "XR State") begin State := NewState; end;
}
page 69805 "XR Factbox"
{
    PageType = CardPart; SourceTable = "XR Param"; SourceTableTemporary = true; ApplicationArea = All;
    layout { area(Content) { field(StateValue; State.GetValue()) { } } }
    actions
    {
        area(Processing)
        {
            action(SetState42) { trigger OnAction() begin State.SetValue(42); end; }
            action(SetState42AndUpdate) { trigger OnAction() begin State.SetValue(42); CurrPage.Update(false); end; }
            action(SetState42AndUpdateTrue) { trigger OnAction() begin State.SetValue(42); CurrPage.Update(true); end; }
        }
    }
    var State: Codeunit "XR State";
    procedure SetSource(var TempParam: Record "XR Param" temporary) begin Rec.Copy(TempParam, true); end;
    procedure SetState(NewState: Codeunit "XR State") begin State := NewState; end;
}
page 69807 "XR Rec Card"
{
    PageType = Card; SourceTable = "XR Param"; SourceTableTemporary = true; UsageCategory = None; ApplicationArea = All;
    layout { area(Content) { group(G) { field("Code"; Rec."Code") { } field(Twice; Rec.Amount * 2) { } } } }
    trigger OnOpenPage() begin Rec.Code := 'A'; Rec.Amount := 1; Rec.Insert(); end;
}
codeunit 69806 T
{
    local procedure OpenMain(var MainTestPage: TestPage "XR Main"; var State: Codeunit "XR State")
    var TempParam: Record "XR Param" temporary; MainPage: Page "XR Main"; i: Integer;
    begin
        for i := 1 to 3 do begin
            TempParam.Init();
            TempParam."Code" := CopyStr('ABC', i, 1);
            TempParam.Insert();
        end;
        MainPage.SetParam(TempParam);
        MainPage.SetState(State);
        MainTestPage.Trap();
        MainPage.Run();
    end;
    local procedure Observe(var MainTestPage: TestPage "XR Main"; var State: Codeunit "XR State"): Text
    begin
        MainTestPage.SubA.First();
        exit(Read(MainTestPage, State));
    end;
    local procedure Read(var MainTestPage: TestPage "XR Main"; var State: Codeunit "XR State"): Text
    begin
        exit(StrSubstNo('test=%1;main=%2;subA=%3;subB=%4;factbox=%5', State.GetValue(), MainTestPage.StateValue.Value(),
            MainTestPage.SubA.StateValue.Value(), MainTestPage.SubB.StateValue.Value(), MainTestPage.Factbox.StateValue.Value()));
    end;

    // The reporter's FactboxAction_ChangesStateEverywhere.
    procedure FactboxAction(): Text
    var State: Codeunit "XR State"; P: TestPage "XR Main"; Observed: Text;
    begin
        OpenMain(P, State);
        Observed := 'opened:' + Read(P, State);
        P.Factbox.SetState42.Invoke();
        Observed += '|read:' + Read(P, State);
        Observed += '|afterAction:' + Observe(P, State);
        P.UpdatePage.Invoke();
        Observed += '|afterUpdate:' + Observe(P, State);
        exit(Observed);
    end;

    // The reporter's FactboxUpdate_WithoutPropagationLeavesMainStale.
    procedure FactboxUpdate(): Text
    var State: Codeunit "XR State"; P: TestPage "XR Main";
    begin
        OpenMain(P, State);
        P.Factbox.SetState42AndUpdate.Invoke();
        exit(StrSubstNo('main=%1;factbox=%2', P.StateValue.Value(), P.Factbox.StateValue.Value()) + '|' + Read(P, State));
    end;

    // The reporter's TestCodeChangesState_PageReadsLiveValue, then the corpus's moves.
    procedure TestCodeChanges(): Text
    var State: Codeunit "XR State"; P: TestPage "XR Main"; Observed: Text;
    begin
        OpenMain(P, State);
        State.SetValue(7);
        Observed := Read(P, State);
        Observed += '|' + Observe(P, State);
        P.SubA.Next();
        Observed += '|' + Read(P, State);
        P.Next();
        Observed += '|' + Read(P, State);
        State.SetValue(9);
        P.UpdatePage.Invoke();
        Observed += '|' + Read(P, State);
        exit(Observed);
    end;

    procedure MainAction(): Text
    var State: Codeunit "XR State"; P: TestPage "XR Main";
    begin
        OpenMain(P, State);
        P.SetState5.Invoke();
        exit(Read(P, State));
    end;

    // Not pinned: whether CurrPage.Update(true) in the FactBox refreshes the main page.
    procedure FactboxUpdateTrue(): Text
    var State: Codeunit "XR State"; P: TestPage "XR Main";
    begin
        OpenMain(P, State);
        P.Factbox.SetState42AndUpdateTrue.Invoke();
        exit(Format(P.StateValue.Value()));
    end;

    // Not pinned: what a list part's action refreshes.
    procedure ListPartAction(): Text
    var State: Codeunit "XR State"; P: TestPage "XR Main";
    begin
        OpenMain(P, State);
        P.SubA.SubSet8.Invoke();
        exit(Format(P.StateValue.Value()));
    end;

    // Not pinned: whether a part without OnAfterGetCurrRecord refreshes when the test moves it.
    procedure PlainPartFirst(): Text
    var State: Codeunit "XR State"; P: TestPage "XR Main";
    begin
        OpenMain(P, State);
        State.SetValue(3);
        P.SubB.First();
        exit(Format(P.SubB.StateValue.Value()));
    end;

    // An expression over the record is computed when it is read: after reads as before.
    procedure RecExpressionAfterReads(): Text
    var P: TestPage "XR Rec Card";
    begin
        P.OpenEdit();
        P.Twice.Value();
        exit(P.Twice.Value());
    end;

    // Not pinned: whether Next on a card computes an expression over the record again (assumed, flagged).
    procedure RecExpressionAfterNext(): Text
    var P: TestPage "XR Rec Card";
    begin
        P.OpenEdit();
        P.Next();
        exit(P.Twice.Value());
    end;

    // Not pinned and not assumed: Previous.
    procedure RecExpressionAfterPrevious(): Text
    var P: TestPage "XR Rec Card";
    begin
        P.OpenEdit();
        P.Previous();
        exit(P.Twice.Value());
    end;

    // A value that did not change since the refresh reads as it is, whatever happened since.
    procedure UnchangedAfterPlainPartFirst(): Text
    var State: Codeunit "XR State"; P: TestPage "XR Main";
    begin
        OpenMain(P, State);
        P.SubB.First();
        exit(Format(P.SubB.StateValue.Value()) + Format(P.StateValue.Value()));
    end;
}
"#;

#[test]
fn a_factbox_action_refreshes_only_the_factbox_until_the_main_page_updates() {
    assert_eq!(
        text(run_in(SRC, "T", "FactboxAction")),
        "opened:test=0;main=0;subA=0;subB=0;factbox=0\
         |read:test=42;main=0;subA=0;subB=0;factbox=42\
         |afterAction:test=42;main=0;subA=42;subB=0;factbox=42\
         |afterUpdate:test=42;main=42;subA=42;subB=42;factbox=42"
    );
}

#[test]
fn currpage_update_false_in_a_factbox_leaves_the_main_page_stale() {
    assert_eq!(
        text(run_in(SRC, "T", "FactboxUpdate")),
        "main=0;factbox=42|test=42;main=0;subA=0;subB=0;factbox=42"
    );
}

#[test]
fn state_set_by_test_code_shows_only_after_a_refresh() {
    assert_eq!(
        text(run_in(SRC, "T", "TestCodeChanges")),
        "test=7;main=0;subA=0;subB=0;factbox=0\
         |test=7;main=0;subA=7;subB=0;factbox=0\
         |test=7;main=0;subA=7;subB=0;factbox=0\
         |test=7;main=0;subA=7;subB=0;factbox=0\
         |test=9;main=9;subA=9;subB=9;factbox=9"
    );
}

#[test]
fn a_main_page_action_refreshes_every_page() {
    assert_eq!(
        text(run_in(SRC, "T", "MainAction")),
        "test=5;main=5;subA=5;subB=5;factbox=5"
    );
}

fn refused(r: Result<al_builtins::value::Value, Signal>) {
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn a_changed_value_after_currpage_update_true_in_a_factbox_is_refused() {
    refused(run_in(SRC, "T", "FactboxUpdateTrue"));
}

#[test]
fn a_changed_value_after_a_list_part_action_is_refused() {
    refused(run_in(SRC, "T", "ListPartAction"));
}

#[test]
fn a_changed_value_after_moving_a_part_without_onaftergetcurrrecord_is_refused() {
    refused(run_in(SRC, "T", "PlainPartFirst"));
}

#[test]
fn an_unchanged_value_reads_after_an_unpinned_event() {
    assert_eq!(text(run_in(SRC, "T", "UnchangedAfterPlainPartFirst")), "00");
}

#[test]
fn a_record_expression_reads_as_it_is_after_reads() {
    assert_eq!(text(run_in(SRC, "T", "RecExpressionAfterReads")), "2.00");
}

#[test]
fn a_record_expression_after_a_call_whose_refresh_is_not_pinned_is_refused() {
    refused(run_in(SRC, "T", "RecExpressionAfterPrevious"));
}

// Assumption page-expression-control-live-read-after-next-filter-activate (Price Lists UI.T003_SalesPriceListsDiscountsFromCustomersList).
#[test]
fn a_record_expression_after_next_reads_as_it_is_now_and_is_flagged() {
    use al_builtins::assumptions::{clear_hits, take_hits, Assumption};
    clear_hits();
    assert_eq!(text(run_in(SRC, "T", "RecExpressionAfterNext")), "2.00");
    assert!(take_hits().contains(&Assumption::PageExpressionControlLiveReadAfterNextFilterActivate));
}
