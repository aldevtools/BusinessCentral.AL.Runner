//! A journal batch switch from the new-row line of an empty batch to a batch with rows (gap G5-20): the page leaves the line
//! and refreshes as from a row; Update(true) modifies the row Find('-') moved Rec onto; a Worksheet with InsertAllowed = false
//! opens on the empty batch with OnNewRecord and OnAfterGetCurrRecord once (pages/lt-y50-update-from-new-line/Y50UpdateFromNewLine.al).

const SRC: &str = r#"
table 51540 "NL Line"
{
    fields { field(1; "Batch Name"; Code[10]) { } field(2; "Line No."; Integer) { } field(3; "No."; Code[20]) { } }
    keys { key(PK; "Batch Name", "Line No.") { } }
    trigger OnModify() begin L.Add('L.OnModify:' + "Batch Name" + '/' + Format("Line No.")); end;
    var L: Codeunit "NL Log";
}
codeunit 51541 "NL Log"
{
    SingleInstance = true;
    var E: Text; UpdTrue: Boolean;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
    procedure SetUpdTrue(B: Boolean) begin UpdTrue := B; end;
    procedure UpdateTrue(): Boolean begin exit(UpdTrue); end;
}
page 51542 "NL Ws"
{
    PageType = Worksheet; SourceTable = "NL Line"; AutoSplitKey = true; DelayedInsert = true;
    layout
    {
        area(Content)
        {
            group(Header)
            {
                field(CurrentBatch; CurrentBatch)
                {
                    trigger OnValidate()
                    begin
                        L.Add('V:start ' + Format(Rec."Line No."));
                        CurrPage.SaveRecord();
                        SetBatch();
                        if L.UpdateTrue() then CurrPage.Update(true) else CurrPage.Update(false);
                        L.Add('V:done ' + Format(Rec."Line No."));
                    end;
                }
            }
            repeater(Rows) { field("Line No."; Rec."Line No.") { } field("No."; Rec."No.") { } }
        }
    }
    var L: Codeunit "NL Log"; CurrentBatch: Code[10];
    trigger OnOpenPage() begin CurrentBatch := 'B3'; SetBatch(); end;
    trigger OnAfterGetRecord() begin L.Add('R:' + Rec."Batch Name" + '/' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Rec."Batch Name" + '/' + Format(Rec."Line No.")); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('N:' + Rec."Batch Name" + '/' + Format(Rec."Line No.")); end;
    trigger OnModifyRecord(): Boolean begin L.Add('M:' + Rec."Batch Name" + '/' + Format(Rec."Line No.")); end;
    local procedure SetBatch()
    begin
        Rec.FilterGroup := 2; Rec.SetRange("Batch Name", CurrentBatch); Rec.FilterGroup := 0;
        if Rec.Find('-') then;
    end;
}
page 51543 "NL Ws NoIns"
{
    PageType = Worksheet; SourceTable = "NL Line"; AutoSplitKey = true; DelayedInsert = true; InsertAllowed = false;
    layout
    {
        area(Content)
        {
            repeater(Rows) { field("Line No."; Rec."Line No.") { } }
        }
    }
    var L: Codeunit "NL Log";
    trigger OnOpenPage() begin Rec.FilterGroup := 2; Rec.SetRange("Batch Name", 'B3'); Rec.FilterGroup := 0; end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Rec."Batch Name" + '/' + Format(Rec."Line No.")); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('N:' + Rec."Batch Name" + '/' + Format(Rec."Line No.")); end;
}
codeunit 51544 C
{
    local procedure Add(B: Code[10]; N: Integer; No: Code[20])
    var Line: Record "NL Line";
    begin
        Line."Batch Name" := B; Line."Line No." := N; Line."No." := No; Line.Insert();
    end;
    local procedure Switch(UpdTrue: Boolean): Text
    var W: TestPage "NL Ws"; L: Codeunit "NL Log"; Line: Record "NL Line"; s: Text;
    begin
        Line.DeleteAll();
        Add('B2', 10000, 'B'); Add('B2', 20000, 'D');
        L.SetUpdTrue(UpdTrue);
        W.OpenEdit(); L.Take();
        W.CurrentBatch.SetValue('B2');
        s := L.Take() + '|' + W."Line No.".Value() + '/' + W."No.".Value();
        s += '|' + Format(W.Next()) + '/' + W."Line No.".Value() + ':' + L.Take();
        W.Close();
        exit(s + '|' + Format(Line.Count()));
    end;
    procedure UpdFalse(): Text begin exit(Switch(false)); end;
    procedure UpdTrue(): Text begin exit(Switch(true)); end;
    procedure NoIns(): Text
    var W: TestPage "NL Ws NoIns"; L: Codeunit "NL Log";
    begin
        L.Take();
        W.OpenEdit();
        exit(W."Line No.".Value() + ':' + L.Take());
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn a_switch_from_the_new_line_to_a_batch_with_rows_reads_as_from_a_row() {
    assert_eq!(
        run("UpdFalse"),
        "V:start 0;V:done 10000;R:B2/10000;R:B2/20000;R:B2/10000;R:B2/10000;C:B2/10000;|10000/B\
         |Yes/20000:R:B2/20000;R:B2/20000;C:B2/20000;|2"
    );
}

#[test]
fn update_true_from_the_new_line_modifies_the_row_find_moved_rec_onto() {
    assert_eq!(
        run("UpdTrue"),
        "V:start 0;M:B2/10000;L.OnModify:B2/10000;V:done 10000;R:B2/10000;R:B2/20000;R:B2/10000;R:B2/10000;C:B2/10000;|10000/B\
         |Yes/20000:R:B2/20000;R:B2/20000;C:B2/20000;|2"
    );
}

#[test]
fn an_empty_worksheet_without_insert_runs_on_a_blank_record() {
    assert_eq!(run("NoIns"), ":N:B3/0;C:B3/0;");
}
