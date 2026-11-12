//! Page view forms next to the pinned ones of pages/view-filters/ViewConstBlankCode.al (al-runner issue 7371) that no test pins
//! keep the behavior they had before: a SourceTableView member filter read after the test set its own filter on the field, or on
//! a page run with a record, and an Option `const` written as a number, reads the member's name as before.

use al_testkit::{run_in, text};

const SRC: &str = r#"
enum 69930 "VFU Kind" { value(0; Quote) { } value(1; Order) { } }
table 69931 "VFU Row"
{
    fields { field(1; Id; Integer) { } field(2; Kind; Option) { OptionMembers = None,Item,Res; } field(3; DocType; Enum "VFU Kind") { } field(4; TableNo; Integer) { } }
    keys { key(PK; Id) { } }
}
page 69932 "VFU OptConst"
{
    PageType = List; SourceTable = "VFU Row"; SourceTableView = where(Kind = const(Item));
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
}
page 69933 "VFU EnumFilter"
{
    PageType = List; SourceTable = "VFU Row"; SourceTableView = where(DocType = filter(Order));
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
}
page 69937 "VFU OptConstNo"
{
    PageType = List; SourceTable = "VFU Row"; SourceTableView = where(Kind = const(1));
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
}
codeunit 69936 C
{
    local procedure Seed()
    var V: Record "VFU Row"; I: Integer;
    begin
        for I := 1 to 4 do begin
            V.Id := I; V.Kind := I mod 3; V.DocType := "VFU Kind".FromInteger(I mod 2); V.TableNo := 69931 + (I mod 2); V.Insert();
        end;
    end;

    procedure OptConstOpen(): Text
    var P: TestPage "VFU OptConst";
    begin
        Seed();
        P.OpenView();
        exit(P.Filter.GetFilter(Kind));
    end;

    procedure OptConstNumberOpen(): Text
    var P: TestPage "VFU OptConstNo";
    begin
        Seed();
        P.OpenView();
        exit(P.Filter.GetFilter(Kind));
    end;

    procedure OptConstSetThenRead(): Text
    var P: TestPage "VFU OptConst";
    begin
        Seed();
        P.OpenView();
        P.Filter.SetFilter(Kind, 'Item');
        exit(P.Filter.GetFilter(Kind));
    end;

    procedure EnumFilterSetThenRead(): Text
    var P: TestPage "VFU EnumFilter";
    begin
        Seed();
        P.OpenView();
        P.Filter.SetFilter(DocType, 'Order');
        exit(P.Filter.GetFilter(DocType));
    end;

    procedure OptConstRun(): Text
    var P: TestPage "VFU OptConst"; V: Record "VFU Row";
    begin
        Seed();
        P.Trap();
        V.SetRange(Id, 1, 4);
        Page.Run(Page::"VFU OptConst", V);
        exit(P.Filter.GetFilter(Kind));
    end;

}
"#;

/// pages/view-filters/ViewConstBlankCode.al:490 (an Option `const` of the view, read right after OpenView): the member's number.
#[test]
fn a_view_member_read_after_open_view_is_its_number() {
    assert_eq!(text(run_in(SRC, "C", "OptConstOpen")), "1");
}

#[test]
fn a_view_member_read_after_the_tests_own_filter_keeps_the_name() {
    assert_eq!(text(run_in(SRC, "C", "OptConstSetThenRead")), "Item");
    assert_ne!(text(run_in(SRC, "C", "EnumFilterSetThenRead")), "1");
}

#[test]
fn a_view_member_read_on_a_page_run_with_a_record_keeps_the_name() {
    assert_eq!(text(run_in(SRC, "C", "OptConstRun")), "Item");
}

#[test]
fn a_view_option_const_written_as_a_number_keeps_the_name() {
    assert_eq!(text(run_in(SRC, "C", "OptConstNumberOpen")), "Item");
}
