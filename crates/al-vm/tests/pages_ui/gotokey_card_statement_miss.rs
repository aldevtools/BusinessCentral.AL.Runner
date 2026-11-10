//! GoToKey and GoToRecord called as a statement for a key no row has. On a list page the row error is pinned
//! (pages/run-modal/Y7412PageRunRecordFilters.al:337 Y7412_Run0_StatementForm, pages/list-key-navigation/
//! Y7409GoToRecordSkippedRow.al:435 Y7409_StatementForm_HiddenAndShownRow). On a Card or a Document page only the form
//! that uses the result is pinned (pages/W1Navigation.al:284 GoToKey_Card_WhichRowsAreRead `goto_99_miss` `No ... id=4`):
//! the statement form is refused, whatever makes the key a miss.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 60 "RW Order" { fields { field(1; "No."; Code[10]) { } field(2; Name; Text[30]) { } } keys { key(PK; "No.") { Clustered = true; } } }
page 61 "RW Order Card" { PageType = Card; SourceTable = "RW Order"; layout { area(Content) { field("No."; Rec."No.") { } field(Name; Rec.Name) { } } } }
page 62 "RW Order List" { PageType = List; SourceTable = "RW Order"; layout { area(Content) { repeater(R) { field("No."; Rec."No.") { } } } } }
page 63 "RW Order Doc" { PageType = Document; SourceTable = "RW Order"; layout { area(Content) { group(G) { field("No."; Rec."No.") { } } } } }
codeunit 64 C
{
    local procedure Seed()
    var
        O: Record "RW Order";
    begin
        O.DeleteAll();
        O."No." := 'O1'; O.Insert();
        O."No." := 'O2'; O.Insert();
    end;

    procedure CardGoToKey(): Text
    var P: TestPage "RW Order Card";
    begin
        Seed(); P.OpenView();
        P.GoToKey('ZZ');
        exit(P."No.".Value());
    end;

    procedure CardGoToKeyLowercase(): Text
    var P: TestPage "RW Order Card";
    begin
        Seed(); P.OpenView();
        P.GoToKey('o2');
        exit(P."No.".Value());
    end;

    procedure CardGoToKeyWrongType(): Text
    var P: TestPage "RW Order Card";
    begin
        Seed(); P.OpenView();
        P.GoToKey(1);
        exit(P."No.".Value());
    end;

    procedure DocGoToKey(): Text
    var P: TestPage "RW Order Doc";
    begin
        Seed(); P.OpenView();
        P.GoToKey('ZZ');
        exit(P."No.".Value());
    end;

    procedure CardGoToRecord(): Text
    var P: TestPage "RW Order Card"; O: Record "RW Order";
    begin
        Seed(); P.OpenView();
        O."No." := 'ZZ';
        P.GoToRecord(O);
        exit(P."No.".Value());
    end;

    procedure ListGoToKey(): Text
    var P: TestPage "RW Order List";
    begin
        Seed(); P.OpenView();
        P.GoToKey('ZZ');
        exit(P."No.".Value());
    end;

    procedure CardGoToKeyResultUsed(): Text
    var P: TestPage "RW Order Card"; Found: Boolean;
    begin
        Seed(); P.OpenView();
        Found := P.GoToKey('ZZ');
        exit(Format(Found) + ' ' + P."No.".Value());
    end;
}
"#;

fn run(routine: &str) -> Result<Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

fn refused(routine: &str) {
    match run(routine) {
        Err(Signal::Unsupported(why)) => assert!(
            why.contains("as a statement") && why.contains("neither a list nor a part") && why.contains("not pinned"),
            "{routine}: {why}"
        ),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_statement_miss_on_a_card_or_a_document_page_is_refused() {
    refused("CardGoToKey");
    refused("DocGoToKey");
    refused("CardGoToRecord");
}

#[test]
fn a_statement_miss_by_a_lowercase_or_mistyped_key_on_a_card_is_refused() {
    refused("CardGoToKeyLowercase");
    refused("CardGoToKeyWrongType");
}

// Y7412PageRunRecordFilters.al:337, Y7409GoToRecordSkippedRow.al:435.
#[test]
fn a_statement_miss_on_a_list_raises_the_row_error() {
    assert_eq!(
        al_testkit::error_message(run("ListGoToKey")),
        "The row does not exist on the TestPage."
    );
}

// W1Navigation.al:284 GoToKey_Card_WhichRowsAreRead `goto_99_miss`.
#[test]
fn a_miss_on_a_card_whose_result_is_used_answers_no_and_stays() {
    assert_eq!(al_testkit::text(run("CardGoToKeyResultUsed")), "No O1");
}
