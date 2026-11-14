//! Records that platform code filled, or that are not the declared variables the Blob copy probes measured (cells F10, F12,
//! F15, F22, F23 of the round-3 review of al-runner PR 7951): passed by value they share their Blob with the callee, as before
//! assumption `blob-transfer-copies`. Each test ends with `Error('OBS ...')` of what the caller reads after the callee wrote
//! 'zz': `zz` at the start is the shared Blob.

const SRC: &str = r#"
table 50460 "RV Row"
{
    fields
    {
        field(1; Key1; Integer) { }
        field(2; Data; Blob) { }
        field(3; Name; Text[30]) { }
    }
    keys { key(PK; Key1) { Clustered = true; } }
}
codeunit 50461 "RV Lib"
{
    procedure W(var R: Record "RV Row"; T: Text)
    var OS: OutStream;
    begin R.Data.CreateOutStream(OS); OS.WriteText(T); end;

    procedure Rd(var R: Record "RV Row"): Text
    var IS: InStream; T: Text;
    begin
        R.Data.CreateInStream(IS); IS.ReadText(T);
        if R.Data.HasValue() then exit('hv:' + T);
        exit('nohv:' + T);
    end;
}
codeunit 50462 "RV RunByValue"
{
    TableNo = "RV Row";
    var L: Codeunit "RV Lib";
    trigger OnRun() begin L.W(Rec, 'alpha'); ByValue(Rec); Error('OBS %1', L.Rd(Rec)); end;
    local procedure ByValue(R: Record "RV Row") begin L.W(R, 'zz'); end;
}
page 50463 "RV List"
{
    PageType = List;
    SourceTable = "RV Row";
    UsageCategory = None;
    layout { area(Content) { repeater(G) { field(Key1; Rec.Key1) { } field(Name; Rec.Name) { } } } }
    actions
    {
        area(Processing)
        {
            action(Sel)
            {
                trigger OnAction()
                var S: Record "RV Row";
                begin CurrPage.SetSelectionFilter(S); Clear(S.Data); L.W(S, 'beta'); ByValue(S); Error('OBS %1', L.Rd(S)); end;
            }
            action(Base)
            {
                trigger OnAction()
                var S: Record "RV Row";
                begin L.W(S, 'beta'); ByValue(S); Error('OBS %1', L.Rd(S)); end;
            }
        }
    }
    var L: Codeunit "RV Lib";
    local procedure ByValue(R: Record "RV Row") begin L.W(R, 'zz'); end;
}
codeunit 50464 "RV N4"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        L: Codeunit "RV Lib";

    local procedure ByValue(R: Record "RV Row") begin L.W(R, 'zz'); end;
    local procedure Seed() var R: Record "RV Row"; begin R.DeleteAll(); R.Key1 := 1; R.Name := 'one'; R.Insert(); end;
    local procedure Make() R: Record "RV Row" begin L.W(R, 'alpha'); ByValue(R); Error('OBS %1', L.Rd(R)); end;

    [Test] procedure F00_Baseline_WrittenByValue()
    var Src: Record "RV Row";
    begin L.W(Src, 'alpha'); ByValue(Src); Error('OBS %1', L.Rd(Src)); end;

    [Test] [HandlerFunctions('ListOK')] procedure F10_PageRunModalLookup_ClearWrite_ByValue()
    var Src: Record "RV Row";
    begin Seed(); if Page.RunModal(Page::"RV List", Src) = Action::LookupOK then; Clear(Src.Data); L.W(Src, 'beta'); ByValue(Src); Error('OBS %1 %2', L.Rd(Src), Src.Name); end;

    [ModalPageHandler] procedure ListOK(var TP: TestPage "RV List") begin TP.First(); TP.OK().Invoke(); end;

    [Test] procedure F12_PageSetSelectionFilter_ClearWrite_ByValue()
    var TP: TestPage "RV List";
    begin Seed(); TP.OpenView(); TP.First(); TP.Sel.Invoke(); end;

    [Test] procedure F15_PageLocalBaseline()
    var TP: TestPage "RV List";
    begin Seed(); TP.OpenView(); TP.First(); TP.Base.Invoke(); end;

    [Test] procedure F22_NamedReturnValue_ByValue()
    begin Make(); end;

    [Test] procedure F23_CodeunitRunRec_ByValueInside()
    var Src: Record "RV Row";
    begin Codeunit.Run(Codeunit::"RV RunByValue", Src); end;
}
"#;

/// The `OBS` text test `name` ends with.
fn observed(name: &str) -> String {
    let outcomes = al_testkit::outcomes_in(SRC, "RV N4");
    match outcomes.iter().find(|(n, _)| n == name) {
        Some((_, al_testrun::Outcome::Fail(text))) => {
            let at = text.find("OBS ").unwrap_or_else(|| panic!("{name}: {text}"));
            text[at..].lines().next().unwrap_or_default().to_string()
        }
        other => panic!("{name}: {other:?}"),
    }
}

/// The measured source: a local record of a codeunit procedure, written and passed by value, keeps its Blob (corpus PR 1375,
/// streams/BlobCopyStates.al:1165 ByValue_Mem at c33cc186).
#[test]
fn a_written_local_record_by_value_keeps_its_blob() {
    assert_eq!(observed("F00_Baseline_WrittenByValue"), "OBS hv:alpha");
}

/// F10: `Page.RunModal(Id, Rec)` answered LookupOK hands the picked row to the record. F12: `CurrPage.SetSelectionFilter(Rec)`
/// fills a local record of a page action. No probe measures a record after either.
#[test]
fn a_record_a_page_filled_shares_by_value_as_before() {
    assert_eq!(observed("F10_PageRunModalLookup_ClearWrite_ByValue"), "OBS hv:zzta one");
    assert_eq!(observed("F12_PageSetSelectionFilter_ClearWrite_ByValue"), "OBS hv:zzta");
}

/// F15: a local of a page action trigger. F22: a named return value. F23: Rec of an OnRun trigger, handed in by
/// `Codeunit.Run(Id, Rec)`. The probes' sources are local variables of codeunit procedures and codeunit globals.
#[test]
fn a_record_that_is_not_a_measured_variable_shares_by_value_as_before() {
    assert_eq!(observed("F15_PageLocalBaseline"), "OBS hv:zzta");
    assert_eq!(observed("F22_NamedReturnValue_ByValue"), "OBS hv:zzpha");
    assert_eq!(observed("F23_CodeunitRunRec_ByValueInside"), "OBS hv:zzpha");
}
