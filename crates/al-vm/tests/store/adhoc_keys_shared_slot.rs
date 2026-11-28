//! Two records of one table with SetCurrentKey on fields no key starts with (two ad hoc keys on other fields) and the
//! same filters, which bound a declared key and not the ad hoc ones: each reads the rows in its own key's order, its
//! fields and then the primary key (record/keys-sorting/SetCurrentKeyUndeclared.al:83-116, with filters :188-212). On
//! 200 rows the runner reads them through another key's band and keeps the rows for later steps; the second record
//! read the first one's rows in the first key's order and stopped early (issue 9257).

use al_testkit::text;

const SRC: &str = r#"
table 69990 "AKS Entry"
{
    fields { field(1; Id; Integer) { } field(2; Grp; Integer) { } field(3; X; Integer) { } field(4; Y; Integer) { } }
    keys { key(PK; Id) { } key(ByGrp; Grp) { } }
}
codeunit 69991 C
{
    procedure Ids(var R: Record "AKS Entry"): Text
    var Out: Text;
    begin
        if R.FindSet() then
            repeat
                Out += Format(R.Id) + ',';
            until R.Next() = 0;
        exit(Out);
    end;

    procedure F(): Text
    var E: Record "AKS Entry"; ByX: Record "AKS Entry"; ByY: Record "AKS Entry"; i: Integer; FirstX: Text;
    begin
        for i := 1 to 200 do begin
            E.Id := i; E.Grp := i mod 4; E.X := (i * 7) mod 50; E.Y := (i * 13) mod 50; E.Insert();
        end;
        ByX.SetCurrentKey(X);
        ByX.SetRange(Grp, 1);
        ByY.SetCurrentKey(Y);
        ByY.SetRange(Grp, 1);
        FirstX := Ids(ByX);
        exit(FirstX + '|' + Ids(ByY));
    end;
}
"#;

/// The ids of the rows with `Grp = 1`, in the order of `field` (X or Y as the codeunit fills them), then `Id`.
fn expected(factor: i32) -> String {
    let mut ids: Vec<i32> = (1..=200).filter(|i| i % 4 == 1).collect();
    ids.sort_by_key(|i| ((i * factor) % 50, *i));
    ids.iter().map(|i| format!("{i},")).collect()
}

#[test]
fn two_ad_hoc_keys_with_the_same_filters_each_read_in_their_own_order() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "F")),
        format!("{}|{}", expected(7), expected(13))
    );
}
