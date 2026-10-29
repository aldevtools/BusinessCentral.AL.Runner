//! Arithmetic with an Enum operand takes the Enum as its ordinal (al-runner issue 6156; corpus
//! types/lt-uf-enum-arithmetic/UfEnumArithmetic.al, writer UF branch al-runtime-rs-staging-uf, commit 3d7d7b66).

use al_testkit::text;

const SRC: &str = r#"
enum 69790 "EA Usage" { Extensible = true; value(0; " ") { } value(1; Quote) { } value(2; Order) { } value(3; Invoice) { } }
table 69791 "EA Row" { fields { field(1; Code; Code[20]) { } field(2; Usage; Enum "EA Usage") { } field(3; Other; Enum "EA Usage") { } } keys { key(PK; Code) { } } }
codeunit 69792 C
{
    procedure Ops(): Text
    var U: Enum "EA Usage"; R: Integer; T: Text;
    begin
        U := U::Invoice;
        R := U - 1; T += Format(R) + ';';
        R := U + 1; T += Format(R) + ';';
        R := U * 2; T += Format(R) + ';';
        R := U div 2; T += Format(R) + ';';
        R := U mod 2; T += Format(R) + ';';
        R := 10 - U; T += Format(R) + ';';
        R := U - U::Quote; T += Format(R) + ';';
        R := U + U; T += Format(R) + ';';
        U := U::Quote;
        R := U - 5; T += Format(R);
        exit(T);
    end;

    procedure Fields(): Text
    var Row: Record "EA Row"; R: Integer; T: Text;
    begin
        Row.Usage := Row.Usage::Invoice;
        R := Row.Usage * 3; T += Format(R) + ';';
        Row.Other := Row.Other::Order;
        R := Row.Usage - Row.Other; T += Format(R) + ';';
        T += Format(Row.Usage - 1 = Row.Usage::Order) + ';';
        T += Format(Row.Other + 1 = Row.Usage) + ';';
        T += Format(Row.Usage + 1 > 3);
        exit(T);
    end;

    procedure Into(): Text
    var Row: Record "EA Row"; U: Enum "EA Usage"; Back: Enum "EA Usage"; D: Decimal; T: Text;
    begin
        U := U::Invoice;
        D := U - 1; T += Format(D) + ';';
        D := U / 2; T += Format(D) + ';';
        Back := U - 1; T += Format(Back) + ';' + Format(Back.AsInteger()) + ';';
        Row.Usage := U - 2; T += Format(Row.Usage) + ';';
        Row.Usage := Row.Usage + 1; T += Format(Row.Usage) + ';';
        T += Format(U - 1) + '/' + Format(U + 1);
        exit(T);
    end;
}
"#;

#[test]
fn enum_operands_take_their_ordinal() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Ops")), "2;4;6;1;1;7;2;6;-4");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Fields")), "9;1;Yes;Yes;Yes");
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Into")),
        "2;1.5;Order;2;Quote;Order;2/4"
    );
}
