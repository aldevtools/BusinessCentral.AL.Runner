//! A plain `TestPage.Close()` in a ModalPageHandler (no OnQueryClosePage veto): RunModal raises. Pinned only for the static
//! RunModal of a StandardDialog (misc/lt-cb-odd-leads/CbHandlerTests.al:181 ModalHandler_CallsClose_RunModalOutcome) and a Page
//! variable's RunModal of a Card (pages/lt-bl-unopened-currpage/UnopenedCurrPage.al:273 Update_Then_RunModal_Card_HandlerCloses).
//! Every other page type and run kind is refused.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 61420 "MHC Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }
page 61421 "MHC Card" { PageType = Card; SourceTable = "MHC Row"; layout { area(Content) { field(IdCtl; Rec.Id) { } } } }
page 61422 "MHC Dialog" { PageType = StandardDialog; SourceTable = "MHC Row"; layout { area(Content) { field(IdCtl; Rec.Id) { } } } }
page 61423 "MHC List" { PageType = List; SourceTable = "MHC Row"; layout { area(Content) { repeater(R) { field(IdCtl; Rec.Id) { } } } } }
"#;

const HELPERS: &str = "
[TryFunction] local procedure VarCard() var P: Page \"MHC Card\"; begin P.RunModal(); end;
[TryFunction] local procedure VarDialog() var P: Page \"MHC Dialog\"; begin P.RunModal(); end;
[TryFunction] local procedure VarList() var P: Page \"MHC List\"; begin P.RunModal(); end;
[TryFunction] local procedure StaticCard() begin Page.RunModal(Page::\"MHC Card\"); end;
[TryFunction] local procedure StaticDialog() begin Page.RunModal(Page::\"MHC Dialog\"); end;
local procedure Check(Ok: Boolean; Id: Integer) begin
    if Ok then Error('no error');
    if GetLastErrorText() <> StrSubstNo('The RunModal procedure could not close the page %1 as it has already been closed.', Id) then
        Error('text <%1>', GetLastErrorText());
    if GetLastErrorCode() <> 'TestCodeUnit' then Error('code <%1>', GetLastErrorCode()); end;
[ModalPageHandler] procedure CardClose(var P: TestPage \"MHC Card\") begin P.Close(); end;
[ModalPageHandler] procedure DialogClose(var P: TestPage \"MHC Dialog\") begin P.Close(); end;
[ModalPageHandler] procedure ListClose(var P: TestPage \"MHC List\") begin P.Close(); end;";

const TESTS: &str = "
[Test] [HandlerFunctions('CardClose')] procedure VarCardRaises() begin Check(VarCard(), 61421); end;
[Test] [HandlerFunctions('DialogClose')] procedure StaticDialogRaises() begin Check(StaticDialog(), 61422); end;
[Test] [HandlerFunctions('DialogClose')] procedure VarDialogRefused() begin Check(VarDialog(), 61422); end;
[Test] [HandlerFunctions('ListClose')] procedure VarListRefused() begin Check(VarList(), 61423); end;
[Test] [HandlerFunctions('CardClose')] procedure StaticCardRefused() begin Check(StaticCard(), 61421); end;";

#[test]
fn close_in_a_modal_handler_raises_only_for_the_pinned_page_types() {
    let src = format!("{OBJECTS}\ncodeunit 61430 T {{ Subtype = Test; {HELPERS} {TESTS} }}");
    let o = al_testkit::outcomes_in(&src, "T");
    assert_eq!(o.len(), 5);
    for (n, r) in &o {
        if n.ends_with("Raises") {
            assert_eq!(r, &Outcome::Pass, "{n}");
        } else {
            assert!(
                matches!(r, Outcome::NotRun(why) if why.contains("TestPage.Close() in a ModalPageHandler")),
                "{n}: {r:?}"
            );
        }
    }
}
