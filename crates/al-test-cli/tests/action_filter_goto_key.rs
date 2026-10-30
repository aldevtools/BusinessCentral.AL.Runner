//! al-runner issue 6381: a list without row triggers whose action changes Rec's filter, then a GoToKey to a row of the new
//! view that lies past the first block of rows. The rows the client read under the old filter must not stay in its window.
//! AL source -> compile -> run as a test codeunit.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62520 "AFG Row" { fields { field(1; Grp; Integer) { } field(2; No; Integer) { } } keys { key(PK; Grp, No) { Clustered = true; } } }
codeunit 62521 "AFG Seed" {
    procedure Seed() var R: Record "AFG Row"; G: Integer; I: Integer; begin R.DeleteAll(); for G := 1 to 2 do for I := 1 to 60 do begin R.Grp := G; R.No := I; R.Insert(); end; end; }
page 62522 "AFG List" { PageType = List; SourceTable = "AFG Row";
    layout { area(Content) { repeater(R) { field(Grp; Rec.Grp) { } field(No; Rec.No) { } } } }
    actions { area(Processing) {
        action(NextGroup) { trigger OnAction() begin Rec.SetRange(Grp, 2); if Rec.FindFirst() then; CurrPage.Update(false); end; }
        action(OnlyFilter) { trigger OnAction() begin Rec.SetRange(Grp, 2); end; }
        action(SameFilter) { trigger OnAction() begin Rec.SetRange(Grp, 1); end; }
        action(Descending) { trigger OnAction() begin Rec.Ascending(false); end; }
        action(FirstTen) { trigger OnAction() begin Rec.SetRange(No, 1, 10); end; } } }
    trigger OnOpenPage() begin Rec.SetRange(Grp, 1); end; }
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62530 T {{ Subtype = Test; {tests} }}");
    let o = al_testkit::outcomes_in(&src, "T");
    assert!(!o.is_empty());
    o.into_iter().map(|(n, r)| (n.to_string(), r)).collect()
}

fn passes(tests: &str) {
    let bad: Vec<_> = outcomes(tests)
        .into_iter()
        .filter(|(_, r)| *r != Outcome::Pass)
        .collect();
    assert!(bad.is_empty(), "{bad:?}");
}

// The reporter's repro (al-runner issue 6381), run in a modal handler as reported and on a TestPage opened directly. Values
// measured on real BC by corpus writer Y5 (branch al-runtime-rs-staging-y5, pages/lt-y5/Y5GoToKeyAfterActionFilter.al,
// codeunit 90985): after the action the row is 2/1; GoToKey(2,50) Yes; Next 2/51, Previous twice 2/49, GoToKey(2,10) Yes,
// First 2/1, Last 2/60; GoToKey(1,50) after the action No, the row stays 2/1; an action with only SetRange: row 2/1,
// GoToKey(2,50) Yes, GoToKey(1,40) No with the row staying 2/50.
#[test]
fn goto_key_after_an_action_changed_the_filter_finds_the_row_of_the_new_view() {
    passes(
        r#"[Test] [HandlerFunctions('ListHandler')] procedure Modal() var S: Codeunit "AFG Seed"; begin
            S.Seed(); Page.RunModal(Page::"AFG List"); end;
        [ModalPageHandler] procedure ListHandler(var L: TestPage "AFG List") begin
            L.GoToKey(1, 50); L.NextGroup.Invoke(); L.GoToKey(2, 50);
            if L.No.AsInteger() <> 50 then Error('expected row 50, got %1', L.No.Value);
            if L.Grp.AsInteger() <> 2 then Error('expected group 2, got %1', L.Grp.Value); end;
        [Test] procedure Direct() var S: Codeunit "AFG Seed"; L: TestPage "AFG List"; begin
            S.Seed(); L.OpenView(); L.GoToKey(1, 50); L.NextGroup.Invoke();
            if (L.Grp.AsInteger() <> 2) or (L.No.AsInteger() <> 1) then Error('after action %1/%2', L.Grp.Value, L.No.Value);
            if not L.GoToKey(2, 50) then Error('goto 2/50');
            L.Next(); if L.No.AsInteger() <> 51 then Error('next %1', L.No.Value);
            L.Previous(); L.Previous(); if L.No.AsInteger() <> 49 then Error('previous %1', L.No.Value);
            if not L.GoToKey(2, 10) then Error('goto 2/10');
            if L.No.AsInteger() <> 10 then Error('expected row 10, got %1', L.No.Value);
            L.First(); if (L.Grp.AsInteger() <> 2) or (L.No.AsInteger() <> 1) then Error('first %1/%2', L.Grp.Value, L.No.Value);
            L.Last(); if (L.Grp.AsInteger() <> 2) or (L.No.AsInteger() <> 60) then Error('last %1/%2', L.Grp.Value, L.No.Value); end;
        [Test] procedure WithoutFirstGoTo() var S: Codeunit "AFG Seed"; L: TestPage "AFG List"; begin
            S.Seed(); L.OpenView(); L.NextGroup.Invoke(); L.GoToKey(2, 50);
            if L.No.AsInteger() <> 50 then Error('expected row 50, got %1', L.No.Value); end;
        [Test] procedure OnlySetRange() var S: Codeunit "AFG Seed"; L: TestPage "AFG List"; begin
            S.Seed(); L.OpenView(); L.GoToKey(1, 50); L.OnlyFilter.Invoke();
            if (L.Grp.AsInteger() <> 2) or (L.No.AsInteger() <> 1) then Error('after action %1/%2', L.Grp.Value, L.No.Value);
            if not L.GoToKey(2, 50) then Error('goto 2/50');
            if L.No.AsInteger() <> 50 then Error('expected row 50, got %1', L.No.Value);
            if L.GoToKey(1, 40) then Error('row 1/40 is outside the new view');
            if (L.Grp.AsInteger() <> 2) or (L.No.AsInteger() <> 50) then Error('after miss %1/%2', L.Grp.Value, L.No.Value); end;
        [Test] procedure OldRowLeftTheView() var S: Codeunit "AFG Seed"; L: TestPage "AFG List"; begin
            S.Seed(); L.OpenView(); L.GoToKey(1, 50); L.NextGroup.Invoke();
            if L.GoToKey(1, 50) then Error('row 1/50 is outside the new view');
            if (L.Grp.AsInteger() <> 2) or (L.No.AsInteger() <> 1) then Error('after miss %1/%2', L.Grp.Value, L.No.Value); end;"#,
    );
}

// A sort order change drops the rows read too: GoToKey walks the new order (not measured; the row stays in the view).
#[test]
fn goto_key_after_an_action_changed_the_sort_order_finds_the_row() {
    passes(
        r#"[Test] procedure Desc() var S: Codeunit "AFG Seed"; L: TestPage "AFG List"; begin
            S.Seed(); L.OpenView(); L.GoToKey(1, 50); L.Descending.Invoke();
            if L.No.AsInteger() <> 50 then Error('after action %1', L.No.Value);
            if not L.GoToKey(1, 5) then Error('goto 1/5');
            if L.No.AsInteger() <> 5 then Error('expected row 5, got %1', L.No.Value); end;"#,
    );
}

// A filter that leaves no row of the view after the one the page stood on: the row before it is not measured without row
// triggers, so it is refused.
#[test]
fn a_filter_change_with_no_row_after_the_current_one_is_refused() {
    let o = outcomes(
        r#"[Test] procedure NoRowAfter() var S: Codeunit "AFG Seed"; L: TestPage "AFG List"; begin
            S.Seed(); L.OpenView(); L.GoToKey(1, 50); L.FirstTen.Invoke(); end;"#,
    );
    assert!(o.iter().all(|(_, r)| matches!(r, Outcome::NotRun(_))), "{o:?}");
}

// The same filter set again keeps the view: GoToKey back and forth in it passes as before (variants in the issue).
#[test]
fn goto_key_after_an_action_that_kept_the_filter_still_passes() {
    passes(
        r#"[Test] procedure Same() var S: Codeunit "AFG Seed"; L: TestPage "AFG List"; begin
            S.Seed(); L.OpenView(); L.GoToKey(1, 50); L.SameFilter.Invoke(); L.GoToKey(1, 40);
            if L.No.AsInteger() <> 40 then Error('expected row 40, got %1', L.No.Value); end;"#,
    );
}
