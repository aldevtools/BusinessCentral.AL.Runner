//! A page extension that moves actions (several at once, into a promoted group, and a promoted group after another) leaves
//! a page whose actions a test page still invokes by name (al-runner issue 6483, the reporter's app).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 50493 "PMA T"
{
    fields { field(1; K; Code[10]) { } }
    keys { key(PK; K) { Clustered = true; } }
}
page 50494 "PMA P"
{
    PageType = Card;
    SourceTable = "PMA T";
    layout { area(Content) { field(K; Rec.K) { } } }
    actions
    {
        area(Processing)
        {
            action(First) { trigger OnAction() begin Error('First ran'); end; }
            action(Second) { trigger OnAction() begin Error('Second ran'); end; }
            action(Third) { trigger OnAction() begin Error('Third ran'); end; }
        }
        area(Promoted)
        {
            group(Category_Process)
            {
                actionref(First_Promoted; First) { }
                actionref(Second_Promoted; Second) { }
                actionref(Third_Promoted; Third) { }
            }
            group(Category_Category4) { }
        }
    }
}
pageextension 50495 "PMA PE" extends "PMA P"
{
    actions
    {
        moveafter(Third; First)
        movefirst(Category_Category4; Second_Promoted, Third_Promoted)
        moveafter(Category_Process; Category_Category4)
    }
}
codeunit 50496 "T"
{
    Subtype = Test;
    [Test]
    procedure MovedActionsStillRun()
    var TP: TestPage "PMA P";
    begin
        TP.OpenEdit();
        asserterror TP.First.Invoke();
        if GetLastErrorText() <> 'First ran' then Error('first: %1', GetLastErrorText());
        asserterror TP.Second.Invoke();
        if GetLastErrorText() <> 'Second ran' then Error('second: %1', GetLastErrorText());
        asserterror TP.Third.Invoke();
        if GetLastErrorText() <> 'Third ran' then Error('third: %1', GetLastErrorText());
    end;
}
"#;

#[test]
fn moved_actions_still_run() {
    let out = outcomes_in(SRC, "T");
    assert_eq!(out, vec![("MovedActionsStillRun".to_string(), Outcome::Pass)]);
}
