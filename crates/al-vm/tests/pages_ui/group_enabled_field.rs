//! A field with `Enabled = true` inside a group with `Enabled = false` is disabled on a TestPage of a card
//! (pages/lt-ap-testpage-642/ApTestPageTests.al:1082 Ap16_GroupEnabledFalse_FieldEnabledTrue). Another page type is the
//! registered assumption `group-enabled-disables-field`.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};

const SRC: &str = r#"
table 50711 "Load Row" { fields { field(1; Code; Code[20]) { } field(2; Description; Text[100]) { } field(3; Notes; Text[250]) { } } keys { key(PK; Code) { } } }
page 50704 "Group Enabled Card"
{
    PageType = Card; SourceTable = "Load Row";
    layout
    {
        area(Content)
        {
            group(Locked)
            {
                Enabled = false;
                field(Description; Rec.Description) { ApplicationArea = All; Enabled = true; }
            }
            group(Open)
            {
                field(Notes; Rec.Notes) { ApplicationArea = All; }
            }
        }
    }
}
codeunit 50768 C
{
    procedure F(): Text
    var Row: Record "Load Row"; Card: TestPage "Group Enabled Card";
    begin
        Row.Code := 'G1'; Row.Insert();
        Card.OpenEdit(); Card.GoToKey('G1');
        exit(Format(Card.Description.Enabled()) + ',' + Format(Card.Notes.Enabled()));
    end;
}
"#;

#[test]
fn a_field_in_a_disabled_group_is_disabled() {
    clear_hits();
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "F")), "No,Yes");
    assert!(!take_hits().contains(&Assumption::GroupEnabledDisablesField));
}

#[test]
fn a_field_in_a_disabled_group_of_a_document_is_the_assumption() {
    let src = SRC.replace("PageType = Card;", "PageType = Document;");
    clear_hits();
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "F")), "No,Yes");
    assert!(take_hits().contains(&Assumption::GroupEnabledDisablesField));
}

#[test]
fn a_card_field_without_enabled_in_a_disabled_group_is_pinned() {
    // pages/lt-ap-testpage-642/ApTestPageTests.al:895 Ap16_GroupEnabledFalse_Card: no assumption on this path.
    let src = SRC.replace("ApplicationArea = All; Enabled = true;", "ApplicationArea = All;");
    clear_hits();
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "C", "F")), "No,Yes");
    assert!(!take_hits().contains(&Assumption::GroupEnabledDisablesField));
}
