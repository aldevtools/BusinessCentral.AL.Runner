//! ClearAll in a page extension resets the extension's globals and the base page's, and keeps Rec
//! (pages/extensions/CyClearAllExtensions.al:166-193, record/ProbeClearAllSetKeyHyperlink.al:389-412).

use al_testkit::text;

const SRC: &str = r#"
table 69870 "CAP Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
}
page 69871 "CAP Card"
{
    PageType = Card; SourceTable = "CAP Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { } field(NameCtl; Rec.Name) { } field(PageGlobCtl; PageGlob) { } field(PageOpenCtl; PageOpen) { } } }
    var PageGlob: Integer; PageOpen: Integer;
    trigger OnOpenPage() begin PageOpen := 3; end;
    trigger OnAfterGetRecord() begin PageGlob := 42; end;
}
pageextension 69872 "CAP Card Ext" extends "CAP Card"
{
    layout { addlast(Content) { field(ExtGlobCtl; ExtGlob) { ApplicationArea = All; } field(ExtSeenCtl; ExtSeen) { ApplicationArea = All; } } }
    var ExtGlob: Integer; ExtSeen: Integer; ExtText: Text;
    trigger OnOpenPage() begin ExtGlob := 9; end;
    trigger OnAfterGetRecord() begin ClearAll(); ExtSeen := 1; end;
    procedure SetGlobals(V: Integer; T: Text) begin ExtGlob := V; ExtText := T; end;
    procedure Describe(): Text begin exit('int=' + Format(ExtGlob) + ',text=' + ExtText); end;
    procedure ClearInExt(): Text begin ClearAll(); exit(Describe()); end;
}
codeunit 69873 C
{
    procedure InTrigger(): Text
    var R: Record "CAP Row"; P: TestPage "CAP Card";
    begin
        R.Id := 1; R.Name := 'n'; R.Insert();
        P.OpenView();
        P.GoToKey(1);
        exit(P.PageGlobCtl.Value() + '|' + P.PageOpenCtl.Value() + '|' + P.ExtGlobCtl.Value() + '|' + P.ExtSeenCtl.Value()
            + '|' + P.IdCtl.Value() + '|' + P.NameCtl.Value());
    end;

    procedure InProcedure(): Text
    var Card: Page "CAP Card"; Before: Text;
    begin
        Card.SetGlobals(5, 'set');
        Before := Card.Describe();
        exit(Before + ';' + Card.ClearInExt() + ';' + Card.Describe());
    end;
}
"#;

/// CyClearAllExtensions.al:166-193: the base page's globals from OnOpenPage and OnAfterGetRecord are 0, Rec stays.
#[test]
fn clear_all_in_a_page_extension_trigger_resets_both_pages_globals_and_keeps_rec() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "InTrigger")), "0|0|0|1|1|n");
}

/// ProbeClearAllSetKeyHyperlink.al:389-412: through a Page variable that is not open.
#[test]
fn clear_all_in_a_page_extension_procedure_resets_its_globals() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "InProcedure")),
        "int=5,text=set;int=0,text=;int=0,text="
    );
}
