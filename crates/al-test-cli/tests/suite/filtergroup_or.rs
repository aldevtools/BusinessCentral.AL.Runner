//! `FilterGroup(-1)` is the "OR group" in BC: filters set in it on different fields are alternatives
//! (pinned by corpus record/PWFilterGroupOr.al; issue 17: Base Application's
//! `FilterBlankSalesDiscountAccounts`). Other negative groups are refused, not guessed.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62600 "FG Row"
{
    fields { field(1; "No."; Code[20]) { } field(2; A; Code[20]) { } field(3; B; Code[20]) { } }
    keys { key(PK; "No.") { } }
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62601 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

#[test]
fn the_or_group_selects_alternatives_and_other_negative_groups_are_refused() {
    let o = outcomes(
        "[Test] procedure OrGroup() var R: Record \"FG Row\"; begin
             R.DeleteAll();
             R.\"No.\" := '1'; R.A := ''; R.B := 'x'; R.Insert();
             R.\"No.\" := '2'; R.A := 'x'; R.B := ''; R.Insert();
             R.\"No.\" := '3'; R.A := 'x'; R.B := 'x'; R.Insert();
             Clear(R);
             R.FilterGroup(-1); R.SetRange(A, ''); R.SetRange(B, '');
             if R.Count() <> 2 then Error('or group counts %1', R.Count());
             R.FilterGroup(0);
             if R.HasFilter() then Error('group 0 has no filter');
             if R.Count() <> 2 then Error('group 0 still applies it, %1', R.Count());
         end;
         [Test] procedure OtherNegative() var R: Record \"FG Row\"; begin R.FilterGroup(-2); end;
         [Test] procedure OtherGroups() var R: Record \"FG Row\"; begin
             R.FilterGroup(2); R.SetRange(A, 'x'); R.FilterGroup(0);
             if R.FilterGroup() <> 0 then Error('group'); end;",
    );
    // corpus record/PWFilterGroupOr.al pins the OR group in full.
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0]);
    assert!(
        matches!(&o[1].1, Outcome::NotRun(why) if why.contains("FilterGroup(-2)") && why.contains("only -1")),
        "{:?}",
        o[1]
    );
    assert_eq!(o[2].1, Outcome::Pass, "{:?}", o[2]);
}
