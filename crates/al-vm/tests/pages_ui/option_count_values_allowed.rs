//! `OptionCount` and `GetOption` of an Enum control: all the enum's values; with ValuesAllowed on the control, or on the table
//! field the control shows, only the allowed ones (misc/lt-y13-triage/Y13PageTriage.al:197-243, corpus master).

const SRC: &str = r#"
enum 69940 "OV Kind" { Extensible = false; value(0; Open) { } value(1; Released) { } value(2; Closed) { } value(3; Archived) { } }
table 69941 "OV Row" { fields { field(1; Id; Integer) { } field(2; Kind; Enum "OV Kind") { } } keys { key(PK; Id) { } } }
table 69942 "OV VA Row" { fields { field(1; Id; Integer) { } field(2; Kind; Enum "OV Kind") { ValuesAllowed = Open, Closed; } } keys { key(PK; Id) { } } }
page 69943 "OV Kinds"
{
    PageType = Card; SourceTable = "OV Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Plain; Rec.Kind) { } field(Limited; Rec.Kind) { ValuesAllowed = Open, Released; } } }
}
page 69944 "OV TableVA"
{
    PageType = Card; SourceTable = "OV VA Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Kind; Rec.Kind) { } } }
}
codeunit 69945 C
{
    procedure F(): Text
    var R: Record "OV Row"; V: Record "OV VA Row"; P: TestPage "OV Kinds"; T: TestPage "OV TableVA"; I: Integer; Out: Text;
    begin
        R.Id := 1; R.Insert(); V.Id := 1; V.Insert();
        P.OpenEdit();
        Out := Format(P.Plain.OptionCount()) + ':';
        for I := 1 to P.Plain.OptionCount() do Out += P.Plain.GetOption(I) + ',';
        Out += ' ' + Format(P.Limited.OptionCount()) + ':';
        for I := 1 to P.Limited.OptionCount() do Out += P.Limited.GetOption(I) + ',';
        T.OpenEdit();
        Out += ' ' + Format(T.Kind.OptionCount()) + ':';
        for I := 1 to T.Kind.OptionCount() do Out += T.Kind.GetOption(I) + ',';
        exit(Out);
    end;
}
"#;

#[test]
fn values_allowed_on_the_control_or_the_field_limits_the_options() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "4:Open,Released,Closed,Archived, 2:Open,Released, 2:Open,Closed,"
    );
}
