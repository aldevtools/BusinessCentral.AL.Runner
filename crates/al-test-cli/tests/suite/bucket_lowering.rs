//! Lowering gaps found by running Microsoft's test buckets (branch bucket-lowering). The
//! fixtures are small AL written for this file; each test names the corpus test that pins the
//! behaviour it relies on.

use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

fn outcomes(main: &str) -> Vec<(String, Outcome)> {
    let symbols = al_version::corpus_symbols(27);
    let (_, l) = al_sema::lower::compile_apps_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: Vec::new(),
            files: vec![("main.al".into(), main.into())],
        }],
        vec![AppKind::Workspace],
        &symbols,
        None,
        al_sema::check::Roots::default(),
    );
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "T")
        .expect("test codeunit T");
    run_codeunit(&engine, &l, oi)
}

fn passes(o: &[(String, Outcome)], test: &str) {
    match o.iter().find(|(n, _)| n == test).map(|(_, o)| o) {
        Some(Outcome::Pass) => {}
        other => panic!("{test} was expected to pass, got {other:?}"),
    }
}

/// `EI.Message := 'x'` is the property form of `EI.Message('x')`; the storage and the getter are
/// pinned through the call form (errorinfo/TestErrorInfoType.al:249-271 Message and
/// DetailedMessage, :284-307 Verbosity, :313-345 CustomDimensions, errorinfo/
/// TestErrorInfoErrorType.al:25-45 ErrorType).
#[test]
fn error_info_members_assign_as_properties() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Texts() var EI: ErrorInfo; begin
    EI := ErrorInfo.Create();
    EI.Message := 'MSG-1'; EI.DetailedMessage := 'DETAIL-3'; EI.Title := 'TTL';
    if EI.Message() <> 'MSG-1' then Error('message %1', EI.Message());
    if EI.DetailedMessage() <> 'DETAIL-3' then Error('detailed');
    if EI.Title() <> 'TTL' then Error('title');
    if EI.Message <> 'MSG-1' then Error('getter'); end;
  [Test] procedure Options() var EI: ErrorInfo; begin
    EI := ErrorInfo.Create();
    EI.Verbosity := Verbosity::Warning; EI.ErrorType := ErrorType::Client; EI.Collectible := false;
    if Format(EI.Verbosity()) <> 'Warning' then Error('verbosity');
    if EI.Collectible() then Error('collectible'); end;
  [Test] procedure Numbers() var EI: ErrorInfo; begin
    EI := ErrorInfo.Create();
    EI.FieldNo := 7; EI.PageNo := 9; EI.TableId := 3;
    if (EI.FieldNo() <> 7) or (EI.PageNo() <> 9) or (EI.TableId() <> 3) then Error('numbers'); end; }"#,
    );
    for t in ["Texts", "Options", "Numbers"] {
        passes(&o, t);
    }
}

const MARKS: &str = r#"table 50001 "Thing" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } }
  procedure GroupInTableCode(): Integer begin FilterGroup := 2; exit(FilterGroup); end;
  procedure MarkedInTableCode(): Boolean begin MarkedOnly := true; exit(MarkedOnly); end; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure MarkAndMarkedOnly() var R: Record "Thing"; begin
    R.DeleteAll(); R."No." := 1; R.Insert(); R."No." := 2; R.Insert();
    R.FindFirst(); R.Mark := true;
    if not R.Mark then Error('mark getter');
    R.MarkedOnly := true;
    if R.Count() <> 1 then Error('marked only count %1', R.Count());
    if not R.MarkedOnly then Error('marked only getter'); end;
  [Test] procedure AscendingProperty() var R: Record "Thing"; begin
    R.DeleteAll(); R."No." := 1; R.Insert(); R."No." := 2; R.Insert();
    R.Ascending := false;
    R.FindFirst(); if R."No." <> 2 then Error('descending first %1', R."No.");
    if R.Ascending then Error('ascending getter'); end;
  [Test] procedure ImplicitRecInTableCode() var R: Record "Thing"; begin
    if R.GroupInTableCode() <> 2 then Error('group');
    if not R.MarkedInTableCode() then Error('marked'); end;
  [Test] procedure WorkDateAssignment() var Old: Date; begin
    Old := WorkDate(); WorkDate := 20240101D;
    if WorkDate() <> 20240101D then Error('workdate %1', WorkDate()); WorkDate := Old; end; }"#;

/// `Rec.Mark := b`, `Rec.MarkedOnly := b`, `Rec.Ascending := b` and `WorkDate := d` are the
/// property forms of the calls the corpus pins (record/TestRecordFind.al:237-284 for Mark and
/// MarkedOnly, handlers/TestPartOwnSortVsSubPageView.al:187 for Ascending, session/
/// TestSessionFunctions.al:63-65 for WorkDate). Unqualified in table code they act on the
/// implicit Rec.
#[test]
fn record_properties_and_work_date_assign() {
    let o = outcomes(MARKS);
    for t in [
        "MarkAndMarkedOnly",
        "AscendingProperty",
        "ImplicitRecInTableCode",
        "WorkDateAssignment",
    ] {
        passes(&o, t);
    }
}

/// `ErrorInfo.RecordId` can be assigned and read back (record/ProbePropertyAssignmentForms.al:
/// ErrorInfo_RecordId_AssignAndDefault); a routine that does not reach the assignment runs as well.
#[test]
fn assigning_error_info_record_id_runs() {
    let o = outcomes(
        r#"codeunit 50001 "Helper" {
  procedure Fill(var EI: ErrorInfo; Rid: RecordId; Go: Boolean) begin if Go then EI.RecordId := Rid; end; }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure NotReached() var H: Codeunit "Helper"; EI: ErrorInfo; Rid: RecordId; begin
    EI := ErrorInfo.Create(); H.Fill(EI, Rid, false); end;
  [Test] procedure Reached() var H: Codeunit "Helper"; EI: ErrorInfo; Rid: RecordId; begin
    EI := ErrorInfo.Create(); H.Fill(EI, Rid, true); end; }"#,
    );
    passes(&o, "NotReached");
    passes(&o, "Reached");
}

/// Unqualified in a dataitem trigger, `FilterGroup := g` and `MarkedOnly := b` act on the
/// dataitem's record, as the unqualified call forms do. The properties are the ones pinned above;
/// the dataitem record in triggers is pinned by handlers/RgfFilterReport.Report.al:38 (`Rows`).
#[test]
fn dataitem_record_properties_assign_unqualified_in_report_triggers() {
    let o = outcomes(
        r#"table 50002 "Row" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 50003 "Log" { fields { field(1; "Key"; Integer) { } field(2; Seen; Integer) { } } keys { key(PK; "Key") { Clustered = true; } } }
report 50004 "Rep" { ProcessingOnly = true;
  dataset { dataitem(Rows; "Row") {
    trigger OnPreDataItem() var L: Record "Log"; begin
      FilterGroup := 2; MarkedOnly := true;
      L.DeleteAll(); L."Key" := 1; L.Seen := FilterGroup; L.Insert(); end; } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Unqualified() var Rep: Report "Rep"; L: Record "Log"; begin
    Rep.UseRequestPage(false); Rep.RunModal();
    L.Get(1); if L.Seen <> 2 then Error('group %1', L.Seen); end; }"#,
    );
    passes(&o, "Unqualified");
}

/// Microsoft's tests write `Page.FindFirstField(Name, Value)` with the control named bare, and so
/// does the corpus (handlers/TestPagePlatformPageSourceTable_Tests.al:66, :125): a bare name in the
/// field position of a test page method is a control of that page.
#[test]
fn a_bare_control_name_as_a_testfield_argument_is_that_pages_control() {
    let o = outcomes(
        r#"table 50002 "Row" { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { Clustered = true; } } }
page 50010 "Rows" { PageType = List; SourceTable = "Row"; layout { area(Content) { repeater(R) { field("No."; Rec."No.") { } } } } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Bare() var TP: TestPage "Rows"; begin TP.OpenView(); if TP.FindFirstField("No.", 1) then Error('no rows, no hit'); end; }"#,
    );
    passes(&o, "Bare");
}

/// Microsoft's code compares `ClientType::ODataV4` (Price List Lines, Calc. Running GL. Acc.
/// Balance). The member is a name of the platform option; its position is not pinned
/// (session/TestSessionExtended.al:21,31 only calls the Session functions), so nothing here may
/// depend on it.
#[test]
fn the_odata_v4_client_type_member_exists_and_differs_from_the_others() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Names() var C: ClientType; begin
    C := ClientType::ODataV4;
    if C = ClientType::OData then Error('same as OData');
    if C <> ClientType::ODataV4 then Error('not itself'); end; }"#,
    );
    passes(&o, "Names");
}

/// Microsoft's tests call `BindSubscription(this)`. The corpus pins it (handlers/
/// ProbeEventSubscriberOrder.al: BindSubscription_This_BindsOwnInstance): `this` is the running
/// instance. A codeunit that is not Manual gets the binding error of any other codeunit.
#[test]
fn this_passed_to_bind_subscription_is_the_running_instance() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Bind() begin BindSubscription(this); end; }"#,
    );
    match o.iter().find(|(n, _)| n == "Bind").map(|(_, o)| o) {
        Some(Outcome::Fail(why)) => assert!(why.contains("does not use manual binding"), "{why}"),
        other => panic!("{other:?}"),
    }
}

/// `Enum::Ns.Sub."Name".FromInteger(n)` names the enum through its namespace
/// (`Enum::Microsoft.Manufacturing.Document."Planning Create Prod. Order"` in Microsoft's code).
/// The static members are pinned for the plain form (enum shipped-ordinal tests: types/
/// TestEnumShippedOrdinalTests.Codeunit.al:91 `FromInteger`, types/TestDateRoundFormatStringContracts.al:221
/// `Names`), and a namespace-qualified object name resolves as `Codeunit::A.B."X"` does
/// (fieldref/TestFieldRefRelation.al:270-325).
#[test]
fn a_namespace_qualified_enum_has_the_static_members() {
    let o = outcomes(
        r#"namespace Acme.Sales.Docs;
enum 50010 "Kind" { Extensible = true; value(0; Alpha) { } value(1; Beta) { } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure FromIntegerAndNames() var K: Enum "Acme.Sales.Docs"."Kind"; N: List of [Text]; begin
    K := Enum::Acme.Sales.Docs."Kind".FromInteger(1);
    if K <> Enum::Acme.Sales.Docs."Kind"::Beta then Error('from integer');
    N := Enum::Acme.Sales.Docs."Kind".Names();
    if N.Count() <> 2 then Error('names %1', N.Count()); end; }"#,
    );
    passes(&o, "FromIntegerAndNames");
}

/// `SupplyType[1] := SupplyType::Purchase` with `SupplyType: array[5] of Option "None",Released,
/// FirmPlanned,Purchase` (Microsoft's SCM Supply Planning tests): `Arr::Member` names the member of
/// the element's option, as `Opt::Member` does for a scalar option (handlers/
/// TestPageFieldFormats_Tests.al:1160-1162 `Row.Opt::Beta`). Arrays of options themselves are
/// listed in docs/probes-needed.md, Probe batch 4.
#[test]
fn an_array_of_options_names_members_with_a_double_colon() {
    let o = outcomes(
        r#"codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Members() var Kind: array[3] of Option "None",Released,FirmPlanned,Purchase; begin
    Kind[1] := Kind::Purchase; Kind[2] := Kind::Released;
    if Kind[1] <> Kind::Purchase then Error('purchase');
    if Kind[2] = Kind::None then Error('none');
    if Kind[1] = Kind[2] then Error('same'); end; }"#,
    );
    passes(&o, "Members");
}

/// `Ns.Sub."Enum"::Value` names a member of a namespace-qualified enum without the `Enum::`
/// keyword (`Microsoft.Manufacturing.ProductionBOM."Production BOM Line Type"::Item` in Microsoft's
/// code). The member lookup is pinned for the plain forms (`"ALT Status"::Active`, types/
/// TestEnumNamesContracts.al:76; `Enum::"TPEDO Kind"::Beta`, handlers/TestPageEnumDeclaredOrder_Tests.al:56)
/// and the namespace resolution as for `Enum::Ns."Name"` above.
#[test]
fn a_namespace_qualified_enum_names_its_members_without_the_enum_keyword() {
    let o = outcomes(
        r#"namespace Acme.Sales.Docs;
enum 50010 "Kind" { Extensible = true; value(0; Alpha) { } value(1; Beta) { } }
codeunit 50000 "T" { Subtype = Test;
  [Test] procedure Members() var K: Enum "Acme.Sales.Docs"."Kind"; begin
    K := Acme.Sales.Docs."Kind"::Beta;
    if K <> Enum::Acme.Sales.Docs."Kind"::Beta then Error('member');
    if K.AsInteger() <> 1 then Error('ordinal'); end; }"#,
    );
    passes(&o, "Members");
}
