//! A Variant that held a record and is given another record of the same table holds the new record's view: what
//! `RecordRef.GetTable(V)` takes from it is the new record's filters, not the old one's (al-runner issue 6503, the
//! reporter's run on real BC, newest major: `first{count=1 filters=[No: A]} second{count=0 filters=[No: NONE]} back{count=1 filters=[]}`).

use al_testkit::text;

const SRC: &str = r#"
table 50495 "VGP Row" { fields { field(1; No; Code[20]) { } } keys { key(PK; No) { Clustered = true; } } }
codeunit 50496 Reader
{
    procedure ByVar(var V: Variant): Text
    var RR: RecordRef;
    begin
        RR.GetTable(V);
        exit(StrSubstNo('count=%1 filters=[%2]', RR.Count(), RR.GetFilters()));
    end;
    procedure ByValue(V: Variant): Text
    var RR: RecordRef;
    begin
        RR.GetTable(V);
        exit(StrSubstNo('count=%1 filters=[%2]', RR.Count(), RR.GetFilters()));
    end;
}
codeunit 50495 C
{
    local procedure Seed()
    var Row: Record "VGP Row";
    begin
        Row.DeleteAll();
        Row.No := 'A';
        Row.Insert();
    end;
    procedure Probe(): Text
    var First: Record "VGP Row"; Filtered: Record "VGP Row"; Back: Record "VGP Row"; V: Variant; RR: RecordRef; R1, R2, R3: Text;
    begin
        Seed();
        First.Get('A');
        First.SetRecFilter();
        V := First;
        RR.GetTable(V);
        R1 := StrSubstNo('count=%1 filters=[%2]', RR.Count(), RR.GetFilters());
        Filtered.SetRange(No, 'NONE');
        V := Filtered;
        Clear(RR);
        RR.GetTable(V);
        R2 := StrSubstNo('count=%1 filters=[%2]', RR.Count(), RR.GetFilters());
        Back := V;
        R3 := StrSubstNo('count=%1 filters=[%2]', Back.Count(), Back.GetFilters());
        exit(StrSubstNo('first{%1} second{%2} back{%3}', R1, R2, R3));
    end;
    procedure NoRecFilter(): Text
    var First: Record "VGP Row"; Filtered: Record "VGP Row"; V: Variant; RR: RecordRef;
    begin
        Seed();
        First.Get('A');
        V := First;
        Filtered.SetFilter(No, 'NONE');
        V := Filtered;
        RR.GetTable(V);
        exit(StrSubstNo('count=%1 filters=[%2]', RR.Count(), RR.GetFilters()));
    end;
    procedure ReuseFirst(): Text
    var First: Record "VGP Row"; V: Variant; RR: RecordRef;
    begin
        Seed();
        First.Get('A');
        First.SetRecFilter();
        V := First;
        First.Reset();
        First.SetRange(No, 'NONE');
        V := First;
        RR.GetTable(V);
        exit(StrSubstNo('count=%1 filters=[%2]', RR.Count(), RR.GetFilters()));
    end;
    procedure OtherCodeunit(): Text
    var First: Record "VGP Row"; Filtered: Record "VGP Row"; V: Variant; Rd: Codeunit Reader;
    begin
        Seed();
        First.Get('A');
        First.SetRecFilter();
        V := First;
        Filtered.SetRange(No, 'NONE');
        V := Filtered;
        exit(Rd.ByVar(V) + '/' + Rd.ByValue(V));
    end;
}
"#;

#[test]
fn get_table_of_a_variant_given_another_record_of_the_same_table_takes_the_new_view() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Probe")),
        "first{count=1 filters=[No: A]} second{count=0 filters=[No: NONE]} back{count=1 filters=[]}"
    );
}

#[test]
fn the_new_view_wins_without_set_rec_filter_and_with_set_filter() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "NoRecFilter")),
        "count=0 filters=[No: NONE]"
    );
}

#[test]
fn the_new_view_wins_when_the_same_record_is_reset_and_assigned_again() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "ReuseFirst")),
        "count=0 filters=[No: NONE]"
    );
}

#[test]
fn the_new_view_wins_in_another_codeunit_by_var_and_by_value() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "OtherCodeunit")),
        "count=0 filters=[No: NONE]/count=0 filters=[No: NONE]"
    );
}
