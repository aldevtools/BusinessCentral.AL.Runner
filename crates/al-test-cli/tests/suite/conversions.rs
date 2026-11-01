//! Conversions between Enum, Option, Integer and BigInteger values that Microsoft's tests reach (Sales Line
//! UpdatePlanned, Sales Price Calc. Mgt. FindSalesLineDisc, Application Area Mgmt. SaveExperienceTierCurrentCompany).
//! Each test cites what pins the behaviour; where nothing does, the test says so and docs/probes-needed.md has a row.

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const OBJECTS: &str = "
enum 69300 \"CV Kind\" { Extensible = true; value(0; None) { } value(1; One) { } value(2; Two) { } }
table 69300 \"CV Row\" { fields { field(1; Code; Code[20]) { } field(7; Number; Integer) { } field(2; Kind; Option) { OptionMembers = A,B,C; }
  field(3; Flag; Boolean) { } field(4; Pick; Enum \"CV Kind\") { } }
  keys { key(PK; Code) { Clustered = true; } } }
";

fn compile(src: String) -> (al_sema::workspace::Workspace, al_sema::lower::Lowered) {
    al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    )
}

fn run(tests: &str) -> Vec<(String, Outcome)> {
    let (ws, l) = compile(format!("{OBJECTS} codeunit 69301 T {{ Subtype = Test; {tests} }}"));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    run_codeunit(&engine, &l, oi)
}

fn all_pass(tests: &str) {
    for (name, outcome) in run(tests) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

// Application Area Mgmt. does `FieldRef := RecRef.Field(Buffer."Field No.")`: a record field as the argument is the
// field's VALUE for RecordRef.Field, not a reference to the field (the parameter is an Integer). Corpus
// recordref/TestRecordRefRuntimeContracts.al passes Integer variables; Microsoft's test
// Test ApplicationArea Setup.TestSetExperienceTierCurrentCompanyErrorOnInvalidValues needs the field form.
#[test]
fn recordref_field_takes_the_value_of_a_record_field_argument() {
    all_pass(
        "[Test] procedure FieldByRecordField() var Row: Record \"CV Row\"; Buf: Record \"CV Row\" temporary; RecRef: RecordRef; FldRef: FieldRef; B: Boolean; begin
             Buf.Number := 3; Buf.Insert();
             RecRef.GetTable(Row);
             FldRef := RecRef.Field(Buf.Number);
             if FldRef.Name <> 'Flag' then Error('name %1', FldRef.Name);
             if not RecRef.FieldExist(Buf.Number) then Error('exist');
             Row.Flag := true; RecRef.GetTable(Row); FldRef := RecRef.Field(Buf.Number); B := FldRef.Value;
             if not B then Error('value');
         end;",
    );
}

// Sales Price Calc. Mgt. FindSalesLineDisc: `for R."Sales Type" := R."Sales Type"::Customer to R."Sales Type"::Campaign`.
// Each round assigns the counter to the Option field as its ordinal. No corpus test loops over an Option field
// (docs/probes-needed.md); the loop over an Option bound with an Integer variable is pinned by PROptionInteger.al.
#[test]
fn a_for_loop_over_an_option_field_visits_every_member() {
    all_pass(
        "[Test] procedure LoopOverOptionField() var Row: Record \"CV Row\"; n: Integer; seen: Text; begin
             for Row.Kind := Row.Kind::A to Row.Kind::C do begin n += 1; seen += Format(Row.Kind); end;
             if n <> 3 then Error('rounds %1', n);
             if seen <> 'ABC' then Error('seen %1', seen);
         end;",
    );
}

// A RecordId as a `%1` of an error: Gen. Jnl.-Post Line (ERM Gen. Lines for Vendor.EqualAmountToApplyOnInvoice).
// Format(RecordId) is pinned (record/TestProbeRecordRefContracts.al:472-502, `PA Rec Row: 42`); that an error message
// shows it the way Format does is assumed (docs/probes-needed.md).
#[test]
fn a_record_id_in_an_error_message_reads_as_format_gives_it() {
    all_pass(
        "[Test] procedure RecordIdInError() var Row: Record \"CV Row\"; begin
             Row.Code := 'A';
             asserterror Error('at %1', Row.RecordId);
             if GetLastErrorText <> 'at CV Row: A' then Error('text %1', GetLastErrorText);
         end;",
    );
}
