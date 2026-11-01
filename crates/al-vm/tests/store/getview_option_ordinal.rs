//! `Record.GetView(false)` writes the filter of an Option field as its ordinal: `WHERE(Field1=1(1))` for
//! `SetRange(DocType, DocType::Order)` (record/lt-set-current-key/Y43SetAscendingFlowField.al:234, corpus PR 966).

use al_testkit::text;

const SRC: &str = r#"
table 69965 "GV Row"
{
    fields { field(1; DocType; Option) { OptionMembers = Quote,Order; } field(2; "No."; Code[20]) { } }
    keys { key(PK; DocType, "No.") { } }
}
codeunit 69966 "GV Probe"
{
    procedure View(): Text
    var R: Record "GV Row";
    begin
        R.SetRange(DocType, R.DocType::Order);
        exit(R.GetView(false));
    end;
}
"#;

#[test]
fn get_view_without_names_writes_an_option_filter_as_its_ordinal() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "GV Probe", "View")),
        "VERSION(1) SORTING(Field1,Field2) WHERE(Field1=1(1))"
    );
}
