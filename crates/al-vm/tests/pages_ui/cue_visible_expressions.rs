//! Visible of a cuegroup and of the fields in it bound to expressions (al-runner issue 6675; corpus writer Y65,
//! pages/lt-y65-cue-visible/Y65CueVisible.al:235, :262 on corpus branch al-runtime-rs-staging-y65, not pinned yet): variables
//! set in OnInit, OnOpenPage and OnAfterGetRecord, `not` a variable, a Boolean field of Rec, an expression over Rec, a variable
//! and a Rec expression, an array element and `not Rec.IsEmpty()` give the value they have after the page opened; a cuegroup
//! bound to a call of the page's own procedure reads No. A hidden field still answers its value.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69970 "CV Cue"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(2; CueA; Integer) { }
        field(3; CueB; Integer) { }
        field(4; CueC; Integer) { }
        field(5; CueD; Integer) { }
        field(6; CueE; Integer) { }
        field(7; CueF; Integer) { }
        field(8; CueG; Integer) { }
        field(9; CueH; Integer) { }
        field(10; CueI; Integer) { }
        field(11; Flag; Boolean) { }
        field(12; CueJ; Integer) { }
        field(13; CueK; Integer) { }
        field(14; CueL; Integer) { }
    }
    keys { key(PK; "Primary Key") { Clustered = true; } }
}
page 69971 "CV Activities"
{
    PageType = CardPart; SourceTable = "CV Cue"; ApplicationArea = All; RefreshOnActivate = true;
    layout
    {
        area(Content)
        {
            cuegroup(GrpOpenTrue) { Visible = ShowOpenTrue; field(CueA; Rec.CueA) { } }
            cuegroup(GrpOpenFalse) { Visible = ShowOpenFalse; field(CueB; Rec.CueB) { } }
            cuegroup(GrpNot) { Visible = not ShowOpenFalse; field(CueC; Rec.CueC) { } }
            cuegroup(GrpInit) { Visible = ShowInit; field(CueD; Rec.CueD) { } }
            cuegroup(GrpLate) { Visible = ShowLate; field(CueE; Rec.CueE) { } }
            cuegroup(GrpFields)
            {
                field(CueF; Rec.CueF) { Visible = Rec.CueF <> 0; }
                field(CueG; Rec.CueG) { Visible = Rec.Flag; }
                field(CueH; Rec.CueH) { Visible = ShowOpenTrue and (Rec.CueH = 0); }
                field(CueI; Rec.CueI) { Visible = ShowOpenFalse; }
            }
            cuegroup(GrpCall) { Visible = IsShown(); field(CueJ; Rec.CueJ) { } }
            cuegroup(GrpArr) { Visible = ShowArr[2]; field(CueK; Rec.CueK) { } }
            cuegroup(GrpEmpty) { Visible = not Rec.IsEmpty(); field(CueL; Rec.CueL) { } }
        }
    }
    trigger OnInit() begin ShowInit := true; end;
    trigger OnOpenPage()
    begin
        Rec.Reset();
        if not Rec.Get() then begin
            Rec.Init();
            Rec.CueF := 3;
            Rec.Insert();
        end;
        ShowOpenTrue := true;
        ShowOpenFalse := false;
        ShowArr[2] := true;
    end;
    trigger OnAfterGetRecord() begin ShowLate := true; end;
    var
        ShowOpenTrue: Boolean;
        ShowOpenFalse: Boolean;
        ShowInit: Boolean;
        ShowLate: Boolean;
        ShowArr: array[2] of Boolean;
    local procedure IsShown(): Boolean begin exit(ShowOpenTrue); end;
}
codeunit 69972 C
{
    local procedure ReadAll(var P: TestPage "CV Activities"): Text
    begin
        exit(Format(P.CueA.Visible()) + ' ' + P.CueA.Value() + ',' + Format(P.CueB.Visible()) + ' ' + P.CueB.Value() + ','
            + Format(P.CueC.Visible()) + ' ' + P.CueC.Value() + ',' + Format(P.CueD.Visible()) + ' ' + P.CueD.Value() + ','
            + Format(P.CueE.Visible()) + ' ' + P.CueE.Value() + ',' + Format(P.CueF.Visible()) + ' ' + P.CueF.Value() + ','
            + Format(P.CueG.Visible()) + ' ' + P.CueG.Value() + ',' + Format(P.CueH.Visible()) + ' ' + P.CueH.Value() + ','
            + Format(P.CueI.Visible()) + ' ' + P.CueI.Value() + ',' + Format(P.CueJ.Visible()) + ' ' + P.CueJ.Value() + ','
            + Format(P.CueK.Visible()) + ' ' + P.CueK.Value() + ',' + Format(P.CueL.Visible()) + ' ' + P.CueL.Value());
    end;
    procedure NoRowYet(): Text
    var Cue: Record "CV Cue"; P: TestPage "CV Activities"; S: Text;
    begin
        Cue.DeleteAll();
        P.OpenEdit();
        S := ReadAll(P);
        P.Close();
        exit(S + ' rows=' + Format(Cue.Count()));
    end;
    procedure RowWithFlag(): Text
    var Cue: Record "CV Cue"; P: TestPage "CV Activities"; S: Text;
    begin
        Cue.DeleteAll();
        Cue.Init();
        Cue.Flag := true;
        Cue.CueH := 4;
        Cue.Insert();
        P.OpenView();
        S := ReadAll(P);
        P.Close();
        exit(S);
    end;
}
"#;

/// Y65CueVisible.al:235 (Y65_CueVisible_NoRowYet).
#[test]
fn cuegroup_and_field_visible_expressions_read_their_value_after_open() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "NoRowYet")),
        "Yes 0,No 0,Yes 0,Yes 0,Yes 0,Yes 3,No 0,Yes 0,No 0,No 0,Yes 0,Yes 0 rows=1"
    );
}

/// Y65CueVisible.al:262 (Y65_CueVisible_RowWithFlag).
#[test]
fn cue_field_visible_follows_the_row_the_page_shows() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "RowWithFlag")),
        "Yes 0,No 0,Yes 0,Yes 0,Yes 0,No 0,Yes 0,No 4,No 0,No 0,Yes 0,Yes 0"
    );
}

const UNPINNED: &str = r#"
table 69973 "CV Row" { fields { field(1; Id; Integer) { } field(2; A; Integer) { } } keys { key(PK; Id) { } } }
page 69974 "CV Call In Expr"
{
    PageType = CardPart; SourceTable = "CV Row"; ApplicationArea = All;
    layout { area(Content) { cuegroup(Grp) { Visible = IsShown() and Show; field(A; Rec.A) { } } } }
    var Show: Boolean;
    local procedure IsShown(): Boolean begin exit(true); end;
}
page 69975 "CV Fixed Call"
{
    PageType = Card; SourceTable = "CV Row"; ApplicationArea = All;
    layout { area(Content) { fixed(Fx) { Visible = IsShown(); group(G) { field(A; Rec.A) { } } } } }
    local procedure IsShown(): Boolean begin exit(true); end;
}
codeunit 69976 C
{
    procedure CallInExpr(): Text
    var P: TestPage "CV Call In Expr";
    begin
        P.OpenView();
        exit(Format(P.A.Visible()));
    end;
    procedure FixedCall(): Text
    var P: TestPage "CV Fixed Call";
    begin
        P.OpenView();
        exit(Format(P.A.Visible()));
    end;
}
"#;

/// What Y65 does not pin stays refused with a reason, never "has no routine": a call inside a larger cuegroup expression, and a
/// procedure call on a fixed control (BC compiles both).
#[test]
fn unpinned_cuegroup_and_fixed_visible_calls_stay_refused() {
    for routine in ["CallInExpr", "FixedCall"] {
        let r = al_testkit::run_in(UNPINNED, "C", routine);
        assert!(
            matches!(&r, Err(Signal::Unsupported(w)) if w.contains("a call") && !w.contains("has no routine")),
            "{routine}: {r:?}"
        );
    }
}
