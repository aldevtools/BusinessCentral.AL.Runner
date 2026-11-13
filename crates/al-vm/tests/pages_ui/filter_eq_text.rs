//! `TestPage.Filter.SetFilter` with a text that starts with `=`: Business Central sets no filter on a List the test opened
//! with OpenView, for a Boolean word or an Option member (corpus testpage/filters/TempListOffPageBooleanFilter.al, al-runner
//! issue 6924). These tests hold the shapes outside that cell to main's outcome: the filter is applied.

const SRC: &str = r#"
table 69990 "EQ Row"
{
    fields { field(1; Id; Integer) { } field(2; Flag; Boolean) { } field(3; Name; Code[10]) { } field(4; Kind; Option) { OptionMembers = Quote,"Order"; } }
    keys { key(PK; Id) { } }
}
page 69991 "EQ List"
{
    PageType = List; SourceTable = "EQ Row"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Flag; Rec.Flag) { } field(Name; Rec.Name) { } field(Kind; Rec.Kind) { } } } }
}
page 69992 "EQ Card"
{
    PageType = Card; SourceTable = "EQ Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } field(Flag; Rec.Flag) { } } }
}
codeunit 69993 C
{
    local procedure Seed()
    var Row: Record "EQ Row";
    begin
        Row.DeleteAll();
        Add(1, false, 'A', 0); Add(2, true, 'B', 1); Add(3, false, 'C', 0); Add(4, true, 'D', 1);
    end;
    local procedure Add(Id: Integer; Flag: Boolean; Name: Code[10]; Kind: Integer)
    var Row: Record "EQ Row";
    begin
        Row.Init(); Row.Id := Id; Row.Flag := Flag; Row.Name := Name; Row.Kind := Kind; Row.Insert();
    end;
    local procedure Walk(var P: TestPage "EQ List"): Text
    var Ids: Text;
    begin
        if not P.First() then exit('none');
        repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit(Ids);
    end;
    procedure ViewBoolean(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Flag, '=1');
        exit('[' + P.Filter.GetFilter(Flag) + ']' + Walk(P));
    end;
    procedure ViewOption(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Kind, '=Quote');
        exit('[' + P.Filter.GetFilter(Kind) + ']' + Walk(P));
    end;
    procedure ViewOptionSecond(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Kind, '=Order');
        exit(Walk(P));
    end;
    procedure EditBoolean(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenEdit(); P.Filter.SetFilter(Flag, '=1');
        exit(Walk(P));
    end;
    procedure ViewCode(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Name, '=B');
        exit(Walk(P));
    end;
    procedure ViewInteger(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Id, '=2');
        exit(Walk(P));
    end;
    procedure ViewBooleanOtherWord(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Flag, '=1|0');
        exit(Walk(P));
    end;
    procedure ViewBooleanAfterOwnFilter(): Text
    var P: TestPage "EQ List";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Flag, '0'); P.Filter.SetFilter(Flag, '=1');
        exit(Walk(P));
    end;
    procedure CardBoolean(): Text
    var P: TestPage "EQ Card";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Flag, '=1');
        exit(P.Id.Value());
    end;
}
"#;

fn run(proc: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", proc))
}

// The measured cell on a database list: no filter, every row (TempListOffPageBooleanFilter.al:516 and :447-449).
#[test]
fn an_eq_text_on_a_viewed_list_sets_no_filter() {
    assert_eq!(run("ViewBoolean"), "[]1,2,3,4,");
    assert_eq!(run("ViewOption"), "[]1,2,3,4,");
}

// Not measured: OpenEdit, a Code or Integer field, another text after the `=` (also a second Option member), a field the test
// already filtered, a Card.
// Each keeps main's filter.
#[test]
fn shapes_outside_the_cell_keep_the_filter() {
    assert_eq!(run("EditBoolean"), "2,4,");
    assert_eq!(run("ViewCode"), "2,");
    assert_eq!(run("ViewInteger"), "2,");
    assert_eq!(run("ViewBooleanOtherWord"), "1,2,3,4,");
    assert_eq!(run("ViewBooleanAfterOwnFilter"), "2,4,");
    assert_eq!(run("CardBoolean"), "2");
    assert_eq!(run("ViewOptionSecond"), "2,4,");
}
