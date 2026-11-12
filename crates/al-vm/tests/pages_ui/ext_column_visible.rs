//! A repeater column whose Visible a page extension's modify() binds to a variable the extension sets in OnAfterGetRecord
//! (pages/extensions/PageExtModifyVisibleVariable.al:519-547). Only that shape counts the value after the open read's
//! OnAfterGetRecord; the neighbors keep the value frozen at open, as before.

use al_testkit::text;

const SRC: &str = r#"
table 69880 "ECV Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
}
page 69881 "ECV List"
{
    PageType = List; SourceTable = "ECV Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(ColRec; Rec.Name) { } } } }
}
pageextension 69882 "ECV List Ext" extends "ECV List"
{
    layout { modify(ColRec) { Visible = SetInRec; } }
    var SetInRec: Boolean;
    trigger OnAfterGetRecord() begin SetInRec := true; end;
}
page 69883 "ECV Card"
{
    PageType = Card; SourceTable = "ECV Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(IdCtl; Rec.Id) { } field(CtlRec; Rec.Name) { } } }
}
pageextension 69884 "ECV Card Ext" extends "ECV Card"
{
    layout { modify(CtlRec) { Visible = SetInRec; } }
    var SetInRec: Boolean;
    trigger OnAfterGetRecord() begin SetInRec := true; end;
}
page 69885 "ECV List Own"
{
    PageType = List; SourceTable = "ECV Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(ColRec; Rec.Name) { Visible = SetInRec; } } } }
    var SetInRec: Boolean;
    trigger OnAfterGetRecord() begin SetInRec := true; end;
}
codeunit 69886 C
{
    local procedure Seed()
    var R: Record "ECV Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
    end;

    procedure ExtList(): Text
    var P: TestPage "ECV List";
    begin
        Seed();
        P.OpenView();
        exit(Format(P.ColRec.Visible()));
    end;

    procedure ExtCard(): Text
    var P: TestPage "ECV Card";
    begin
        Seed();
        P.OpenView();
        exit(Format(P.CtlRec.Visible()));
    end;

    procedure OwnList(): Text
    var P: TestPage "ECV List Own";
    begin
        Seed();
        P.OpenView();
        exit(Format(P.ColRec.Visible()));
    end;
}
"#;

/// PageExtModifyVisibleVariable.al:534-547 (`ColRec=Yes` after OpenView).
#[test]
fn an_extension_column_counts_its_variable_after_the_open_read() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "ExtList")), "Yes");
}

/// Neighbors outside the pinned shape keep the open-time value, as on main: a card field, and a list's own column.
#[test]
fn a_card_field_and_a_list_own_column_keep_the_open_time_value() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "ExtCard")), "No");
    assert_eq!(text(al_testkit::run_in(SRC, "C", "OwnList")), "No");
}
