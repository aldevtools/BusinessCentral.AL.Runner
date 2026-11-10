//! An OnQueryClosePage that reaches its end without giving a value lets the page close (Item Tracking Lines' ends with its
//! named return value untouched when the availability check finds nothing; assumption query-close-page-falls-through-as-true;
//! Tests-SCM TestInventoryReceiptPreviewWithSeriallyTrackedItem).

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
page 50400 "QCF Named" { PageType = Card; layout { area(Content) { field(F; Flag) { } } }
    trigger OnQueryClosePage(CloseAction: Action) Result: Boolean
    begin
        if Flag then begin
            Result := true;
            if Flag then exit(Confirm('go?', true));
        end;
    end;
    var Flag: Boolean; }
page 50403 "QCF Reads" { PageType = Card; layout { area(Content) { field(F; Flag) { } } }
    trigger OnQueryClosePage(CloseAction: Action) Result: Boolean
    begin
        if Result then exit(true);
    end;
    var Flag: Boolean; }
page 50404 "QCF False" { PageType = Card; layout { area(Content) { field(F; Flag) { } } }
    trigger OnQueryClosePage(CloseAction: Action) Result: Boolean
    begin
        Result := false;
    end;
    var Flag: Boolean; }
page 50401 "QCF Plain" { PageType = Card; layout { area(Content) { field(F; Flag) { } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        if Flag then exit(false);
    end;
    var Flag: Boolean; }
codeunit 50402 "T"
{
    Subtype = Test;
    [Test]
    [HandlerFunctions('OkNamed')]
    procedure NamedFallThroughCloses()
    begin
        if Page.RunModal(Page::"QCF Named") <> Action::LookupOK then Error('did not close');
    end;
    [Test]
    [HandlerFunctions('OkPlain')]
    procedure PlainFallThroughCloses()
    begin
        if Page.RunModal(Page::"QCF Plain") <> Action::LookupOK then Error('did not close');
    end;
    [Test]
    [HandlerFunctions('OkReads')]
    procedure ReadBeforeAssignReadsYes()
    begin
        if Page.RunModal(Page::"QCF Reads") <> Action::LookupOK then Error('did not close');
    end;
    [Test]
    [HandlerFunctions('OkFalse')]
    procedure AssignedFalseKeepsThePageOpen()
    begin
        if Page.RunModal(Page::"QCF False") <> Action::None then Error('closed');
    end;
    [ModalPageHandler]
    procedure OkReads(var P: TestPage "QCF Reads") begin P.OK().Invoke(); end;
    [ModalPageHandler]
    procedure OkFalse(var P: TestPage "QCF False") begin P.OK().Invoke(); end;
    [ModalPageHandler]
    procedure OkNamed(var P: TestPage "QCF Named") begin P.OK().Invoke(); end;
    [ModalPageHandler]
    procedure OkPlain(var P: TestPage "QCF Plain") begin P.OK().Invoke(); end;
}
"#;

#[test]
fn a_query_close_page_that_falls_through_lets_the_page_close() {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    // A read of the named return value before it has one gives Yes, and an assigned false at the end of the trigger refuses
    // the close: RunModal answers None (corpus pages/triggers/QcrNamedResult.al:248-286, :288-331).
    for name in [
        "NamedFallThroughCloses",
        "PlainFallThroughCloses",
        "ReadBeforeAssignReadsYes",
        "AssignedFalseKeepsThePageOpen",
    ] {
        let o = rt.first_outcome(&CaseId {
            codeunit: 50402,
            name: name.into(),
        });
        assert_eq!(o, Outcome::Pass, "{name}");
    }
}
