//! Page behaviors the test client does not implement are refused, never run as if the property were not there
//! (hard rule 4). Each test names the corpus test that pins what BC does.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn refused(r: Result<Value, Signal>, needle: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains(needle), "{why}"),
        other => panic!("expected a refusal naming {needle}, got {other:?}"),
    }
}

/// pageext/lt-page-ext-properties/PageFieldValuesAllowed.al:64-73: a value outside the control's ValuesAllowed is rejected with
/// a TestValidation error that names the entry (and the card's Refresh hint).
#[test]
fn set_value_on_a_control_with_values_allowed_rejects_a_value_outside_it() {
    let src = r#"
table 69990 "VA Row" { fields { field(1; Id; Integer) { } field(2; Kind; Option) { OptionMembers = Open,Released,Closed; } } keys { key(PK; Id) { } } }
page 69991 "VA Card"
{
    PageType = Card; SourceTable = "VA Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Kind; Rec.Kind) { ValuesAllowed = Open, Released; } field(Id; Rec.Id) { } } }
}
codeunit 69992 C
{
    procedure F(): Text
    var R: Record "VA Row"; P: TestPage "VA Card";
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        P.Kind.SetValue('Closed');
        exit(P.Kind.Value());
    end;
    procedure G(): Text
    var R: Record "VA Row"; P: TestPage "VA Card";
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        exit(P.Kind.Value());
    end;
}
"#;
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(src, "C", "F")),
        "Validation error for Field: Kind,  Message = 'Your entry of 'Closed' is not an acceptable value for 'Kind'. (Select Refresh to discard errors)'"
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "G")), "Open");
}

/// pages/lt-page-presentation-properties/P119LabelHideValue.al:74: `HideValue = <variable>` hides the value, so HideValue is
/// evaluated like Visible, Editable and Enabled. Another boolean property written as an expression is refused, not read as false.
#[test]
fn a_boolean_control_property_written_as_an_expression_is_evaluated_or_refused() {
    let src = r#"
table 69993 "HV Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[20]) { } } keys { key(PK; Id) { } } }
page 69994 "HV Card"
{
    PageType = Card; SourceTable = "HV Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(NameF; Rec.Name) { HideValue = HideIt; ShowMandatory = MandIt; } field(Plain; Rec.Name) { HideValue = true; } } }
    var HideIt: Boolean; MandIt: Boolean;
    trigger OnOpenPage() begin HideIt := true; MandIt := true; end;
}
codeunit 69995 C
{
    procedure F(): Text
    var R: Record "HV Row"; P: TestPage "HV Card";
    begin
        R.Id := 1; R.Name := 'Shown?'; R.Insert();
        P.OpenView();
        exit('[' + P.NameF.Value() + ']');
    end;
    procedure G(): Text
    var R: Record "HV Row"; P: TestPage "HV Card";
    begin
        R.Id := 1; R.Name := 'Shown?'; R.Insert();
        P.OpenView();
        exit(Format(P.Plain.HideValue()));
    end;
    procedure H(): Text
    var R: Record "HV Row"; P: TestPage "HV Card";
    begin
        R.Id := 1; R.Name := 'Shown?'; R.Insert();
        P.OpenView();
        exit(Format(P.NameF.ShowMandatory()));
    end;
}
"#;
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "F")), "[]");
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "G")), "Yes");
    refused(al_testkit::run_in(src, "C", "H"), "showmandatory");
}

/// record/lt-x-table-props/XTablePropsTests.al:229-230: a page control shows the session language's entry of its table
/// field's OptionCaptionML (`deu_page_op` is `Zwei`). A session language other than 1033, 1031 and 1036 is not pinned and
/// refused (the pinned languages: al-vm/tests/language/option_caption_ml.rs).
#[test]
fn a_page_control_on_a_field_with_option_caption_ml_is_refused_in_an_unpinned_language() {
    let src = r#"
table 69996 "OM Row" { fields { field(1; Id; Integer) { } field(2; Op; Option) { OptionMembers = A,B,C; OptionCaptionML = ENU = 'Alpha,Beta,Gamma', DEU = 'Eins,Zwei,Drei'; } } keys { key(PK; Id) { } } }
page 69997 "OM Card"
{
    PageType = Card; SourceTable = "OM Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Op; Rec.Op) { } } }
}
codeunit 69998 C
{
    procedure F(): Text
    var R: Record "OM Row"; P: TestPage "OM Card";
    begin
        R.Id := 1; R.Op := R.Op::B; R.Insert();
        GlobalLanguage(1040);
        P.OpenView();
        exit(P.Op.Value());
    end;
}
"#;
    refused(al_testkit::run_in(src, "C", "F"), "OptionCaptionML");
}

/// pages/lt-ah-pages/AhPageTests.al:986-989: Expand on a tree shows the rows below the current one. That is pinned for a
/// page without row triggers; which rows a page with OnAfterGetRecord reads again is not, so Expand there is refused.
#[test]
fn expand_on_a_tree_page_with_row_triggers_is_refused() {
    let src = r#"
table 69960 "TR Row" { fields { field(1; Id; Integer) { } field(2; Ind; Integer) { } } keys { key(PK; Id) { } } }
page 69961 "TR List"
{
    PageType = List; SourceTable = "TR Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { ShowAsTree = true; IndentationColumn = Rec.Ind; field(Id; Rec.Id) { } } } }
    trigger OnAfterGetRecord() begin end;
}
codeunit 69962 C
{
    procedure F(): Text
    var R: Record "TR Row"; P: TestPage "TR List";
    begin
        R.Id := 1; R.Ind := 0; R.Insert(); R.Id := 2; R.Ind := 1; R.Insert();
        P.OpenView();
        P.Expand(true);
        exit(P.Id.Value());
    end;
}
"#;
    refused(al_testkit::run_in(src, "C", "F"), "row triggers");
}

/// pages/lt-page-editability-width-lookups/P115Lookups.al:120-122: Lookup on a control with its own TableRelation opens the
/// lookup page of that relation. The client reads only the table field's relation and refuses.
#[test]
fn lookup_through_a_control_level_table_relation_is_refused() {
    let src = r#"
table 69963 "LK Target" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 69964 "LK Row" { fields { field(1; Id; Integer) { } field(2; Ref; Code[10]) { } } keys { key(PK; Id) { } } }
page 69965 "LK Card"
{
    PageType = Card; SourceTable = "LK Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Ref; Rec.Ref) { TableRelation = "LK Target"; } } }
}
codeunit 69966 C
{
    procedure F(): Text
    var R: Record "LK Row"; P: TestPage "LK Card";
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        P.Ref.Lookup();
        exit(P.Ref.Value());
    end;
}
"#;
    refused(al_testkit::run_in(src, "C", "F"), "TableRelation");
}

/// events/lt-global-triggers-statements/P114GlobalTriggerEvents.al:301, :323: every caption class translation raises
/// CaptionClassTranslate of UI Helper Triggers. The runtime does not raise it and refuses while a subscriber listens.
#[test]
fn a_caption_class_translation_with_a_ui_helper_subscriber_is_refused() {
    let src = r#"
codeunit 69967 Subs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", 'CaptionClassTranslate', '', false, false)]
    local procedure T(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024]) begin end;
}
codeunit 69968 C
{
    procedure F(): Text begin exit(CaptionClassTranslate('3,Foo')); end;
}
"#;
    refused(al_testkit::run_in(src, "C", "F"), "CaptionClassTranslate");
}

const FIND_LIST: &str = r#"
table 69950 "FL Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
page 69951 "FL List"
{
    PageType = List; SourceTable = "FL Row"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
    trigger OnOpenPage() begin end;
    trigger OnFindRecord(Which: Text): Boolean begin exit(Rec.Find(Which)); end;
    trigger OnNextRecord(Steps: Integer): Integer begin exit(Rec.Next(Steps)); end;
    trigger OnAfterGetRecord() begin end;
    trigger OnAfterGetCurrRecord() begin end;
}
codeunit 69952 C
{
    local procedure Seed(N: Integer)
    var R: Record "FL Row"; i: Integer;
    begin
        for i := 1 to N do begin R.Id := i; R.Insert(); end;
    end;
    procedure Descending(): Text
    var P: TestPage "FL List";
    begin
        Seed(3);
        P.OpenView();
        exit(P.Id.Value());
    end;
    procedure GoToKeyFar(): Text
    var P: TestPage "FL List";
    begin
        Seed(25);
        P.OpenView();
        P.GoToKey(22);
        exit(P.Id.Value());
    end;
}
"#;

/// misc/lt-probes-needed-6/PnF06FindListMoves.al:304, :318: a descending OnFindRecord list with OnAfterGetCurrRecord opens on
/// the last row of its view (the row Find('=><') finds) and steps back over the rows above it (the reads: pages_ui/row_window.rs).
#[test]
fn opening_a_descending_find_list_with_on_after_get_curr_record_stands_on_the_last_row() {
    let src = FIND_LIST.replace(
        "trigger OnOpenPage() begin end;",
        "trigger OnOpenPage() begin Rec.Ascending(false); end;",
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "Descending")), "1");
}

/// pages/lt-ai-list-order/AiListOrderTests.al:999, :1020: GoToKey on an OnFindRecord list with a full window of 20 rows makes
/// the client ask the page for the rows around the key. That is not implemented, so it is refused.
#[test]
fn go_to_key_on_a_find_list_with_a_full_window_is_refused() {
    refused(al_testkit::run_in(FIND_LIST, "C", "GoToKeyFar"), "full window");
}

/// An error with an empty text out of a TestPage action's Invoke ends the Invoke quietly: the test goes on, the last error
/// has an empty text and code Dialog, and the rows the action wrote before the error stay
/// (pages/lt-qi-empty-error/QiEmptyErrorPages.al:192 QI_EE_EmptyAction_NoAsserterror). An error with a text still raises.
#[test]
fn an_empty_error_out_of_an_action_ends_the_invoke_quietly() {
    let src = r#"
table 69995 "EE Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
page 69996 "EE Card"
{
    PageType = Card; SourceTable = "EE Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } } }
    actions { area(Processing) { action(Quiet) { trigger OnAction() var W: Record "EE Row"; begin W.Id := 100; W.Insert(); Error(''); end; } action(Loud) { trigger OnAction() begin Error('loud'); end; } } }
}
codeunit 69997 C
{
    procedure F(): Text
    var R: Record "EE Row"; P: TestPage "EE Card";
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        P.Quiet.Invoke();
        exit('returned text=[' + GetLastErrorText() + '] code=[' + GetLastErrorCode() + '] kept=' + Format(R.Get(100)));
    end;
    procedure G(): Text
    var R: Record "EE Row"; P: TestPage "EE Card";
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        asserterror P.Loud.Invoke();
        exit(GetLastErrorText());
    end;
}
"#;
    assert_eq!(
        al_testkit::text(al_testkit::run_in(src, "C", "F")),
        "returned text=[] code=[Dialog] kept=Yes"
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "G")), "loud");
}

/// A list's repeater control over a page variable that OnAfterGetRecord computes: BC's client shows each row's own value
/// (pages/lt-qi-row-page-var/QiRowPageVar.al:223 QI_RV_List_Navigation; Tests-ERM 134760 Empl. Ledger Entries Preview AmountFCY
/// after First()).
#[test]
fn a_page_variable_control_on_another_row_shows_that_rows_value() {
    let src = r#"
table 69998 "PV Row" { fields { field(1; Id; Integer) { } field(2; Amt; Decimal) { } } keys { key(PK; Id) { } } }
page 69999 "PV List"
{
    PageType = List; SourceTable = "PV Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } field(Calc; CalcVar) { } } } }
    var CalcVar: Decimal;
    trigger OnAfterGetRecord() begin CalcVar := Rec.Amt * 10; end;
}
codeunit 69989 C
{
    procedure F(): Text
    var R: Record "PV Row"; P: TestPage "PV List";
    begin
        R.Id := 1; R.Amt := 1; R.Insert(); R.Id := 2; R.Amt := 2; R.Insert();
        P.OpenView();
        P.First();
        if P.Calc.AsDecimal() <> 10 then Error('first %1', P.Calc.Value());
        P.Next();
        exit(P.Calc.Value());
    end;
}
"#;
    assert_eq!(al_testkit::text(al_testkit::run_in(src, "C", "F")), "20.00");
}

/// pages/lt-qi-empty-error/QiEmptyErrorPages.al:253 QI_EE_EmptyValidate_NoAsserterror: an error with an empty text out of a card
/// control's own OnValidate ends the SetValue quietly; the entry does not stay, the last error is empty with code
/// NSValidateField:Dialog, and what the trigger wrote stays.
#[test]
fn an_empty_error_out_of_a_card_controls_on_validate_ends_the_set_value_quietly() {
    let src = r#"
table 69985 "EV Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
page 69986 "EV Card"
{
    PageType = Card; SourceTable = "EV Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } field(NameCtl; Rec.Name) { trigger OnValidate() var W: Record "EV Row"; begin W.Id := 300; W.Insert(); Error(''); end; } } }
}
codeunit 69987 C
{
    procedure F(): Text
    var R: Record "EV Row"; P: TestPage "EV Card";
    begin
        R.Id := 1; R.Name := 'seed'; R.Insert();
        P.OpenEdit();
        P.NameCtl.SetValue('empty');
        exit('went on text=[' + GetLastErrorText() + '] code=[' + GetLastErrorCode() + '] kept=' + Format(R.Get(300)) + ' name=' + P.NameCtl.Value());
    end;
}
"#;
    assert_eq!(
        al_testkit::text(al_testkit::run_in(src, "C", "F")),
        "went on text=[] code=[NSValidateField:Dialog] kept=Yes name=seed"
    );
}
