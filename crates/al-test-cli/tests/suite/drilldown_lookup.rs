//! `DrillDown`, `Lookup` and `AssistEdit` on TestPage controls: the control's own `DrillDown`, `DrillDownPageId`,
//! `LookupPageId` and `TableRelation` properties. AL source -> compile -> run as a test codeunit. Each test names the
//! corpus test it mirrors.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 64500 "DL Grp"
{
    LookupPageId = "DL Grp List";
    fields { field(1; Code; Code[10]) { } field(2; Descr; Text[30]) { } }
    keys { key(PK; Code) { } }
}
table 64501 "DL Other"
{
    fields { field(1; Code; Code[10]) { } field(2; Descr; Text[30]) { } }
    keys { key(PK; Code) { } }
}
table 64502 "DL Item"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Grp; Code[10]) { }
        field(3; Other; Text[30]) { }
        field(4; Qty; Integer) { }
        field(5; Rel; Code[10]) { TableRelation = "DL Grp".Code; }
        field(6; Txt; Text[30]) { }
    }
    keys { key(PK; "No.") { } }
}
codeunit 64503 "DL Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(E: Text) begin L += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := L; L := ''; exit(R); end;
}
page 64510 "DL Grp List"
{
    PageType = List; SourceTable = "DL Grp"; Editable = false;
    layout { area(Content) { repeater(R) { field(CodeCtl; Rec.Code) { } field(DescrCtl; Rec.Descr) { } } } }
}
page 64511 "DL Other List"
{
    PageType = List; SourceTable = "DL Other"; Editable = false;
    layout { area(Content) { repeater(R) { field(CodeCtl; Rec.Code) { } field(DescrCtl; Rec.Descr) { } } } }
}
page 64512 "DL Card"
{
    PageType = Card; SourceTable = "DL Item";
    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { }
            field(DrillOn; Rec.Txt) { DrillDown = true; trigger OnDrillDown() begin Error('drilled on'); end; }
            field(DrillOff; Rec.Txt) { DrillDown = false; trigger OnDrillDown() begin Error('drilled off'); end; }
            field(Plain; Rec.Txt) { }
            field(DrillPage; Rec.Qty) { DrillDownPageId = "DL Grp List"; }
            field(CtlRel; Rec.Grp) { TableRelation = "DL Grp".Code; }
            field(CtlRelPage; Rec.Other) { LookupPageId = "DL Other List"; TableRelation = "DL Other".Descr; }
            field(CtlRelOtherPage; Rec.Grp) { LookupPageId = "DL Other List"; TableRelation = "DL Grp".Code; }
            field(BothRel; Rec.Rel) { TableRelation = "DL Grp".Code; }
            field(CtlRelWhere; Rec.Grp) { TableRelation = "DL Grp".Code where(Descr = const('Open')); }
            field(PageOnly; Rec.Txt) { LookupPageId = "DL Other List"; }
        }
    }
}
report 64520 "DL Report"
{
    ProcessingOnly = true;
    dataset { dataitem(Rows; "DL Item") { } }
    requestpage
    {
        layout { area(Content) { field(Off; Txt) { DrillDown = false; trigger OnDrillDown() begin Error('drilled off'); end; } } }
    }
    var Txt: Text[30];
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 64600 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

fn refused(tests: &str, reason: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        match r {
            Outcome::NotRun(why) => assert!(why.contains(reason), "{n}: {why}"),
            other => panic!("{n}: expected a refusal, got {other:?}"),
        }
    }
}

const SEED: &str =
    "local procedure Seed() var I: Record \"DL Item\"; G: Record \"DL Grp\"; O: Record \"DL Other\"; begin
    I.DeleteAll(); G.DeleteAll(); O.DeleteAll();
    G.Code := 'G1'; G.Descr := 'Open'; G.Insert(); G.Code := 'G2'; G.Descr := 'Closed'; G.Insert();
    O.Code := 'O1'; O.Descr := 'Other one'; O.Insert(); O.Code := 'O2'; O.Descr := 'Other two'; O.Insert();
    I.\"No.\" := 'I1'; I.Insert();
end;
local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;";

// pageext/lt-u-page-props/UPageProps.al:104-128 PageExt_DrillDownOnAndOff: an error of OnDrillDown comes out with the
// code TestWrapped:Dialog; DrillDown = false refuses the drill-down before the trigger runs.
#[test]
fn drill_down_false_is_refused_and_a_trigger_error_is_wrapped() {
    passes(&format!(
        "{SEED}
        [Test]
        procedure OnAndOff() var C: TestPage \"DL Card\"; begin
            Seed(); C.OpenEdit();
            asserterror C.DrillOn.Drilldown();
            Check('drilled on', GetLastErrorText()); Check('TestWrapped:Dialog', GetLastErrorCode());
            asserterror C.DrillOff.Drilldown();
            Check('The NavDrilldownAction method is not supported.', GetLastErrorText()); Check('MethodNotSupported', GetLastErrorCode());
            C.Close(); end;"
    ));
}

// report/lt-bj-request-drilldown/BjRequestDrillDown.al:226-227 and UPageProps.al:122-123: no trigger, no page.
#[test]
fn drill_down_without_trigger_or_page_raises_method_not_supported() {
    passes(&format!(
        "{SEED}
        [Test]
        procedure NoTrigger() var C: TestPage \"DL Card\"; begin
            Seed(); C.OpenEdit();
            asserterror C.Plain.Drilldown();
            Check('The NavDrilldownAction method is not supported.', GetLastErrorText()); Check('MethodNotSupported', GetLastErrorCode());
            C.Close(); end;"
    ));
}

// pages/lt-page-autoformat-lookups/P113Lookups.al:300-314 P113_LK_DrilldownNonModal (one row in the target table).
#[test]
fn drill_down_page_id_opens_that_page_non_modally() {
    passes(&format!(
        "{SEED}
        [Test] [HandlerFunctions('SeeList')]
        procedure DrillPage() var C: TestPage \"DL Card\"; G: Record \"DL Grp\"; L: Codeunit \"DL Log\"; begin
            Seed(); G.Get('G2'); G.Delete(); C.OpenEdit();
            C.DrillPage.Drilldown();
            Check('0', C.DrillPage.Value()); Check('seen;', L.Take());
            C.Close(); end;
        [PageHandler] procedure SeeList(var P: TestPage \"DL Grp List\") var L: Codeunit \"DL Log\"; begin P.First(); L.Add('seen'); end;"
    ));
}

// The rows a DrillDownPageId page shows (its filter, its first row) are not pinned: with more than one row in the
// target table the drill-down is refused.
#[test]
fn drill_down_page_id_over_several_rows_is_refused() {
    refused(
        &format!(
            "{SEED}
        [Test] [HandlerFunctions('SeeList')]
        procedure DrillPage() var C: TestPage \"DL Card\"; begin
            Seed(); C.OpenEdit(); C.DrillPage.Drilldown(); C.Close(); end;
        [PageHandler] procedure SeeList(var P: TestPage \"DL Grp List\") begin P.First(); end;"
        ),
        "DrillDownPageId",
    );
}

// pages/lt-page-editability-width-lookups/P115Lookups.al:108-124 P115_Lookup_PageIdAndRelation_OK: the control's
// LookupPageId opens modally on the first row, read only, and the pick writes the relation's field.
#[test]
fn control_relation_with_control_lookup_page_writes_the_related_field() {
    passes(&format!(
        "{SEED}
        [Test] [HandlerFunctions('PickO2')]
        procedure PageAndRelation() var C: TestPage \"DL Card\"; L: Codeunit \"DL Log\"; begin
            Seed(); C.OpenEdit();
            C.CtlRelPage.Lookup();
            Check('Other two', C.CtlRelPage.Value()); Check('O1;No;', L.Take());
            C.Close(); end;
        [ModalPageHandler] procedure PickO2(var P: TestPage \"DL Other List\") var L: Codeunit \"DL Log\"; begin
            L.Add(P.CodeCtl.Value()); L.Add(Format(P.Editable())); P.GoToKey('O2'); P.OK().Invoke(); end;"
    ));
}

// pages/lt-page-autoformat-lookups/P113Lookups.al:253-270 P113_LK_LookupWithPageRelation: without a control
// LookupPageId the related table's LookupPageId opens, and the pick is stored.
#[test]
fn control_relation_uses_the_related_tables_lookup_page() {
    passes(&format!(
        "{SEED}
        [Test] [HandlerFunctions('PickG2')]
        procedure TableLookupPage() var C: TestPage \"DL Card\"; I: Record \"DL Item\"; begin
            Seed(); C.OpenEdit();
            C.CtlRel.Lookup();
            Check('G2', C.CtlRel.Value());
            C.Close(); I.Get('I1'); Check('G2', I.Grp); end;
        [ModalPageHandler] procedure PickG2(var P: TestPage \"DL Grp List\") begin P.GoToKey('G2'); P.OK().Invoke(); end;"
    ));
}

// Not pinned: which page wins when the control and the related table name different lookup pages.
#[test]
fn control_lookup_page_other_than_the_tables_is_refused() {
    refused(
        &format!(
            "{SEED}
        [Test] [HandlerFunctions('PickO2')]
        procedure Differ() var C: TestPage \"DL Card\"; begin Seed(); C.OpenEdit(); C.CtlRelOtherPage.Lookup(); C.Close(); end;
        [ModalPageHandler] procedure PickO2(var P: TestPage \"DL Other List\") begin P.OK().Invoke(); end;"
        ),
        "LookupPageId",
    );
}

// Not pinned: a control relation over a field that has its own table relation, and a control relation with where().
#[test]
fn control_relation_over_a_table_relation_or_with_where_is_refused() {
    refused(
        &format!(
            "{SEED}
        [Test] [HandlerFunctions('PickG2')]
        procedure Both() var C: TestPage \"DL Card\"; begin Seed(); C.OpenEdit(); C.BothRel.Lookup(); C.Close(); end;
        [ModalPageHandler] procedure PickG2(var P: TestPage \"DL Grp List\") begin P.OK().Invoke(); end;"
        ),
        "TableRelation",
    );
    refused(
        &format!(
            "{SEED}
        [Test] [HandlerFunctions('PickG2')]
        procedure Where() var C: TestPage \"DL Card\"; begin Seed(); C.OpenEdit(); C.CtlRelWhere.Lookup(); C.Close(); end;
        [ModalPageHandler] procedure PickG2(var P: TestPage \"DL Grp List\") begin P.OK().Invoke(); end;"
        ),
        "TableRelation",
    );
}

// pages/lt-page-editability-width-lookups/P115Lookups.al:173-191 P115_AssistEdit_OnFieldsWithoutTrigger,
// lookups/W2Lookups.al:569, :593: AssistEdit with no trigger raises a fixed platform error.
#[test]
fn assist_edit_without_trigger_raises_method_not_supported() {
    passes(&format!(
        "{SEED}
        [Test]
        procedure NoTrigger() var C: TestPage \"DL Card\"; begin
            Seed(); C.OpenEdit();
            asserterror C.CtlRelPage.AssistEdit();
            Check('The NavAssistEditAction method is not supported.', GetLastErrorText()); Check('MethodNotSupported', GetLastErrorCode());
            asserterror C.BothRel.AssistEdit();
            Check('The NavAssistEditAction method is not supported.', GetLastErrorText()); Check('MethodNotSupported', GetLastErrorCode());
            C.Close(); end;"
    ));
}

// pageext/lt-z-modify-controls/ZModifyControls.al:435-452 ZLookupPageId_OpensTargetPage: a control LookupPageId over a
// field with no relation raises nothing, needs no handler and leaves the value.
#[test]
fn control_lookup_page_without_relation_does_nothing() {
    passes(&format!(
        "{SEED}
        [Test]
        procedure PageOnly() var C: TestPage \"DL Card\"; begin
            Seed(); C.OpenEdit(); C.PageOnly.SetValue('x'); C.PageOnly.Lookup(); Check('x', C.PageOnly.Value()); C.Close(); end;"
    ));
}

// Not pinned: `DrillDown = false` on a request page control (UPageProps.al pins it on pages only).
#[test]
fn drill_down_false_on_a_request_page_is_refused() {
    refused(
        &format!(
            "{SEED}
        [Test] [HandlerFunctions('Rp')]
        procedure RpOff() var Probe: Report \"DL Report\"; begin Seed(); Commit(); Probe.Run(); end;
        [RequestPageHandler] procedure Rp(var R: TestRequestPage \"DL Report\") begin R.Off.Drilldown(); end;"
        ),
        "DrillDown = false on a request page",
    );
}
