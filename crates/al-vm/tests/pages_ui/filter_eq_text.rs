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

// Not measured: OpenEdit, a Code or Integer field, another text after the `=`, a field the test
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
    // Another Option member after the `=` sets no filter either (corpus PR 1392, WorksheetEnumEqFilter.al:604-609 on a List).
    assert_eq!(run("ViewOptionSecond"), "1,2,3,4,");
}

/// A Worksheet and a List over a table with an Enum, for the cells of WorksheetEnumEqFilter.al (corpus PR 1392) and their neighbors.
const WS_SRC: &str = r#"
enum 69980 "EQW Type" { Extensible = false; value(0; Quote) { } value(1; "Order") { } value(2; Invoice) { } }
table 69981 "EQW Row"
{
    fields { field(1; Id; Integer) { } field(2; DocType; Enum "EQW Type") { } field(3; Kind; Option) { OptionMembers = Quote,"Order",Invoice; } field(4; IsHist; Boolean) { } field(5; Name; Code[10]) { } }
    keys { key(PK; Id) { } }
}
page 69982 "EQW Worksheet"
{
    PageType = Worksheet; SourceTable = "EQW Row"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(DocType; Rec.DocType) { } field(Kind; Rec.Kind) { } } } }
}
page 69983 "EQW Temp Worksheet"
{
    PageType = Worksheet; SourceTable = "EQW Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(DocType; Rec.DocType) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.Init(); Rec.Id := 1; Rec.DocType := Rec.DocType::Quote; Rec.Insert(); Rec.Id := 2; Rec.DocType := Rec.DocType::"Order"; Rec.Insert(); Rec.Id := 3; Rec.DocType := Rec.DocType::Invoice; Rec.Insert(); Rec.Id := 4; Rec.DocType := Rec.DocType::"Order"; Rec.Insert(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
}
page 69984 "EQW List"
{
    PageType = List; SourceTable = "EQW Row"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(DocType; Rec.DocType) { } } } }
}
codeunit 69985 C
{
    local procedure Seed()
    var Row: Record "EQW Row";
    begin
        Row.DeleteAll();
        Add(1, 0, false); Add(2, 1, false); Add(3, 2, true); Add(4, 1, true);
    end;
    local procedure Add(Id: Integer; T: Integer; H: Boolean)
    var Row: Record "EQW Row";
    begin
        Row.Init(); Row.Id := Id; Row.DocType := "EQW Type".FromInteger(T); Row.Kind := T; Row.IsHist := H; Row.Name := 'N' + Format(Id); Row.Insert();
    end;
    local procedure Walk(var P: TestPage "EQW Worksheet"): Text
    var Ids: Text;
    begin
        if not P.First() then exit('none');
        repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit(Ids);
    end;
    local procedure WalkTemp(var P: TestPage "EQW Temp Worksheet"): Text
    var Ids: Text;
    begin
        if not P.First() then exit('none');
        repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit(Ids);
    end;
    procedure WsEnum(FilterText: Text): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(DocType, FilterText);
        exit('[' + P.Filter.GetFilter(DocType) + ']' + Walk(P));
    end;
    procedure WsOption(FilterText: Text): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Kind, FilterText);
        exit('[' + P.Filter.GetFilter(Kind) + ']' + Walk(P));
    end;
    procedure WsBoolean(FilterText: Text): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(IsHist, FilterText);
        exit('[' + P.Filter.GetFilter(IsHist) + ']' + Walk(P));
    end;
    procedure WsCode(FilterText: Text): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(Name, FilterText);
        exit(Walk(P));
    end;
    procedure WsEditEnum(FilterText: Text): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenEdit(); P.Filter.SetFilter(DocType, FilterText);
        exit(Walk(P));
    end;
    procedure WsEnumAfterOwnFilter(): Text
    var P: TestPage "EQW Worksheet";
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(DocType, 'Order'); P.Filter.SetFilter(DocType, '=Quote');
        exit(Walk(P));
    end;
    procedure TempWsEnumAfterOwnFilter(): Text
    var P: TestPage "EQW Temp Worksheet";
    begin
        P.OpenView(); P.Fill.Invoke(); P.Filter.SetFilter(DocType, 'Order'); P.Filter.SetFilter(DocType, '=Quote');
        exit('[' + P.Filter.GetFilter(DocType) + ']' + WalkTemp(P));
    end;
    procedure ListEnum(FilterText: Text): Text
    var P: TestPage "EQW List"; Ids: Text;
    begin
        Seed(); P.OpenView(); P.Filter.SetFilter(DocType, FilterText);
        if P.First() then repeat Ids += P.Id.Value() + ','; until not P.Next();
        exit('[' + P.Filter.GetFilter(DocType) + ']' + Ids);
    end;
}
"#;

fn ws(proc: &str, arg: &str) -> String {
    let arg = al_builtins::value::Value::Text(arg.to_string().into());
    al_testkit::text(al_testkit::run_with(WS_SRC, "C", proc, vec![arg]).map(|(v, _)| v))
}

fn ws0(proc: &str) -> String {
    al_testkit::text(al_testkit::run_in(WS_SRC, "C", proc))
}

// WorksheetEnumEqFilter.al (corpus PR 1392): a text that starts with `=` sets no filter on a Worksheet (database :620-642,
// temporary :398-476) and a List (:575-614): Enum names and the ordinals 0 and 1, Option names and `=1`, Boolean `=1` and `=0`.
#[test]
fn an_eq_text_on_a_viewed_worksheet_or_list_sets_no_filter() {
    for t in ["=Quote", "=Order", "=Invoice", "=0", "=1"] {
        assert_eq!(ws("WsEnum", t), "[]1,2,3,4,", "{t}");
    }
    for t in ["=Quote", "=Order", "=Invoice", "=1"] {
        assert_eq!(ws("WsOption", t), "[]1,2,3,4,", "{t}");
    }
    for t in ["=1", "=0"] {
        assert_eq!(ws("WsBoolean", t), "[]1,2,3,4,", "{t}");
    }
    for t in ["=Quote", "=Order", "=Invoice"] {
        assert_eq!(ws("ListEnum", t), "[]1,2,3,4,", "{t}");
    }
}

// :545-569: on the temporary Worksheet an `=` text after a filter of the test keeps that filter.
#[test]
fn an_eq_text_keeps_the_earlier_filter_on_the_temporary_worksheet() {
    assert_eq!(ws0("TempWsEnumAfterOwnFilter"), "[Order]2,4,");
}

// Not measured: OpenEdit, a Code field, an ordinal other than 0 and 1, a name with another case, an earlier filter on the
// database Worksheet. Each keeps main's filter; `<>Quote` and a bare name keep filter.rs's parsing (:406, :408).
#[test]
fn worksheet_shapes_outside_the_cells_keep_the_filter() {
    assert_eq!(ws("WsEditEnum", "=Order"), "2,4,");
    assert_eq!(ws("WsCode", "=N2"), "2,");
    assert_eq!(ws("WsEnum", "=2"), "[Invoice]3,");
    assert_eq!(ws("WsEnum", "=order"), "[Order]2,4,");
    assert_eq!(ws0("WsEnumAfterOwnFilter"), "1,");
    assert_eq!(ws("WsEnum", "<>Quote"), "[<>0]2,3,4,");
    assert_eq!(ws("WsEnum", "Order"), "[Order]2,4,");
}
