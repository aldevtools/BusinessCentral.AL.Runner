//! TestFilter.GetFilter of an Enum field filtered to a range of two values reads as the two ordinals
//! (pages/lt-cd-testpage-leads/CdTestPageLeads.al:923 SH_TestPageFilter_GetFilter_EnumRangeAndList), a range over more values
//! and the forms around it read as pages/lt-y76-view-filters/Y76TestFilterEnum.al pins them.

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
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "0..1|1..2|0..1|<>1|First|Third"
    );
}

fn with_filter(call: &str) -> String {
    SRC.replace(
        "Row.SetRange(Kind, Row.Kind::First, Row.Kind::Second); s := Show(Row);",
        call,
    )
    .replace(
        "        Row.SetRange(Kind, Row.Kind::Second, Row.Kind::Third); s += '|' + Show(Row);\n",
        "",
    )
    .replace(
        "        Row.SetFilter(Kind, 'First..Second'); s += '|' + Show(Row);\n",
        "",
    )
    .replace(
        "        Row.SetFilter(Kind, '<>%1', Row.Kind::Second); s += '|' + Show(Row);\n",
        "",
    )
    .replace(
        "        Row.SetFilter(Kind, '%1|%2', Row.Kind::First, Row.Kind::Third); s += '|' + Show(Row);\n",
        "",
    )
}

// Y76_SetRange_ThreeAndMoreMembers (Y76TestFilterEnum.al:122): a range over more than two values reads as the two ordinals.
#[test]
fn a_range_over_three_values_reads_as_ordinals() {
    let src = with_filter("Row.SetRange(Kind, Row.Kind::First, Row.Kind::Third); s := Show(Row);");
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "F")), "0..2");
}

// Y76_SetFilter_TextAndOpenRanges (:147): an open range keeps the ordinal of its one end; `>`, `>=`, `<`, `<=` read as the
// list of the members they keep.
#[test]
fn open_ranges_read_as_ordinals_and_comparisons_as_the_members_they_keep() {
    for (filter, want) in [
        ("'%1..', Row.Kind::Second", "1.."),
        ("'..%1', Row.Kind::Second", "..1"),
        ("'>%1', Row.Kind::First", "Second|Third"),
        ("'>=%1', Row.Kind::Second", "Second|Third"),
        ("'<%1', Row.Kind::Third", "First|Second"),
        ("'<=%1', Row.Kind::Second", "First|Second"),
    ] {
        let src = with_filter(&format!("Row.SetFilter(Kind, {filter}); s := Show(Row);"));
        assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "F")), want, "{filter}");
    }
}

// Y76_SetFilter_ListsAndNegations (:179): a list with a range reads every part as an ordinal; a list of single values keeps
// the names; `<>` parts joined by `&` read as ordinals.
#[test]
fn a_list_with_a_range_reads_as_ordinals() {
    for (filter, want) in [
        (
            "'%1..%2|%3', Row.Kind::First, Row.Kind::Second, Row.Kind::Third",
            "0..1|2",
        ),
        (
            "'%1|%2..%3', Row.Kind::First, Row.Kind::Second, Row.Kind::Third",
            "0|1..2",
        ),
        ("'%1|%2', Row.Kind::First, Row.Kind::Third", "First|Third"),
        ("'<>%1&<>%2', Row.Kind::First, Row.Kind::Second", "<>0&<>1"),
    ] {
        let src = with_filter(&format!("Row.SetFilter(Kind, {filter}); s := Show(Row);"));
        assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "F")), want, "{filter}");
    }
}
