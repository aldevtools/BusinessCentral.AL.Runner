//! A new record of a card whose key an action or a SetValue sets (pages/actions.rs, pages/draft.rs). Each test names the corpus
//! test of pages/table-globals-page/TableGlobalsPageWithoutValidation.al that pins what BC does.

const SRC: &str = r#"
table 69980 "DK Row"
{
    fields { field(1; "No."; Code[10]) { } field(2; Payload; Integer) { } field(3; Seen; Text[50]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
    var Counter: Integer;
    trigger OnInsert() begin Seen := 'I' + Format(Counter); end;
    procedure Bump(): Integer begin Counter += 1; exit(Counter); end;
    procedure GetCounter(): Integer begin exit(Counter); end;
}
page 69981 "DK Card"
{
    PageType = Card; SourceTable = "DK Row"; UsageCategory = None;
    layout { area(Content) { group(G) { field("No."; Rec."No.") { } field(Seen; Rec.Seen) { } field(LastCount; LastCount) { } } } }
    actions
    {
        area(Processing)
        {
            action(SetKeyAndBump) { trigger OnAction() begin Rec."No." := 'N1'; LastCount := Rec.Bump(); Rec.Bump(); end; }
            action(BumpOnly) { trigger OnAction() begin LastCount := Rec.Bump(); Rec.Bump(); end; }
            action(ReadCount) { trigger OnAction() begin LastCount := Rec.GetCounter(); end; }
        }
    }
    var LastCount: Integer;
}
codeunit 69982 C
{
    procedure KeyByAction(): Text
    var Card: TestPage "DK Card"; Row: Record "DK Row"; R: Text;
    begin
        Card.OpenNew();
        Card.SetKeyAndBump.Invoke();
        R := Format(Row.Get('N1')) + ':' + Card.LastCount.Value() + ':' + Card.Seen.Value();
        Card.ReadCount.Invoke();
        R += '|' + Card.LastCount.Value();
        Card.BumpOnly.Invoke();
        R += '|' + Card.LastCount.Value();
        Card.ReadCount.Invoke();
        R += '|' + Card.LastCount.Value();
        Card.Close();
        Row.Get('N1');
        exit(R + '|' + Row.Seen + ':' + Format(Row.Count()));
    end;

    procedure KeyTyped(): Text
    var Card: TestPage "DK Card"; Row: Record "DK Row"; R: Text;
    begin
        Card.OpenNew();
        Card."No.".SetValue('N3');
        R := Format(Row.Get('N3'));
        Card.BumpOnly.Invoke();
        R += '|' + Format(Row.Get('N3')) + ':' + Row.Seen;
        Card.ReadCount.Invoke();
        R += '|' + Card.LastCount.Value();
        Card.Close();
        exit(R + '|' + Format(Row.Count()));
    end;
}
"#;

/// TableGlobalsPageWithoutValidation.al:235 QG_NewDraft_KeySetByAction_WhenStored (stored right after the action, Seen `I0`, the
/// next action sees fresh globals `0`) and :293 QG_NewDraft_KeySetByAction_ThenBumpAction (`1`, then `2`).
#[test]
fn a_key_an_action_sets_stores_the_new_row_right_after_it_on_fresh_globals() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "KeyByAction")),
        "Yes:1:I0|0|1|2|I0:1"
    );
}

/// TableGlobalsPageWithoutValidation.al:318 QG_NewDraft_KeyTyped_ThenBumpAction: not stored at the SetValue, stored before the next
/// action's trigger (`I0`, then ReadCount `2`).
#[test]
fn a_key_typed_with_set_value_is_stored_before_the_next_action() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "KeyTyped")),
        "No|Yes:I0|2|1"
    );
}
