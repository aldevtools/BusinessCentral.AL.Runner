//! Microsoft-bucket refusals, third round (branch ms-top-refusals-3): `RecordRef.IsDirty`.

use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!(
        "table 63200 \"M3 Row\" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[20]) {{ }} field(3; Qty; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
         codeunit 63201 T {{ Subtype = Test; {tests} }}"
    );
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    run_codeunit(&engine, &l, oi)
}

// The sequence of corpus misc/lt-open-overloads-collections-isolated/P107RecRefNames.al:101-135 (P107_IsDirty) and
// recordref/lt-recordref-fieldref-members/P92DirtyRef.al:21-50 (IsDirty_FieldChanges): the expected answers are theirs.
#[test]
fn record_ref_is_dirty_follows_the_pinned_sequence() {
    let o = outcomes(
        "[Test] procedure Seq()
         var T: Record \"M3 Row\"; RR: RecordRef; S: Text;
         begin
           T.Id := 1; T.Name := 'Alpha'; T.Qty := 1; T.Insert();
           RR.Open(63200);
           S += Format(RR.IsDirty());
           RR.FindFirst();
           S += Format(RR.IsDirty());
           RR.Field(3).Value := 1;
           S += Format(RR.IsDirty());
           RR.Field(3).Value := 2;
           S += Format(RR.IsDirty());
           RR.Modify();
           S += Format(RR.IsDirty());
           RR.Field(2).Value := 'Changed';
           S += Format(RR.IsDirty());
           RR.Find('=');
           S += Format(RR.IsDirty());
           RR.Init();
           S += Format(RR.IsDirty());
           if S <> 'NoNoNoYesNoYesNoYes' then Error(S);
         end;",
    );
    assert!(matches!(o[0].1, Outcome::Pass), "{:?}", o[0].1);
}
