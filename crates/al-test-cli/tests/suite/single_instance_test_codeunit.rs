//! A test codeunit with `SingleInstance = true` runs its test methods on the session's single instance: a variable of its
//! own type sees the globals the test method writes (codeunit/QiSingleInstanceTestCodeunit.al:57
//! QI_SI_1_OwnTypeVariableAndManualSubscriber `other-get` m1, `own-after-other-set` m2).

use al_testrun::Outcome;

#[test]
fn a_single_instance_test_codeunit_runs_on_the_single_instance() {
    let src = r#"
codeunit 62600 T
{
    Subtype = Test;
    SingleInstance = true;
    var Seen: Boolean;
    [Test]
    procedure ReadsItsOwnGlobal()
    var Me: Codeunit T;
    begin
        Me.SetSeen();
        if not Seen then Error('a second instance');
    end;
    procedure SetSeen() begin Seen := true; end;
}
"#;
    let o = al_testkit::outcomes_in(src, "T");
    assert!(matches!(&o[0].1, Outcome::Pass), "{o:?}");
}
