//! A ListPart that declares OnFindRecord (answering Rec.Find(Which), as Base Application's Sales Order Subform does), with
//! UpdatePropagation = Both, on a row other than its first, after CurrPage.Update(false) in a control's OnValidate: it stays on
//! its row; OnFindRecord runs with '=>' before the part's first read and with '=' before its last (al-runner issue 7099; corpus
//! branch al-runtime-rs-staging-r6977, pages/parts/R7099PartFindRecordBoth.al).

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 69950 "PFB Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Reset() begin Entries := ''; end;
    procedure Add(E: Text) begin if Entries <> '' then Entries += ';'; Entries += E; end;
    procedure Get(): Text begin exit(Entries); end;
}
table 69951 "PFB Doc"
{
    fields { field(1; "No."; Code[20]) { } field(2; Info; Text[30]) { } }
    keys { key(PK; "No.") { } }
}
table 69952 "PFB Line"
{
    fields { field(1; "Document No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Note; Text[30]) { } field(4; Qty; Decimal) { } field(5; "Bin Code"; Code[20]) { } }
    keys { key(PK; "Document No.", "Line No.") { } }
}
page 69953 "PFB Part"
{
    PageType = ListPart; SourceTable = "PFB Line"; InsertAllowed = false;
    layout { area(Content) { repeater(G) {
        field("Line No."; Rec."Line No.") { ApplicationArea = All; }
        field(Note; Rec.Note) { ApplicationArea = All; trigger OnValidate() var L: Codeunit "PFB Log"; begin L.Add('N.V'); CurrPage.Update(false); L.Add('N.V.end'); end; }
        field(Qty; Rec.Qty) { ApplicationArea = All; trigger OnValidate() var H: Record "PFB Doc"; L: Codeunit "PFB Log"; begin L.Add('Q.V'); H.Get(Rec."Document No."); H.Info := 'changed'; H.Modify(); CurrPage.Update(false); L.Add('Q.V.end'); end; }
        field("Bin Code"; Rec."Bin Code") { ApplicationArea = All; }
    } } }
    var Total: Decimal;
    trigger OnFindRecord(Which: Text): Boolean var L: Codeunit "PFB Log"; begin Clear(Total); L.Add('F(' + Which + ')'); exit(Rec.Find(Which)); end;
    trigger OnAfterGetRecord() var L: Codeunit "PFB Log"; begin L.Add('R' + Format(Rec."Line No." div 10000)); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "PFB Log"; begin L.Add('C' + Format(Rec."Line No." div 10000)); end;
}
page 69954 "PFB Host"
{
    PageType = Document; SourceTable = "PFB Doc";
    layout { area(Content) {
        field("No."; Rec."No.") { ApplicationArea = All; }
        field(Info; Rec.Info) { ApplicationArea = All; }
        part(Lines; "PFB Part") { ApplicationArea = All; SubPageLink = "Document No." = field("No."); UpdatePropagation = Both; }
    } }
    trigger OnAfterGetRecord() var L: Codeunit "PFB Log"; begin L.Add('HR'); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "PFB Log"; begin L.Add('HC'); end;
}
codeunit 69955 C
{
    local procedure Seed()
    var H: Record "PFB Doc"; L: Record "PFB Line";
    begin
        H."No." := 'D1'; H.Insert();
        L."Document No." := 'D1';
        L."Line No." := 10000; L.Insert();
        L."Line No." := 20000; L.Insert();
        L."Line No." := 30000; L.Insert();
    end;
    procedure Run(Nav: Integer; Ctl: Integer): Text
    var P: TestPage "PFB Host"; Log: Codeunit "PFB Log"; L: Record "PFB Line"; R: Text;
    begin
        Seed();
        P.OpenEdit(); P.GoToKey('D1');
        P.Lines.First(); P.Lines.Next();
        if Nav = 2 then P.Lines.Next();
        Log.Reset();
        if Ctl = 1 then P.Lines.Note.SetValue('A') else P.Lines.Qty.SetValue('5');
        R := P.Lines."Line No.".Value() + ' ' + Log.Get();
        Log.Reset();
        P.Lines."Bin Code".SetValue('Z');
        R += ' bin ' + P.Lines."Line No.".Value() + ' [' + Log.Get() + ']';
        P.Close();
        L.SetRange("Bin Code", 'Z'); L.FindFirst();
        exit(R + ' stored ' + Format(L."Line No."));
    end;
    procedure Line2Note(): Text begin exit(Run(1, 1)); end;
    procedure Line3Qty(): Text begin exit(Run(2, 2)); end;
}
"#;

#[test]
fn a_part_with_onfindrecord_stays_on_line_2_after_update_in_onvalidate() {
    // R7099PartFindRecordBoth.al PG7099_Doc_Line2_Note
    assert_eq!(
        text(run_in(SRC, "C", "Line2Note")),
        "20000 N.V;N.V.end;HR;HC;F(=>);R2;R3;R2;R1;R2;F(=);R2;C2 bin 20000 [] stored 20000"
    );
}

#[test]
fn a_part_with_onfindrecord_stays_on_line_3_after_a_change_of_the_host_row() {
    // R7099PartFindRecordBoth.al PG7099_Doc_Line3_QtyChangesHost
    assert_eq!(
        text(run_in(SRC, "C", "Line3Qty")),
        "30000 Q.V;Q.V.end;HR;HC;F(=>);R3;R2;R1;R3;F(=);R3;C3 bin 30000 [] stored 30000"
    );
}
