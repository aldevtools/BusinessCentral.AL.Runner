//! The caption of a list page whose record has a filter on a data caption field (al-runner issue 5593), pinned by
//! pages/lt-y37-list-caption/Y37ListCaptionTests.al: only the page's DataCaptionFields count, and only a filter on one
//! value adds it (`- Z1`, two fields joined by ` ∙ `); a range, a list, a wildcard or a blank value leaves the caption
//! alone. A Date caption field with one date filtered fails on BC's test client and stays refused.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 61950 "Probe Bin"
{
    DataCaptionFields = "Code";
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Zone Code"; Code[10]) { }
        field(3; "Area"; Code[10]) { }
        field(4; Dt; Date) { }
        field(5; Kind; Option) { OptionMembers = A,B; }
    }
    keys { key(PK; "Code") { } }
}
page 61951 "Probe Bin List"
{
    Caption = 'Probe Bin List'; DataCaptionFields = "Zone Code", "Area"; PageType = List; SourceTable = "Probe Bin"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } field("Zone Code"; Rec."Zone Code") { } } } }
}
page 61953 "Probe Bin Plain"
{
    Caption = 'Probe Bin Plain'; PageType = List; SourceTable = "Probe Bin"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } } } }
}
page 61954 "Probe Bin Dates"
{
    Caption = 'Probe Bin Dates'; DataCaptionFields = Dt; PageType = List; SourceTable = "Probe Bin"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } } } }
}
page 61955 "Probe Bin Kinds"
{
    Caption = 'Probe Bin Kinds'; DataCaptionFields = Kind; PageType = List; SourceTable = "Probe Bin"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } } } }
}
codeunit 61952 C
{
    local procedure Seed(var Bin: Record "Probe Bin")
    begin
        Bin.DeleteAll();
        Bin."Code" := 'B1';
        Bin."Zone Code" := 'Z1';
        Bin."Area" := 'A1';
        Bin.Insert();
        Bin.Reset();
    end;

    local procedure Cap(var Bin: Record "Probe Bin"): Text
    var
        L: TestPage "Probe Bin List";
    begin
        L.Trap();
        Page.Run(Page::"Probe Bin List", Bin);
        exit(L.Caption());
    end;

    procedure ZoneFilter(): Text
    var
        Bin: Record "Probe Bin";
    begin
        Seed(Bin);
        Bin.SetRange("Zone Code", 'Z1');
        exit(Cap(Bin));
    end;

    procedure Kinds(): Text
    var
        Bin: Record "Probe Bin";
        T: Text;
    begin
        Seed(Bin);
        Bin.SetRange("Zone Code", 'Z1');
        Bin.SetRange("Area", 'A1');
        T := Cap(Bin);
        Bin.Reset();
        Bin.SetFilter("Zone Code", 'Z1|Z2');
        T += '|' + Cap(Bin);
        Bin.SetFilter("Zone Code", 'Z*');
        T += '|' + Cap(Bin);
        Bin.SetRange("Zone Code", '');
        T += '|' + Cap(Bin);
        Bin.SetFilter("Zone Code", '%1', 'Z2');
        T += '|' + Cap(Bin);
        Bin.Reset();
        Bin.FilterGroup(2);
        Bin.SetRange("Area", 'A1');
        Bin.FilterGroup(0);
        T += '|' + Cap(Bin);
        exit(T);
    end;

    procedure TableFieldOnly(): Text
    var
        Bin: Record "Probe Bin";
        L: TestPage "Probe Bin Plain";
    begin
        Seed(Bin);
        Bin.SetRange("Code", 'B1');
        L.Trap();
        Page.Run(Page::"Probe Bin Plain", Bin);
        exit(L.Caption());
    end;

    procedure Option(): Text
    var
        Bin: Record "Probe Bin";
        L: TestPage "Probe Bin Kinds";
    begin
        Seed(Bin);
        Bin.SetRange(Kind, Bin.Kind::B);
        L.Trap();
        Page.Run(Page::"Probe Bin Kinds", Bin);
        exit(L.Caption());
    end;

    procedure DateFilter(): Text
    var
        Bin: Record "Probe Bin";
        L: TestPage "Probe Bin Dates";
    begin
        Seed(Bin);
        Bin.SetRange(Dt, DMY2Date(5, 3, 2024));
        L.Trap();
        Page.Run(Page::"Probe Bin Dates", Bin);
        exit(L.Caption());
    end;

    procedure NoFilter(): Text
    var
        Bin: Record "Probe Bin";
    begin
        Seed(Bin);
        exit(Cap(Bin));
    end;
}
"#;

#[test]
fn a_list_caption_adds_the_one_value_of_a_page_data_caption_field() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "ZoneFilter")), "Probe Bin List - Z1");
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Kinds")),
        "Probe Bin List - Z1 \u{2219} A1|Probe Bin List|Probe Bin List|Probe Bin List|Probe Bin List - Z2|Probe Bin List - A1"
    );
    let bare: Result<Value, Signal> = al_testkit::run_in(SRC, "C", "NoFilter");
    assert_eq!(al_testkit::text(bare), "Probe Bin List");
}

#[test]
fn the_table_data_caption_fields_do_not_count_on_a_list() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "TableFieldOnly")), "Probe Bin Plain");
}

#[test]
fn an_option_caption_field_shows_the_member_name() {
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "Option")), "Probe Bin Kinds - B");
}

#[test]
fn a_date_caption_field_with_one_date_filtered_is_refused() {
    match al_testkit::run_in(SRC, "C", "DateFilter") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("data caption field"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
