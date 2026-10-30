//! `RecordRef.SetTable(V)` with a RecordRef in the Variant V (recordref/lt-wc/WcSetTableVariant.al, al-runner issue 6308): the
//! receiver itself is a no-op, another RecordRef of the same table takes a copy of the filters and the current row, another
//! table, a RecordRef that is not open and an empty Variant raise.

use al_testkit::text;

const SRC: &str = r#"
table 50490 "STV Doc" { fields { field(1; "No."; Code[20]) { } field(2; Amount; Decimal) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 50491 "STV Other" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
codeunit 50490 C
{
    local procedure Seed()
    var D: Record "STV Doc"; O: Record "STV Other";
    begin
        D.DeleteAll(); O.DeleteAll();
        D."No." := 'A'; D.Insert(); D."No." := 'B'; D.Insert(); D."No." := 'C'; D.Insert();
        O.Id := 7; O.Insert();
    end;
    local procedure Info(var RR: RecordRef): Text
    begin
        if RR.Number() = 0 then exit('closed');
        exit(RR.GetFilters() + '|' + Format(RR.Count()) + '|' + Format(RR.Field(1).Value));
    end;
    local procedure VInfo(var V: Variant): Text
    var RR: RecordRef;
    begin
        RR := V;
        exit(Info(RR));
    end;
    [TryFunction] local procedure TrySet(var S: RecordRef; var V: Variant) begin S.SetTable(V); end;
    procedure Same(): Text
    var RR: RecordRef; V: Variant; Out: Text;
    begin
        Seed();
        RR.Open(Database::"STV Doc");
        V := RR;
        RR.FindLast();
        RR.SetRecFilter();
        RR.SetTable(V);
        Out := Info(RR) + '/' + VInfo(V);
        RR.Reset();
        exit(Out + '/' + VInfo(V));
    end;
    procedure OtherOfSameTable(): Text
    var S: RecordRef; T: RecordRef; V: Variant; Out: Text;
    begin
        Seed();
        S.Open(Database::"STV Doc");
        S.Field(1).SetFilter('B..C');
        S.FindFirst();
        T.Open(Database::"STV Doc");
        T.FindFirst();
        V := T;
        S.SetTable(V);
        Out := Info(T) + '/' + VInfo(V);
        S.Next();
        Out += '/' + Info(S) + '/' + Info(T);
        T.Reset();
        exit(Out + '/' + VInfo(V) + '/' + Info(S));
    end;
    procedure Errors(): Text
    var S: RecordRef; T: RecordRef; C: RecordRef; V: Variant; E: Variant; Out: Text;
    begin
        Seed();
        S.Open(Database::"STV Doc");
        S.FindFirst();
        T.Open(Database::"STV Other");
        T.FindFirst();
        V := T;
        if not TrySet(S, V) then Out := GetLastErrorText() + '|' + GetLastErrorCode();
        V := C;
        if not TrySet(S, V) then Out += '/' + GetLastErrorText() + '|' + GetLastErrorCode();
        Out += '/' + VInfo(V);
        if not TrySet(S, E) then Out += '/' + GetLastErrorText() + '|' + GetLastErrorCode();
        exit(Out);
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

/// WcSetTableVariant.al:72-100 WC_SetTable_VariantHoldsSameRecordRef.
#[test]
fn the_receiver_in_the_variant_is_a_no_op_and_stays_shared() {
    assert_eq!(run("Same"), "No.: C|1|C/No.: C|1|C/|3|C");
}

/// WcSetTableVariant.al:103-140 WC_SetTable_VariantHoldsOtherRecordRefOfSameTable.
#[test]
fn another_ref_of_the_same_table_takes_a_copy_of_filters_and_row() {
    assert_eq!(
        run("OtherOfSameTable"),
        "No.: B..C|2|B/No.: B..C|2|B/No.: B..C|2|C/No.: B..C|2|B/|3|B/No.: B..C|2|C"
    );
}

/// WcSetTableVariant.al:142-203: another table, a RecordRef that is not open, an empty Variant.
#[test]
fn other_table_closed_ref_and_empty_variant_raise() {
    assert_eq!(
        run("Errors"),
        "The table IDs do not match.|TableIdMismatch/Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavRecordRef to Microsoft.Dynamics.Nav.Runtime.INavRecordHandle.|Conversion/closed/Microsoft.Dynamics.Nav.Runtime.NavVariant variable not initialized.|NotInitialized"
    );
}
