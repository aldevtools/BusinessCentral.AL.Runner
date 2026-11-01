//! A parameterless TestPage method written without parentheses (`P.OK.Invoke()`, `P.Cancel.Invoke()`) is the same call as
//! with them, as for any parameterless method: the corpus reads a TestField's Caption without
//! parentheses as the method (pages/lt-ca-odd-667/CaTestPageTests.al:261-282 Ca07_CaptionWithoutParentheses) and the
//! Assert fixture calls RecRef.Count without them (_fixtures/Assert.al:167). A control of the page with that name still
//! wins (handlers/TestPageKeywordActionName_Tests.al:64 invokes an action named Delete).

use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 61440 "TAP Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
page 61441 "TAP Dialog" { PageType = StandardDialog; SourceTable = "TAP Row"; layout { area(Content) { field(IdCtl; Rec.Id) { } } } }
page 61442 "TAP Named" { PageType = Card; SourceTable = "TAP Row"; layout { area(Content) { field(OK; Rec.Id) { } } } }
codeunit 61450 T { Subtype = Test;
[ModalPageHandler] procedure OkBare(var P: TestPage "TAP Dialog") begin P.OK.Invoke(); end;
[ModalPageHandler] procedure OkCall(var P: TestPage "TAP Dialog") begin P.OK().Invoke(); end;
[ModalPageHandler] procedure CancelBare(var P: TestPage "TAP Dialog") begin P.Cancel.Invoke(); end;
[ModalPageHandler] procedure CancelCall(var P: TestPage "TAP Dialog") begin P.Cancel().Invoke(); end;
local procedure Run(): Action begin exit(Page.RunModal(Page::"TAP Dialog")); end;
[Test] [HandlerFunctions('OkBare')] procedure OkWithoutParentheses() begin Error('bare %1', Run()); end;
[Test] [HandlerFunctions('OkCall')] procedure OkWithParentheses() begin Error('call %1', Run()); end;
[Test] [HandlerFunctions('CancelBare')] procedure CancelWithoutParentheses() begin Error('bare %1', Run()); end;
[Test] [HandlerFunctions('CancelCall')] procedure CancelWithParentheses() begin Error('call %1', Run()); end;
[Test] procedure ControlNamedOkWins() var P: TestPage "TAP Named"; R: Record "TAP Row"; begin R.Id := 7; R.Insert(); P.OpenView(); P.GoToRecord(R); if P.OK.Value <> '7' then Error('value <%1>', P.OK.Value); P.Close(); end;
}
"#;

// Each test fails with the action RunModal returned; without parentheses it must be the same action as with them.
#[test]
fn a_test_page_method_without_parentheses_is_the_method() {
    let o = al_testkit::outcomes_in(SRC, "T");
    assert_eq!(o.len(), 5, "{o:?}");
    let text = |n: &str| match o.iter().find(|(t, _)| t == n).map(|(_, r)| r) {
        Some(Outcome::Fail(m)) => m.split_once(' ').map(|(_, a)| a.to_string()).unwrap_or_default(),
        other => panic!("{n}: {other:?}"),
    };
    assert_eq!(text("OkWithoutParentheses"), text("OkWithParentheses"));
    assert_eq!(text("CancelWithoutParentheses"), text("CancelWithParentheses"));
    assert_ne!(text("OkWithParentheses"), text("CancelWithParentheses"));
    assert!(
        matches!(
            o.iter().find(|(t, _)| t == "ControlNamedOkWins"),
            Some((_, Outcome::Pass))
        ),
        "{o:?}"
    );
}
