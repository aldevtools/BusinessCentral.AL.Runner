//! A list filtered to one template and batch (key fields, filter group 2 or 0) loses its only row to an action: the blank line
//! takes the filter values, validated, before OnNewRecord (Rec and xRec hold them), three times; a value typed there is inserted
//! at Close with Line No. 10000. Opened on no row, the list runs the same steps twice, xRec blank at each OnNewRecord.
//! Pinned by testpage/PnGRefresh.al:672 QL_Jnl_SheetFG2_DeleteNoUpdate_OnlyRow, :702 _DeleteWithUpdate_OnlyRow,
//! :732 QL_Jnl_ListFG2_DeleteWithUpdate_OnlyRow, :762 QL_Jnl_SheetFG0_DeleteWithUpdate_OnlyRow, :792 QL_Jnl_SheetFG2_OpenOnNoRow
//! (corpus bump 42). Tests-ERM 134002 ApplyAndPostPaymentToInvoice posts a General Journal this way.

const SRC: &str = r#"
table 51470 "GK Line"
{
    fields
    {
        field(1; Batch; Code[10]) { trigger OnValidate() var L: Codeunit "GK Log"; begin L.Add('V(' + Batch + ')'); end; }
        field(2; "Line No."; Integer) { }
        field(3; Descr; Text[30]) { }
    }
    keys { key(PK; Batch, "Line No.") { } }
    trigger OnInsert() var L: Codeunit "GK Log"; begin L.Add('INS(' + Batch + '/' + Format("Line No.") + ',' + Descr + ')'); end;
}
codeunit 51473 "GK Log"
{
    SingleInstance = true;
    var Buf: Text;
    procedure Add(T: Text) begin if Buf <> '' then Buf += ';'; Buf += T; end;
    procedure Take(): Text var R: Text; begin R := Buf; Buf := ''; exit(R); end;
}
page 51471 "GK List"
{
    PageType = Worksheet;
    SourceTable = "GK Line";
    AutoSplitKey = true;
    DelayedInsert = true;
    layout { area(Content) { repeater(R) { field(BatchF; Rec.Batch) { } field(LineF; Rec."Line No.") { } field(DescrF; Rec.Descr) { } } } }
    actions { area(Processing) { action(Post) { trigger OnAction() begin Rec.Delete(); end; } } }
    var L: Codeunit "GK Log";
    trigger OnOpenPage() begin Rec.FilterGroup(2); Rec.SetRange(Batch, 'B1'); Rec.FilterGroup(0); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin L.Add('NEW(' + Rec.Batch + ' x=' + xRec.Batch + ')'); end;
    trigger OnAfterGetCurrRecord() begin L.Add('AGCR(' + Rec.Batch + ' x=' + xRec.Batch + ')'); end;
}
codeunit 51472 "GK Test"
{
    var L: Codeunit "GK Log";

    local procedure Rows(): Text
    var G: Record "GK Line"; T: Text;
    begin
        if G.FindSet() then repeat T += G.Batch + '/' + Format(G."Line No.") + ':' + G.Descr + ' '; until G.Next() = 0;
        exit(T);
    end;

    procedure Delete(): Text
    var G: Record "GK Line"; P: TestPage "GK List"; T: Text;
    begin
        G.Batch := 'B2'; G."Line No." := 10000; G.Insert();
        G.Batch := 'B1'; G."Line No." := 10000; G.Insert();
        P.OpenEdit();
        L.Take();
        P.Post.Invoke();
        T := L.Take() + '|' + P.BatchF.Value() + '/' + P.LineF.Value();
        P.DescrF.SetValue('typed');
        P.Close();
        exit(T + '|' + L.Take() + '|' + Rows());
    end;

    procedure OpenOnNoRow(): Text
    var G: Record "GK Line"; P: TestPage "GK List";
    begin
        G.Batch := 'B2'; G."Line No." := 10000; G.Insert();
        L.Take();
        P.OpenEdit();
        exit(L.Take());
    end;
}
"#;

#[test]
fn a_key_filtered_list_that_loses_its_only_row_shows_a_blank_line_with_the_filter_values() {
    let got = al_testkit::text(al_testkit::run_in(SRC, "GK Test", "Delete"));
    let step = "V(B1);NEW(B1 x=B1);AGCR(B1 x=B1)";
    assert_eq!(
        got,
        format!("{step};{step};{step}|B1/0|INS(B1/10000,typed)|B1/10000:typed B2/10000: ")
    );
}

#[test]
fn opened_on_no_row_it_runs_the_steps_twice_with_xrec_blank_at_each_new_record() {
    let got = al_testkit::text(al_testkit::run_in(SRC, "GK Test", "OpenOnNoRow"));
    assert_eq!(got, "V(B1);NEW(B1 x=);AGCR(B1 x=B1);V(B1);NEW(B1 x=);AGCR(B1 x=B1)");
}
