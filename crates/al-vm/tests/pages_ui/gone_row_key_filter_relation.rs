//! A Worksheet without the new-row line, filtered in filter group 2 on a key field with a TableRelation, loses its only row to
//! an action (al-runner issue 5951, the reporter's repro green on real BC; registered assumption
//! `blank-rec-key-filter-table-relation`, awaiting writer SH's RtFilteredGoneRow.al SH_RelWs_KeyFieldWithTableRelationGroup2_*).
//! With the related row present the blank Rec takes the filter value as without the relation (`N:B1;C:0:B1;`). Without it
//! the relation check fails, the error is swallowed and no trigger runs, then or at a later action.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};

const SRC: &str = r#"
table 51480 "RR Batch" { fields { field(1; Name; Code[10]) { } } keys { key(PK; Name) { } } }
table 51481 "RR Line"
{
    fields
    {
        field(1; "Batch Name"; Code[10]) { TableRelation = "RR Batch"; }
        field(2; "Line No."; Integer) { }
        field(3; Description; Text[50]) { }
    }
    keys { key(PK; "Batch Name", "Line No.") { } }
}
codeunit 51482 "RR Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take() Result: Text begin Result := Entries; Entries := ''; end;
}
page 51483 "RR Worksheet"
{
    PageType = Worksheet; SourceTable = "RR Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Rows) { field(LineNo; Rec."Line No.") { } field(Description; Rec.Description) { } } } }
    actions
    {
        area(Processing)
        {
            action(Post) { trigger OnAction() var Line: Record "RR Line"; begin Line.SetRange("Batch Name", 'B1'); Line.DeleteAll(); CurrPage.Update(false); end; }
            action(Touch) { trigger OnAction() begin CurrPage.Update(false); end; }
        }
    }
    trigger OnOpenPage() begin Rec.FilterGroup(2); Rec.SetRange("Batch Name", 'B1'); Rec.FilterGroup(0); end;
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit "RR Log"; begin L.Add('N:' + Rec."Batch Name"); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "RR Log"; begin L.Add('C:' + Format(Rec."Line No.") + ':' + Rec."Batch Name"); end;
}
codeunit 51484 C
{
    procedure Run(WithBatch: Boolean): Text
    var Batch: Record "RR Batch"; Line: Record "RR Line"; L: Codeunit "RR Log"; Ws: TestPage "RR Worksheet"; s: Text;
    begin
        if WithBatch then begin Batch.Name := 'B1'; Batch.Insert(); end;
        Line."Batch Name" := 'B1'; Line."Line No." := 10000; Line.Description := 'Only line'; Line.Insert();
        Ws.OpenEdit();
        L.Take();
        Ws.Post.Invoke();
        s := L.Take() + '|' + Ws.LineNo.Value() + '|' + Format(Ws.First()) + '|' + L.Take();
        Ws.Touch.Invoke();
        s += '|' + L.Take();
        Ws.Close();
        exit(s + '|' + Format(Line.Count()));
    end;
    procedure WithBatch(): Text begin exit(Run(true)); end;
    procedure WithoutBatch(): Text begin exit(Run(false)); end;
}
"#;

#[test]
fn with_the_related_row_the_blank_rec_takes_the_filter_value() {
    clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "WithBatch")),
        "N:B1;C:0:B1;||No||N:B1;C:0:B1;|0"
    );
    assert!(take_hits().contains(&Assumption::BlankRecKeyFilterTableRelation));
}

#[test]
fn without_the_related_row_no_trigger_runs() {
    clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "WithoutBatch")),
        "||No|||0"
    );
    assert!(take_hits().contains(&Assumption::BlankRecKeyFilterTableRelation));
}
