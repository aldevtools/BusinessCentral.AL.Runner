//! `SetFilter` on a RecordId field with the text `Format(RecordId)` (filters/lt-wc/WcRecordIdFilter.al, al-runner issue 6309):
//! an equality filter on that RecordId, `<>` before it keeps the others, `GetFilter` writes the text back; a text that names no
//! table, or names it by its caption, raises the evaluate error.

use al_testkit::text;

const SRC: &str = r#"
table 50497 "RIF Doc" { Caption = 'RIF Document'; fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 50498 "RIF Line" { fields { field(1; "Doc No."; Code[20]) { } field(2; "Line No."; Integer) { } } keys { key(PK; "Doc No.", "Line No.") { Clustered = true; } } }
table 50499 "RIF Log" { fields { field(1; "Entry No."; Integer) { } field(2; "Record ID"; RecordId) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
codeunit 50497 C
{
    local procedure Seed()
    var D: Record "RIF Doc"; L: Record "RIF Line";
    begin
        D.DeleteAll(); L.DeleteAll(); Lg.DeleteAll();
        D."No." := 'A'; D.Insert(); Add(1, D.RecordId);
        D."No." := 'B'; D.Insert(); Add(2, D.RecordId);
        L."Doc No." := 'X Y,Z'; L."Line No." := 10000; L.Insert(); Add(3, L.RecordId);
    end;
    local procedure Add(No: Integer; Id: RecordId)
    begin
        Lg."Entry No." := No; Lg."Record ID" := Id; Lg.Insert();
    end;
    local procedure Ids(): Text
    var R: Text;
    begin
        if Lg.FindSet() then repeat R += Format(Lg."Entry No.") + ','; until Lg.Next() = 0;
        exit(R);
    end;
    [TryFunction] local procedure TryFilter(T: Text; var R: Text)
    begin
        Lg.Reset();
        Lg.SetFilter("Record ID", T);
        R := Lg.GetFilter("Record ID") + '=' + Ids();
    end;
    local procedure Probe(T: Text): Text
    var R: Text;
    begin
        if TryFilter(T, R) then exit(R);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure Run(): Text
    var D: Record "RIF Doc"; L: Record "RIF Line";
    begin
        Seed();
        D.Get('A');
        L.Get('X Y,Z', 10000);
        exit(Probe(Format(D.RecordId)) + '/' + Probe(Format(L.RecordId)) + '/' + Probe('<>' + Format(D.RecordId)) + '/' +
             Probe('RIF Line: X') + '/' + Probe('garbage') + '/' + Probe('RIF Document: A'));
    end;
    var Lg: Record "RIF Log";
}
"#;

/// WcRecordIdFilter.al:118-186.
#[test]
fn a_record_id_text_filters_on_that_record_id() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Run")),
        "RIF Doc: A=1,/RIF Line: \"X Y,Z\",10000=3,/<>RIF Doc: A=2,3,/RIF Line: X=/\
The filter \"garbage\" is not valid for the Record ID field on the RIF Log table. The value \"garbage\" can't be evaluated into type RecordID.|Filter:CSide/\
The filter \"RIF Document: A\" is not valid for the Record ID field on the RIF Log table. The value \"RIF Document: A\" can't be evaluated into type RecordID.|Filter:CSide"
    );
}
