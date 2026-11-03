//! Page.Run and Page.RunModal with the fully qualified name of a page (BC 28 and later; corpus pages/lt-y74-fqn-run/
//! Y74FqnRunForms.al Run_Name_Only, Run_Name_Variants, Err_NotFullyQualified, Err_PageIdAsText). A name no page has is not pinned.

use al_test_cli::report::Outcome;

fn outcome(call: &str) -> Outcome {
    let (handler, check) = if call.starts_with("asserterror") {
        ("", "")
    } else {
        (
            "[HandlerFunctions('H')]",
            "if not Opened then Error('handler did not run');",
        )
    };
    let src = format!(
        r#"
namespace QN.Test;
page 62950 "QN Card" {{ PageType = Card; layout {{ area(Content) {{ }} }} }}
codeunit 62951 T
{{
    Subtype = Test;
    var Opened: Boolean;
    [Test] {handler} procedure Run()
    begin
        {call}
        {check}
    end;
    [PageHandler] procedure H(var P: TestPage "QN Card") begin Opened := true; end;
}}"#
    );
    al_testkit::outcomes_in(&src, "T")[0].1.clone()
}

#[test]
fn a_fully_qualified_name_opens_the_page_in_any_case_and_with_quotes() {
    assert_eq!(outcome("Page.Run('QN.Test.QN Card');"), Outcome::Pass);
    assert_eq!(outcome("Page.Run('qn.test.\"qn card\"');"), Outcome::Pass);
}

#[test]
fn a_name_without_its_namespace_or_a_page_id_raises() {
    for name in ["QN Card", "62950"] {
        let o = outcome(&format!(
            "asserterror Page.Run('{name}'); if GetLastErrorCode() <> 'InvalidOperation' then Error('%1', GetLastErrorCode());"
        ));
        assert_eq!(o, Outcome::Pass, "{name}");
    }
}

#[test]
fn a_name_no_page_has_is_refused() {
    let o = outcome("Page.Run('QN.Test.Missing');");
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("Page.Run with the name")),
        "{o:?}"
    );
}
