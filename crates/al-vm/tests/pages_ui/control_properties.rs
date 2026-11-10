//! Control properties on a TestPage (lane T2, corpus bump 29): CurrPage.Editable(false) in OnOpenPage per open mode, an
//! action's `Visible = <variable>` after an action changed it, a `Visible = false` FlowField on a card opened for viewing,
//! the text of an unset Decimal FlowFilter control, and the control expressions BC compiles on the client.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const LOCK: &str = r#"
table 69990 "CP Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
page 69991 "CP Lock Card"
{
    PageType = Card; SourceTable = "CP Row"; ApplicationArea = All;
    layout { area(Content) { field(IdCtl; Rec.Id) { } field(NameCtl; Rec.Name) { } field(NoteCtl; Note) { } } }
    trigger OnOpenPage() begin CurrPage.Editable(false); end;
    var Note: Text[30];
}
codeunit 69992 C
{
    procedure View(): Text
    var R: Record "CP Row"; P: TestPage "CP Lock Card"; S: Text;
    begin
        R.Id := 1; R.Name := 'n1'; R.Insert();
        P.OpenView();
        S := Format(P.Editable()) + ' ' + Format(P.NameCtl.Editable()) + ' ' + Format(P.NoteCtl.Editable());
        P.Close();
        exit(S);
    end;
    procedure New(): Text
    var R: Record "CP Row"; P: TestPage "CP Lock Card"; S: Text;
    begin
        R.Id := 1; R.Name := 'n1'; R.Insert();
        P.OpenNew();
        S := Format(P.Editable()) + ' ' + Format(P.IdCtl.Editable());
        P.Close();
        exit(S);
    end;
    procedure NewVar(): Text
    var P: TestPage "CP Lock Card";
    begin
        P.OpenNew();
        exit(Format(P.NoteCtl.Editable()));
    end;
    procedure Edit(): Text
    var R: Record "CP Row"; P: TestPage "CP Lock Card"; S: Text;
    begin
        R.Id := 1; R.Name := 'n1'; R.Insert();
        P.OpenEdit();
        S := Format(P.Editable()) + ' ' + Format(P.NameCtl.Editable()) + ' ' + Format(P.NoteCtl.Editable());
        P.Close();
        exit(S);
    end;
}
"#;

/// testpage/CdTestPageLeads.al:93-95 (Item49_LockCard_OpenView): the page and its Rec controls read No,
/// a page-variable control stays editable.
#[test]
fn editable_false_in_on_open_page_of_a_view_card_leaves_a_variable_control_editable() {
    assert_eq!(text(al_testkit::run_in(LOCK, "C", "View")), "No No Yes");
}

/// testpage/CdTestPageLeads.al:117-118 (Item49_LockCard_OpenNew): the page reads Yes, a Rec control No.
#[test]
fn editable_false_in_on_open_page_of_a_new_card_keeps_the_page_editable_and_locks_rec_controls() {
    assert_eq!(text(al_testkit::run_in(LOCK, "C", "New")), "Yes No");
}

/// Nothing pins a page-variable control of a new card after CurrPage.Editable(false): refused.
#[test]
fn a_variable_control_of_a_new_card_after_editable_false_is_refused() {
    match al_testkit::run_in(LOCK, "C", "NewVar") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("Editable(false)"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// testpage/TestPageControlEditableRoutes.al:553 (CurrPageEditableFalse_OpenEdit): everything reads No.
#[test]
fn editable_false_in_on_open_page_of_an_edit_card_locks_every_control() {
    assert_eq!(text(al_testkit::run_in(LOCK, "C", "Edit")), "No No No");
}

const VIS: &str = r#"
table 69993 "CP Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
page 69994 "CP Vis Card"
{
    PageType = Card; SourceTable = "CP Row"; ApplicationArea = All;
    layout { area(Content) { field(NameCtl; Rec.Name) { Visible = ShowField; } group(Grp) { Visible = ShowGroup; field(IdInGroup; Rec.Id) { } } } }
    actions
    {
        area(Processing)
        {
            action(HideAll) { trigger OnAction() begin ShowField := false; ShowGroup := false; ShowAction := false; end; }
            action(HideAllUpdate) { trigger OnAction() begin ShowField := false; ShowGroup := false; ShowAction := false; CurrPage.Update(false); end; }
            action(Target) { Visible = ShowAction; trigger OnAction() begin end; }
        }
    }
    trigger OnInit() begin ShowField := true; ShowGroup := true; ShowAction := true; end;
    var ShowField: Boolean; ShowGroup: Boolean; ShowAction: Boolean;
}
codeunit 69995 C
{
    local procedure Vis(var P: TestPage "CP Vis Card"): Text
    begin
        exit(Format(P.NameCtl.Visible()) + ' ' + Format(P.IdInGroup.Visible()) + ' ' + Format(P.Target.Visible()));
    end;
    procedure Plain(): Text
    var R: Record "CP Row"; P: TestPage "CP Vis Card"; S: Text;
    begin
        R.Id := 1; R.Insert(); R.Id := 2; R.Insert();
        P.OpenEdit();
        S := Vis(P);
        P.HideAll.Invoke();
        S += '|' + Vis(P);
        P.Next(); P.Previous();
        S += '|' + Vis(P);
        P.Close();
        exit(S);
    end;
    procedure Update(): Text
    var R: Record "CP Row"; P: TestPage "CP Vis Card"; S: Text;
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        P.HideAllUpdate.Invoke();
        S := Vis(P);
        P.Close();
        exit(S);
    end;
}
"#;

/// testpage/CdTestPageLeads.al:251-253 (Item51_VisibleVariables_ActionWithoutUpdate): a field keeps its
/// open-time Visible, a group and an action follow the variable.
#[test]
fn an_action_visible_variable_follows_an_action_that_changed_it() {
    assert_eq!(
        text(al_testkit::run_in(VIS, "C", "Plain")),
        "Yes Yes Yes|Yes No No|Yes No No"
    );
}

/// testpage/CdTestPageLeads.al:267 (Item51_VisibleVariables_ActionWithUpdate).
#[test]
fn an_action_visible_variable_follows_an_action_that_updated_the_page() {
    assert_eq!(text(al_testkit::run_in(VIS, "C", "Update")), "Yes No No");
}

const FLOW: &str = r#"
table 69996 "CP Line" { fields { field(1; Id; Integer) { } field(2; LineNo; Integer) { } field(3; Amt; Decimal) { } } keys { key(PK; Id, LineNo) { } } }
table 69997 "CP Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(7; FlowShown; Decimal) { FieldClass = FlowField; CalcFormula = sum("CP Line".Amt where(Id = field(Id))); }
        field(8; FlowHidden; Decimal) { FieldClass = FlowField; CalcFormula = sum("CP Line".Amt where(Id = field(Id))); }
    }
    keys { key(PK; Id) { } }
}
codeunit 69998 "CP Log" { SingleInstance = true; var T: Text; procedure Add(S: Text) begin T += S + ';'; end; procedure Take(): Text var R: Text; begin R := T; T := ''; exit(R); end; }
page 69999 "CP Flow Card"
{
    PageType = Card; SourceTable = "CP Row"; ApplicationArea = All;
    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { }
            field(FlowShown; Rec.FlowShown) { }
            field(FlowHidden; Rec.FlowHidden) { Visible = false; }
        }
    }
    trigger OnAfterGetRecord() var Log: Codeunit "CP Log"; begin Log.Add('shown=' + Format(Rec.FlowShown) + ' hidden=' + Format(Rec.FlowHidden)); end;
}
codeunit 69989 C
{
    local procedure Seed()
    var R: Record "CP Row"; L: Record "CP Line";
    begin
        R.Id := 1; R.Insert();
        L.Id := 1; L.LineNo := 1; L.Amt := 5; L.Insert();
    end;
    procedure View(): Text
    var P: TestPage "CP Flow Card"; Log: Codeunit "CP Log"; S: Text;
    begin
        Seed();
        P.OpenView();
        S := Log.Take() + P.FlowShown.Value();
        P.Close();
        exit(S);
    end;
    procedure Edit(): Text
    var P: TestPage "CP Flow Card"; Log: Codeunit "CP Log";
    begin
        Seed();
        P.OpenEdit();
        exit(Log.Take());
    end;
}
"#;

/// testpage/CcPageLeads.al:722-723 (FlowField_VisibleFalse_OnOpenView): a card opened for viewing does not
/// calculate a FlowField whose control says `Visible = false`.
#[test]
fn a_view_card_does_not_calculate_a_hidden_flowfield() {
    assert_eq!(text(al_testkit::run_in(FLOW, "C", "View")), "shown=5 hidden=0;5.00");
}

/// Whether another open mode calculates it is not pinned (docs/probes-needed.md); it keeps calculating it, as before
/// (kept for testpage/TestPartSubPageViewPrecompiled.al:104).
#[test]
fn an_edit_card_still_calculates_a_hidden_flowfield() {
    assert_eq!(text(al_testkit::run_in(FLOW, "C", "Edit")), "shown=5 hidden=5;");
}

const FILTER: &str = r#"
table 69986 "CP FRow" { fields { field(1; Id; Integer) { } field(3; AmtFilter; Decimal) { FieldClass = FlowFilter; } } keys { key(PK; Id) { } } }
page 69987 "CP FCard" { PageType = Card; SourceTable = "CP FRow"; layout { area(Content) { field(AmtFilter; Rec.AmtFilter) { } } } }
codeunit 69988 C
{
    procedure FilterText(): Text
    var R: Record "CP FRow"; P: TestPage "CP FCard"; S: Text;
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        S := '[' + P.AmtFilter.Value() + ']';
        P.AmtFilter.SetValue('7');
        S += '[' + P.AmtFilter.Value() + ']';
        P.Close();
        exit(S);
    end;
}
"#;

/// testpage/N582TestPageEdges.al:400 (BR_Editable_FlowFilterControl): an unset Decimal FlowFilter control shows nothing;
/// :428 (BR_SetValue_FlowFilterControl) after SetValue('7') it shows 7.00.
#[test]
fn an_unset_decimal_flowfilter_control_shows_nothing() {
    assert_eq!(text(al_testkit::run_in(FILTER, "C", "FilterText")), "[][7.00]");
}

fn expr_src(prop: &str) -> String {
    format!(
        r#"
table 69980 "CP ERow" {{ fields {{ field(1; Id; Integer) {{ }} field(4; Flag; Boolean) {{ }} field(5; D; Date) {{ }} field(6; Big; BigInteger) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
page 69981 "CP ECard"
{{
    PageType = Card; SourceTable = "CP ERow";
    layout {{ area(Content) {{ field(Name; Rec.Id) {{ {prop} }} }} }}
    procedure GetRow(): Record "CP ERow" begin exit(Rec); end;
    var GlobalRow: Record "CP ERow";
}}
codeunit 69982 C
{{
    procedure Open(): Text
    var R: Record "CP ERow"; P: TestPage "CP ECard";
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        exit(Format(P.Name.Editable()));
    end;
}}
"#
    )
}

fn refused(prop: &str) -> String {
    match al_testkit::run_in(&expr_src(prop), "C", "Open") {
        Err(Signal::Unsupported(why)) => why,
        other => panic!("expected a refusal for `{prop}`, got {other:?}"),
    }
}

/// testpage/CcPageLeads.al:764 and :846 and :880: BC compiles these control expressions when the page opens
/// and fails there. Neither the grammar nor the error index rule is pinned beyond these cases, so the page is refused.
#[test]
fn control_expressions_bc_fails_to_compile_are_refused() {
    assert!(refused("Editable = GetRow().Flag;").contains("dynamic control expression"));
    assert!(refused("Editable = Rec.D = 0D;").contains("dynamic control expression"));
    assert!(refused("Enabled = GlobalRow.Id = 2;").contains("dynamic control expression"));
}

#[test]
fn plain_control_expressions_still_run() {
    let r = al_testkit::run_in(&expr_src("Editable = Rec.Flag or (Rec.Id = 1);"), "C", "Open");
    assert!(matches!(r, Ok(Value::Text(ref t)) if t.as_str() == "Yes"), "{r:?}");
}

/// A page with an Enum and an Option field on table "CP QRow", the page's own control `Name` with `{own}` and a control an
/// extension adds, `ExtName`, with `{ext}`. `GlobalRow` is the page's global Record, `ExtRow` the extension's.
fn qualified_src(own: &str, ext: &str) -> String {
    format!(
        r#"
enum 69984 "CP QKind" {{ value(0; Alpha) {{ }} value(1; Beta) {{ }} }}
table 69985 "CP QRow" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Mode; Enum "CP QKind") {{ }} field(3; Opt; Option) {{ OptionMembers = A,B; }} }} keys {{ key(PK; Id) {{ }} }} }}
page 69986 "CP QCard"
{{
    PageType = Card; SourceTable = "CP QRow";
    layout {{ area(Content) {{ field(Name; Rec.Id) {{ {own} }} }} }}
    trigger OnOpenPage() begin Dir := Dir::Beta; end;
    var GlobalRow: Record "CP QRow"; Dir: Enum "CP QKind";
}}
pageextension 69987 "CP QCard Ext" extends "CP QCard"
{{
    layout {{ addlast(Content) {{ field(ExtName; Rec.Id) {{ {ext} }} }} }}
    trigger OnOpenPage() begin ExtDir := ExtDir::Beta; end;
    var ExtRow: Record "CP QRow"; ExtDir: Enum "CP QKind";
}}
codeunit 69988 C
{{
    procedure Open(): Text
    var R: Record "CP QRow"; P: TestPage "CP QCard";
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        exit(Format(P.Name.Editable()) + '|' + Format(P.ExtName.Editable()));
    end;
}}
"#
    )
}

/// testpage/CcPageLeads.al:1051 (VisibleExpr_V1): an Enum value through an Enum field of a global Record runs on the
/// page's own control with no assumption. On a control an extension adds, through the extension's own global, it runs
/// too but marks `control-expr-member-enum-in-extension` (al-runner issue 6924; not pinned).
#[test]
fn an_enum_value_through_a_global_record_runs_and_marks_the_extension_case() {
    use al_builtins::assumptions::{Assumption, take_hits};
    take_hits();
    let src = qualified_src("Editable = Dir = GlobalRow.Mode::Beta;", "Editable = true;");
    let r = al_testkit::run_in(&src, "C", "Open");
    assert!(matches!(r, Ok(Value::Text(ref t)) if t.as_str() == "Yes|Yes"), "{r:?}");
    assert!(!take_hits().contains(&Assumption::ControlExprMemberEnumInExtension));
    let src = qualified_src("Editable = true;", "Editable = ExtDir = ExtRow.Mode::Alpha;");
    let r = al_testkit::run_in(&src, "C", "Open");
    assert!(matches!(r, Ok(Value::Text(ref t)) if t.as_str() == "Yes|No"), "{r:?}");
    assert!(take_hits().contains(&Assumption::ControlExprMemberEnumInExtension));
}

/// Not pinned, so refused: an Option field through a global Record (the form is used by a Microsoft page no test opens),
/// a nested member and a member of `this.`.
#[test]
fn qualified_values_through_other_members_stay_refused() {
    for prop in [
        "Editable = GlobalRow.Opt = GlobalRow.Opt::B;",
        "Editable = this.GlobalRow.Mode::Beta = Dir;",
        "Editable = GlobalRow.Mode.X::Beta = Dir;",
    ] {
        match al_testkit::run_in(&qualified_src(prop, "Editable = true;"), "C", "Open") {
            Err(Signal::Unsupported(why)) => assert!(why.contains("dynamic control expression"), "{prop}: {why}"),
            other => panic!("expected a refusal for `{prop}`, got {other:?}"),
        }
    }
}

fn run_on(bc: &str, src: &str) -> Result<Value, Signal> {
    use al_version::{BcVersion, Target};
    let target = Target::for_bc(BcVersion::parse(bc).unwrap());
    let apps = vec![al_sema::workspace::AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), src.into())],
    }];
    let (ws, l) = al_sema::lower::compile_for(apps, &Default::default(), &target);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = al_testkit::blocked(&l);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "Open")
        .unwrap();
    let mut s = al_vm::Session::new(al_vm::Engine::with_target(l.program, blocked, Some(target)));
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

/// testpage/CcPageLeads.al:801-808 (Expr_BigIntegerLiteral_Enabled): BC 27 fails to compile `0L <> Rec.Big`
/// when the page opens (refused: the error's index rule is not pinned); BC 28 and 29 run it.
#[test]
fn a_big_integer_literal_in_a_control_expression_is_refused_on_bc_27_only() {
    let src = expr_src("Editable = 0L <> Rec.Big;");
    match run_on("27.5", &src) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("BigInteger literal"), "{why}"),
        other => panic!("expected a refusal on 27, got {other:?}"),
    }
    let r = run_on("28.5", &src);
    assert!(matches!(r, Ok(Value::Text(ref t)) if t.as_str() == "No"), "{r:?}");
}

const READ_LOCK: &str = r#"
table 69970 "CP Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
page 69971 "CP Read Card"
{
    PageType = Card; SourceTable = "CP Row"; ApplicationArea = All;
    layout { area(Content) { field(IdCtl; Rec.Id) { } } }
    actions { area(Processing) { action(Read) { trigger OnAction() begin if CurrPage.Editable() then; end; } } }
    trigger OnOpenPage() begin CurrPage.Editable(false); end;
}
codeunit 69972 C
{
    procedure New()
    var P: TestPage "CP Read Card";
    begin
        P.OpenNew();
        P.Read.Invoke();
    end;
}
"#;

/// CurrPage.Editable() read from page code on an OpenNew page after CurrPage.Editable(false): not pinned
/// (CdTestPageLeads.al:117 pins TestPage.Editable() only), refused.
#[test]
fn curr_page_editable_read_on_a_new_page_after_editable_false_is_refused() {
    match al_testkit::run_in(READ_LOCK, "C", "New") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("CurrPage.Editable()"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

const FILTER_ZERO: &str = r#"
table 69973 "CP FRow" { fields { field(1; Id; Integer) { } field(3; AmtFilter; Decimal) { FieldClass = FlowFilter; } } keys { key(PK; Id) { } } }
page 69974 "CP FCard" { PageType = Card; SourceTable = "CP FRow"; layout { area(Content) { field(AmtFilter; Rec.AmtFilter) { } } } }
codeunit 69975 C
{
    procedure Zero(): Text
    var R: Record "CP FRow"; P: TestPage "CP FCard";
    begin
        R.Id := 1; R.Insert();
        P.OpenEdit();
        P.First();
        P.AmtFilter.SetValue('0');
        exit(P.AmtFilter.Value());
    end;
}
"#;

/// A Decimal FlowFilter control given an explicit zero: only the unset field is pinned (N582TestPageEdges.al:400), refused.
#[test]
fn a_decimal_flowfilter_control_set_to_zero_is_refused() {
    match al_testkit::run_in(FILTER_ZERO, "C", "Zero") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("FlowFilter"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
