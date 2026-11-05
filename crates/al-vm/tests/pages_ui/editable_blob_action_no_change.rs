//! An editable List (Editable and ModifyAllowed left true) over a table with a Blob field: an action that changes the row through
//! its own record variable and leaves Rec alone must not be overwritten by the page saving its stale Rec after the action
//! (al-runner issue 7114). A Blob no stream wrote is no change of Rec, with and without a Commit in the action (corpus
//! pages/actions/R7114EditableBlobAction.al:272-300 PG7114_EditableList_ActionModifiesOwnVar_*).

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 69970 "EBA Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin if Entries <> '' then Entries += ';'; Entries += E; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
table 69971 "EBA File"
{
    fields { field(1; Code; Code[10]) { } field(2; Status; Option) { OptionMembers = Open,Imported; } field(3; "Doc No."; Code[20]) { } field(4; Content; Blob) { } }
    keys { key(PK; Code) { } }
    trigger OnModify() var L: Codeunit "EBA Log"; begin L.Add(Format(Status) + '/' + "Doc No."); end;
}
table 69972 "EBA Plain"
{
    fields { field(1; Code; Code[10]) { } field(2; Status; Option) { OptionMembers = Open,Imported; } field(3; "Doc No."; Code[20]) { } }
    keys { key(PK; Code) { } }
    trigger OnModify() var L: Codeunit "EBA Log"; begin L.Add(Format(Status) + '/' + "Doc No."); end;
}
codeunit 69973 "EBA Import"
{
    procedure Import(C: Code[10]; WithCommit: Boolean)
    var F: Record "EBA File";
    begin
        F.Get(C); F.Status := F.Status::Imported; F."Doc No." := 'DOC1'; F.Modify(true);
        if WithCommit then Commit();
    end;
    procedure ImportPlain(C: Code[10])
    var F: Record "EBA Plain";
    begin
        F.Get(C); F.Status := F.Status::Imported; F."Doc No." := 'DOC1'; F.Modify(true);
    end;
}
page 69974 "EBA Files"
{
    PageType = List; SourceTable = "EBA File"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } field(Status; Rec.Status) { } field("Doc No."; Rec."Doc No.") { } } } }
    actions { area(Processing) {
        action(ImportCommit) { trigger OnAction() var I: Codeunit "EBA Import"; begin I.Import(Rec.Code, true); end; }
        action(ImportNoCommit) { trigger OnAction() var I: Codeunit "EBA Import"; begin I.Import(Rec.Code, false); end; }
    } }
}
page 69975 "EBA Plains"
{
    PageType = List; SourceTable = "EBA Plain"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Code; Rec.Code) { } field(Status; Rec.Status) { } field("Doc No."; Rec."Doc No.") { } } } }
    actions { area(Processing) {
        action(ImportNoCommit) { trigger OnAction() var I: Codeunit "EBA Import"; begin I.ImportPlain(Rec.Code); end; }
    } }
}
codeunit 69976 C
{
    procedure Blob(WithCommit: Boolean): Text
    var F: Record "EBA File"; L: Codeunit "EBA Log"; P: TestPage "EBA Files";
    begin
        F.Code := 'F1'; F.Insert();
        P.OpenEdit(); P.GoToKey('F1'); L.Take();
        if WithCommit then P.ImportCommit.Invoke() else P.ImportNoCommit.Invoke();
        P.Close();
        F.Get('F1');
        exit(Format(F.Status) + '/' + F."Doc No." + ' log ' + L.Take());
    end;
    procedure NoCommit(): Text begin exit(Blob(false)); end;
    procedure WithCommit(): Text begin exit(Blob(true)); end;
    procedure Plain(): Text
    var F: Record "EBA Plain"; L: Codeunit "EBA Log"; P: TestPage "EBA Plains";
    begin
        F.Code := 'F1'; F.Insert();
        P.OpenEdit(); P.GoToKey('F1'); L.Take();
        P.ImportNoCommit.Invoke();
        P.Close();
        F.Get('F1');
        exit(Format(F.Status) + '/' + F."Doc No." + ' log ' + L.Take());
    end;
}
"#;

#[test]
fn an_action_that_changes_the_row_through_its_own_record_keeps_the_change() {
    assert_eq!(text(run_in(SRC, "C", "NoCommit")), "Imported/DOC1 log Imported/DOC1");
}

#[test]
fn the_same_page_over_a_table_without_a_blob_keeps_the_change() {
    assert_eq!(text(run_in(SRC, "C", "Plain")), "Imported/DOC1 log Imported/DOC1");
}

#[test]
fn an_action_whose_code_commits_keeps_the_change() {
    assert_eq!(text(run_in(SRC, "C", "WithCommit")), "Imported/DOC1 log Imported/DOC1");
}
