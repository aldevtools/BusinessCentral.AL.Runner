//! The caption of a list page whose record has a filter on a data caption field (al-runner issue 5593). BC 29 shows
//! `Probe Bin List - Z1` for a filter Zone Code = Z1 and the bare caption for no filter; no corpus test pins it yet, so
//! the caption with such a filter is refused, never answered bare.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 61950 "Probe Bin" { fields { field(1; "Code"; Code[20]) { } field(2; "Zone Code"; Code[10]) { } } keys { key(PK; "Code") { } } }
page 61951 "Probe Bin List"
{
    Caption = 'Probe Bin List'; DataCaptionFields = "Zone Code"; PageType = List; SourceTable = "Probe Bin"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } field("Zone Code"; Rec."Zone Code") { } } } }
}
codeunit 61952 C
{
    local procedure Seed(var Bin: Record "Probe Bin")
    begin
        Bin.DeleteAll();
        Bin."Code" := 'B1';
        Bin."Zone Code" := 'Z1';
        Bin.Insert();
        Bin.Reset();
    end;

    procedure ZoneFilter(): Text
    var
        Bin: Record "Probe Bin";
        L: TestPage "Probe Bin List";
    begin
        Seed(Bin);
        Bin.SetRange("Zone Code", 'Z1');
        L.Trap();
        Page.Run(Page::"Probe Bin List", Bin);
        exit(L.Caption());
    end;

    procedure NoFilter(): Text
    var
        Bin: Record "Probe Bin";
        L: TestPage "Probe Bin List";
    begin
        Seed(Bin);
        L.Trap();
        Page.Run(Page::"Probe Bin List", Bin);
        exit(L.Caption());
    end;
}
"#;

#[test]
fn a_list_caption_with_a_filter_on_a_data_caption_field_is_refused() {
    match al_testkit::run_in(SRC, "C", "ZoneFilter") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("data caption field"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    let bare: Result<Value, Signal> = al_testkit::run_in(SRC, "C", "NoFilter");
    assert_eq!(al_testkit::text(bare), "Probe Bin List");
}
