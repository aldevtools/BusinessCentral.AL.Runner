//! A journal batch switch: an insert-allowed Worksheet (AutoSplitKey, DelayedInsert) whose header control's OnValidate calls
//! CurrPage.SaveRecord, sets the batch filter in filter group 2, moves Rec with Find('-') and calls CurrPage.Update(false) (the
//! General Journal shape). A batch with rows reads as the pinned refresh after a filter change; an empty batch shows the new-row
//! line, OnNewRecord and OnAfterGetCurrRecord twice (corpus master 97c29f86, pages/lt-y17-journal-batch/
//! Y17JournalBatchSwitch.al:466 UpdWs_BatchSwitch_ToBatchWithRows, :485 UpdWs_BatchSwitch_ToEmptyBatch).

const SRC: &str = r#"
table 51530 "JB Line"
{
    fields { field(1; "Batch Name"; Code[10]) { } field(2; "Line No."; Integer) { } field(3; "No."; Code[20]) { } }
    keys { key(PK; "Batch Name", "Line No.") { } }
}
codeunit 51531 "JB Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51532 "JB Ws"
{
    PageType = Worksheet; SourceTable = "JB Line"; AutoSplitKey = true; DelayedInsert = true;
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
                        CurrPage.Update(false);
                        L.Add('V:done ' + Format(Rec."Line No."));
                    end;
                }
            }
            repeater(Rows) { field("Line No."; Rec."Line No.") { } field("No."; Rec."No.") { } }
        }
    }
    var L: Codeunit "JB Log"; CurrentBatch: Code[10];
    trigger OnOpenPage() begin CurrentBatch := 'B1'; SetBatch(); end;
    trigger OnAfterGetRecord() begin L.Add('R:' + Rec."Batch Name" + '/' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C:' + Rec."Batch Name" + '/' + Format(Rec."Line No.")); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('N:' + Rec."Batch Name" + '/' + Format(Rec."Line No.") + ' below=' + Format(BelowxRec)); end;
    local procedure SetBatch()
    begin
        Rec.FilterGroup := 2; Rec.SetRange("Batch Name", CurrentBatch); Rec.FilterGroup := 0;
        if Rec.Find('-') then;
    end;
}
codeunit 51533 C
{
    local procedure Add(B: Code[10]; N: Integer; No: Code[20])
    var Line: Record "JB Line";
    begin
        Line."Batch Name" := B; Line."Line No." := N; Line."No." := No; Line.Insert();
    end;
    local procedure Switch(Batch: Code[10]): Text
    var W: TestPage "JB Ws"; L: Codeunit "JB Log"; Line: Record "JB Line"; s: Text;
    begin
        Line.DeleteAll();
        Add('B1', 10000, 'A'); Add('B1', 20000, 'C'); Add('B1', 30000, 'E'); Add('B2', 10000, 'B'); Add('B2', 20000, 'D');
        W.OpenEdit(); W.GoToKey('B1', 20000); L.Take();
        W.CurrentBatch.SetValue(Batch);
        s := L.Take() + '|' + W."Line No.".Value() + '/' + W."No.".Value();
        s += '|' + Format(W.Next()) + '/' + W."Line No.".Value() + ':' + L.Take();
        W.Close();
        exit(s + '|' + Format(Line.Count()));
    end;
    procedure ToRows(): Text begin exit(Switch('B2')); end;
    procedure ToEmpty(): Text begin exit(Switch('B3')); end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn a_switch_to_a_batch_with_rows_reads_them_as_the_refresh_after_a_filter_change() {
    assert_eq!(
        run("ToRows"),
        "V:start 20000;V:done 10000;R:B2/10000;R:B2/20000;R:B2/10000;R:B2/10000;C:B2/10000;|10000/B\
         |Yes/20000:R:B2/20000;R:B2/20000;C:B2/20000;|5"
    );
}

#[test]
fn a_switch_to_an_empty_batch_shows_the_new_row_line() {
    assert_eq!(
        run("ToEmpty"),
        "V:start 20000;V:done 20000;N:B3/0 below=Yes;C:B3/0;N:B3/0 below=Yes;C:B3/0;|0/|No/0:|5"
    );
}
