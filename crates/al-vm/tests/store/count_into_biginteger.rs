//! A BigInteger FlowField with `CalcFormula = count(...)` (al-runner issue 6986): CalcFields gives the number of matching rows as
//! a BigInteger, with a const filter, a FlowFilter, no row, SetAutoCalcFields, a filter on the FlowField and FieldRef.CalcField
//! (corpus branch al-runtime-rs-staging-r6961, record/flowfields/R6986BigIntegerCount.al).

use al_testkit::text;

const SRC: &str = r#"
table 50841 "CB Doc Type"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; CreatedFilter; Boolean) { FieldClass = FlowFilter; }
        field(10; ToArchive; BigInteger) { FieldClass = FlowField; CalcFormula = count("CB Doc" where("Doc Type No." = field("No."), Created = const(false))); }
        field(11; AllBig; BigInteger) { FieldClass = FlowField; CalcFormula = count("CB Doc" where("Doc Type No." = field("No."))); }
        field(14; ByFilterBig; BigInteger) { FieldClass = FlowField; CalcFormula = count("CB Doc" where("Doc Type No." = field("No."), Created = field(CreatedFilter))); }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 50842 "CB Doc"
{
    fields { field(1; "No."; Code[20]) { } field(2; "Doc Type No."; Code[20]) { } field(3; Created; Boolean) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
codeunit 50843 C
{
    local procedure Seed()
    var
        T: Record "CB Doc Type";
        D: Record "CB Doc";
    begin
        T."No." := 'T1'; T.Insert(); T."No." := 'T2'; T.Insert(); T."No." := 'T3'; T.Insert();
        D."No." := 'D1'; D."Doc Type No." := 'T1'; D.Created := false; D.Insert();
        D."No." := 'D2'; D."Doc Type No." := 'T1'; D.Created := true; D.Insert();
        D."No." := 'D3'; D."Doc Type No." := 'T1'; D.Created := false; D.Insert();
        D."No." := 'D4'; D."Doc Type No." := 'T3'; D.Created := false; D.Insert();
        D."No." := 'D5'; D."Doc Type No." := 'T3'; D.Created := false; D.Insert();
    end;

    procedure Values(): Text
    var
        T: Record "CB Doc Type";
        V: Variant;
        R: Text;
    begin
        Seed();
        T.FindSet();
        repeat
            T.CalcFields(ToArchive, AllBig);
            V := T.ToArchive;
            R += T."No." + '=' + Format(T.ToArchive) + '/' + Format(T.AllBig) + '/' + Format(V.IsBigInteger()) + ';';
        until T.Next() = 0;
        exit(R);
    end;

    procedure FlowFilter(): Text
    var
        T: Record "CB Doc Type";
        R: Text;
    begin
        Seed();
        T.Get('T1');
        T.SetRange(CreatedFilter, true);
        T.CalcFields(ByFilterBig);
        R := Format(T.ByFilterBig);
        T.SetRange(CreatedFilter, false);
        T.CalcFields(ByFilterBig);
        R += ' ' + Format(T.ByFilterBig);
        T.SetRange(CreatedFilter);
        T.CalcFields(ByFilterBig);
        exit(R + ' ' + Format(T.ByFilterBig));
    end;

    procedure AutoAndFilter(): Text
    var
        T: Record "CB Doc Type";
        R: Text;
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        Seed();
        T.SetAutoCalcFields(ToArchive);
        T.FindSet();
        repeat
            R += Format(T.ToArchive) + ',';
        until T.Next() = 0;
        T.SetFilter(ToArchive, '>=%1', 2);
        R += ' n=' + Format(T.Count());
        T.SetRange(ToArchive, 0);
        R += ' zero=' + Format(T.Count());
        T.Reset();
        T.Get('T3');
        RecRef.GetTable(T);
        FldRef := RecRef.Field(10);
        FldRef.CalcField();
        exit(R + ' ' + Format(FldRef.Type) + ' ' + Format(FldRef.Value));
    end;
}
"#;

#[test]
fn count_into_a_biginteger_flowfield_gives_the_row_count_as_a_biginteger() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Values")),
        "T1=2/3/Yes;T2=0/0/Yes;T3=2/2/Yes;"
    );
}

#[test]
fn count_into_a_biginteger_flowfield_with_a_flowfilter() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "FlowFilter")), "1 2 3");
}

#[test]
fn count_into_a_biginteger_flowfield_auto_calc_filter_and_fieldref() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "AutoAndFilter")),
        "2,0,2, n=2 zero=1 BigInteger 2"
    );
}
