//! A page procedure that calls the bare `RunModal()` runs the page instance it belongs to modally, after the bare `Caption`
//! and `LookupMode(true)`; the ModalPageHandler picks the first row and OK answers `Action::LookupOK` (corpus
//! pages/lt-wb-6283/WbBareRunModal.al:35-43 and :49-71, PageProcedureRunsItselfModal, corpus branch al-runtime-rs-staging-wb
//! at a07787b2, not yet on master; al-runner issue 6283).

use al_testkit::outcomes_in;

const SRC: &str = r#"
table 50100 "PS Template" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { Clustered = true; } } }
page 50100 "PS Templates"
{
    PageType = List; SourceTable = "PS Template"; Editable = false; UsageCategory = None;
    layout { area(Content) { repeater(Lines) { field("Code"; Rec."Code") { } } } }
    procedure Lookup(var SelectedCode: Code[20]): Boolean
    begin
        Caption('Select a template');
        LookupMode(true);
        if RunModal() = Action::LookupOK then begin
            SelectedCode := Rec."Code";
            exit(true);
        end;
    end;
}
codeunit 50101 "PS Tests"
{
    Subtype = Test;
    [Test] [HandlerFunctions('PickFirstHandler')]
    procedure PageProcedureRunsItselfModal()
    var Template: Record "PS Template"; Templates: Page "PS Templates"; SelectedCode: Code[20];
    begin
        Template.DeleteAll();
        Template."Code" := 'A';
        Template.Insert();
        if not Templates.Lookup(SelectedCode) then
            Error('lookup cancelled');
        if SelectedCode <> 'A' then
            Error('got %1', SelectedCode);
    end;
    [ModalPageHandler]
    procedure PickFirstHandler(var Templates: TestPage "PS Templates")
    begin
        Templates.First();
        Templates.OK().Invoke();
    end;
}
"#;

#[test]
fn a_bare_run_modal_in_a_page_procedure_runs_that_page_instance() {
    let o = outcomes_in(SRC, "PS Tests");
    assert_eq!(o.len(), 1);
    for (name, outcome) in &o {
        assert_eq!(format!("{outcome:?}"), "Pass", "{name}");
    }
}
