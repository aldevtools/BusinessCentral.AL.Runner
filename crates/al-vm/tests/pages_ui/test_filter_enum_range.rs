//! TestFilter.GetFilter of an Enum field filtered to a range of two values reads as the two ordinals
//! (pages/lt-cd-testpage-leads/CdTestPageLeads.al:923 SH_TestPageFilter_GetFilter_EnumRangeAndList). A range over three
//! values is the registered assumption `test-filter-enum-range-ordinals`.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};

const SRC: &str = r#"
enum 51530 "EF Kind" { value(0; First) { } value(1; Second) { } value(2; Third) { } }
table 51531 "EF Row" { fields { field(1; Id; Integer) { } field(2; Kind; Enum "EF Kind") { } } keys { key(PK; Id) { } } }
page 51532 "EF List" { PageType = List; SourceTable = "EF Row"; layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } } }
codeunit 51533 C
{
    local procedure Show(var Row: Record "EF Row"): Text
    var P: TestPage "EF List"; s: Text;
    begin
        P.Trap(); Page.Run(Page::"EF List", Row); s := P.Filter.GetFilter(Kind); P.Close(); exit(s);
    end;
    procedure F(): Text
    var Row: Record "EF Row"; s: Text;
    begin
        Row.SetRange(Kind, Row.Kind::First, Row.Kind::Second); s := Show(Row);
        Row.SetRange(Kind, Row.Kind::Second, Row.Kind::Third); s += '|' + Show(Row);
        Row.SetFilter(Kind, 'First..Second'); s += '|' + Show(Row);
        Row.SetFilter(Kind, '<>%1', Row.Kind::Second); s += '|' + Show(Row);
        Row.SetFilter(Kind, '%1|%2', Row.Kind::First, Row.Kind::Third); s += '|' + Show(Row);
        exit(s);
    end;
}
"#;

#[test]
fn a_two_value_enum_range_reads_as_ordinals() {
    clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "0..1|1..2|0..1|<>1|First|Third"
    );
    assert!(!take_hits().contains(&Assumption::TestFilterEnumRangeOrdinals));
}

#[test]
fn a_three_value_enum_range_is_the_assumption() {
    let src = SRC.replace(
        "Row.SetRange(Kind, Row.Kind::First, Row.Kind::Second); s := Show(Row);",
        "Row.SetRange(Kind, Row.Kind::First, Row.Kind::Third); s := Show(Row);",
    );
    clear_hits();
    assert!(al_testkit::text(al_testkit::run_in(&src, "C", "F")).starts_with("0..2|"));
    assert!(take_hits().contains(&Assumption::TestFilterEnumRangeOrdinals));
}
