//! A part with `UpdatePropagation = Both` whose trigger changes the host's stored row and calls `CurrPage.Update(false)`: the host
//! reads its row again, so its later save keeps the change (pages/parts/QjPartPropagation.al:508, corpus bump 41;
//! Tests-ERM Document Totals Pages 134344 PurchaseQuoteUpdateCurrrencyFactor needs it). A host with an unsaved edit is saved first.

use al_testkit::text;

const SRC: &str = r#"
table 51460 "UP Head" { fields { field(1; "Code"; Code[20]) { } field(2; Descr; Text[30]) { } field(3; Info; Text[30]) { } } keys { key(PK; "Code") { } } }
table 51461 "UP Line"
{
    fields { field(1; "Head Code"; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Descr; Text[30]) { } }
    keys { key(PK; "Head Code", "Line No.") { } }
}
page 51462 "UP Lines"
{
    PageType = ListPart;
    SourceTable = "UP Line";
    layout
    {
        area(Content)
        {
            repeater(R) { field(L; Rec."Line No.") { } }
            field(Total; TotalTxt)
            {
                trigger OnValidate()
                var H: Record "UP Head";
                begin
                    H.Get(Rec."Head Code"); H.Info := TotalTxt; H.Modify();
                    CurrPage.Update(false);
                end;
            }
        }
    }
    var TotalTxt: Text[30];
}
page 51463 "UP Card"
{
    PageType = Document;
    SourceTable = "UP Head";
    layout
    {
        area(Content)
        {
            field(CodeCtl; Rec."Code") { }
            field(DescrCtl; Rec.Descr) { }
            field(InfoCtl; Rec.Info) { }
            part(Lines; "UP Lines") { SubPageLink = "Head Code" = field("Code"); UpdatePropagation = Both; }
        }
    }
    actions { area(Processing) { action(Save) { trigger OnAction() begin Rec.Validate(Descr, 'by action'); CurrPage.SaveRecord(); end; } } }
}
codeunit 51464 "UP Test"
{
    procedure Run(): Text
    var H: Record "UP Head"; L: Record "UP Line"; Doc: TestPage "UP Card"; T: Text;
    begin
        H."Code" := 'H1'; H.Insert();
        L."Head Code" := 'H1'; L."Line No." := 10000; L.Insert();
        Doc.OpenEdit();
        Doc.Lines.Total.SetValue('by part');
        T := 'card=' + Doc.InfoCtl.Value();
        Doc.Save.Invoke();
        H.Get('H1');
        exit(T + '|stored=' + H.Descr + '/' + H.Info);
    end;
    procedure Dirty(): Text
    var H: Record "UP Head"; L: Record "UP Line"; Doc: TestPage "UP Card";
    begin
        H."Code" := 'H1'; H.Insert();
        L."Head Code" := 'H1'; L."Line No." := 10000; L.Insert();
        Doc.OpenEdit();
        Doc.DescrCtl.SetValue('typed');
        Doc.Lines.Total.SetValue('by part');
        exit(Doc.InfoCtl.Value());
    end;
}
"#;

#[test]
fn a_part_update_with_propagation_both_refreshes_the_host_row_its_code_changed() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "UP Test", "Run")),
        "card=by part|stored=by action/by part"
    );
}

#[test]
fn a_part_edit_over_an_unsaved_host_edit_saves_the_host_first() {
    // QjPartPropagation.al:623 QJ_Both_HostEdited_PartModifiesHost_ThenHostSave: the host's edit is saved before the
    // part's OnValidate, and the part's change to the host row shows after the host reads it again.
    assert_eq!(text(al_testkit::run_in(SRC, "UP Test", "Dirty")), "by part");
}
