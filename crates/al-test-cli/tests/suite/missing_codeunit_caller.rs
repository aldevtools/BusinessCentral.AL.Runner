//! The caller a missing-codeunit error names. A `Codeunit.Run` the test method issues itself names the test runner
//! (corpus errors/lt-az-error-texts/AzErrorTexts.al:110-134 CodeunitRun_MissingId_RawText); one a helper codeunit issues
//! names the test codeunit (misc/W5MissingObject.al:19-26 MissingCodeunit_FromHelperCodeunit).

use al_testrun::Outcome;

const SRC: &str = r#"
codeunit 69995 "MC Helper"
{
    procedure RunMissing(Id: Integer) begin Codeunit.Run(Id); end;
}
codeunit 69996 "MC Tests"
{
    Subtype = Test;
    local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;
    [Test] procedure Direct()
    var Ok: Boolean;
    begin
        asserterror Ok := Codeunit.Run(80748);
        Check('You tried to invoke the CodeUnit object with the ID 80748 from the object Test Runner - Mgt. An object with that ID does not exist in the current application compiled with emit version 29000.', GetLastErrorText());
        Check('AL:MetadataNotFound', GetLastErrorCode());
    end;
    [Test] procedure ViaHelper()
    var H: Codeunit "MC Helper";
    begin
        asserterror H.RunMissing(80748);
        Check('You tried to invoke the CodeUnit object with the ID 80748 from the object MC Tests. An object with that ID does not exist in the current application compiled with emit version 29000.', GetLastErrorText());
    end;
}
"#;

#[test]
fn a_missing_codeunit_names_the_test_runner_or_the_test_codeunit() {
    for (n, o) in al_testkit::outcomes_in(SRC, "MC Tests") {
        assert_eq!(o, Outcome::Pass, "{n}");
    }
}
