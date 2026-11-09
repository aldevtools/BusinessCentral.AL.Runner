//! `GetView(false)` writes the filter of an Enum field as its ordinal, after a SetView that names the value by its name, its
//! caption or its ordinal; `GetView(true)` writes the caption (corpus master format-evaluate/EnumCaptionReaders.al:1530-1551,
//! ECR_RecordRefSetViewEnumName_GetViewFalseWritesTheOrdinal; al-runner issue 7456).

use al_testkit::text;

const SRC: &str = r#"
enum 69970 "GVE Ml"
{
    Extensible = true;
    value(0; Open) { CaptionML = ENU = 'Open cap', NLB = 'Open BE'; }
    value(1; Closed) { CaptionML = ENU = 'Closed cap', NLB = 'Gesloten BE'; }
}
enum 69971 "GVE Mixed"
{
    Extensible = true;
    value(0; Alpha) { CaptionML = ENU = 'Alpha cap', NLB = 'Alpha BE'; }
    value(1; Beta) { Caption = 'Beta plain'; }
    value(2; Gamma) { }
}
table 69972 "GVE Row"
{
    fields { field(1; Id; Integer) { } field(2; Status; Enum "GVE Ml") { } field(3; Mixed; Enum "GVE Mixed") { } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69973 "GVE Probe"
{
    local procedure ViewTextOf(ViewText: Text; Localized: Boolean): Text
    var RecRef: RecordRef;
    begin
        RecRef.Open(Database::"GVE Row");
        RecRef.SetView(ViewText);
        exit(RecRef.GetView(Localized));
    end;

    procedure Views(): Text
    var Row: Record "GVE Row";
    begin
        Row.SetView('WHERE(field2=0(Closed))');
        exit(ViewTextOf('WHERE(field2=0(Closed))', false) + '/' + ViewTextOf('WHERE(field2=1(Closed))', false) + '/'
            + ViewTextOf('WHERE(field2=0(Closed cap))', false) + '/' + ViewTextOf('WHERE(field3=0(Beta))', false) + '/'
            + ViewTextOf('WHERE(field2=0(Closed))', true) + '/' + ViewTextOf('WHERE(field3=0(Beta))', true) + '/'
            + Row.GetView(false));
    end;
}
"#;

#[test]
fn get_view_without_names_writes_an_enum_filter_as_its_ordinal() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "GVE Probe", "Views")),
        [
            "VERSION(1) SORTING(Field1) WHERE(Field2=1(1))",
            "VERSION(1) SORTING(Field1) WHERE(Field2=1(1))",
            "VERSION(1) SORTING(Field1) WHERE(Field2=1(1))",
            "VERSION(1) SORTING(Field1) WHERE(Field3=1(1))",
            "VERSION(1) SORTING(Id) WHERE(Status=FILTER(Closed cap))",
            "VERSION(1) SORTING(Id) WHERE(Mixed=FILTER(Beta plain))",
            "VERSION(1) SORTING(Field1) WHERE(Field2=1(1))",
        ]
        .join("/")
    );
}
