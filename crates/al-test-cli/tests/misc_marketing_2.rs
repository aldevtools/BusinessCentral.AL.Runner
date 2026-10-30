//! Behaviours Microsoft's Tests-Misc and Tests-Marketing reach. Each test says what pins it; docs/probes-needed.md has the rows
//! for what nothing pins.

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const OBJECTS: &str = "
table 69400 \"MM Head\" { fields { field(1; No; Code[20]) { } } keys { key(PK; No) { Clustered = true; } } }
table 69402 \"MM Doc\" { fields { field(1; \"Document Type\"; Option) { OptionMembers = \" \",Invoice,\"Credit Memo\"; } field(2; No; Code[20]) { } } keys { key(PK; \"Document Type\", No) { Clustered = true; } } }
table 69401 \"MM Line\" { fields { field(1; No; Code[20]) { } field(2; Line; Integer) { } field(3; Sub; Code[10]) { } }
  keys { key(PK; No, Line, Sub) { Clustered = true; } } }
";

fn run(tests: &str) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![(
                "f.al".into(),
                format!("{OBJECTS} codeunit 69402 T {{ Subtype = Test; {tests} }}"),
            )],
        }],
        &Default::default(),
    );
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

// A Variant that holds a record of one table takes a record of another table whole (Library - Variable Storage.Dequeue into one
// variable twice, Tests-Marketing AddContactsToSegments). Before: the second record was copied into the first one's buffer.
#[test]
fn a_variant_holding_a_record_takes_a_record_of_another_table() {
    all_pass(
        "var Q: array[2] of Variant;
         [Test] procedure Replace() var H: Record \"MM Head\"; L: Record \"MM Line\"; V: Variant; RR: RecordRef; begin
             Q[1] := H; Q[2] := L;
             V := Q[1]; RR.GetTable(V); if RR.Number <> 69400 then Error('first %1', RR.Number);
             V := Q[2]; Clear(RR); RR.GetTable(V); if RR.Number <> 69401 then Error('second %1', RR.Number);
         end;",
    );
}

// Evaluate into a RecordId whose first key field is an Option (types/lt-wc/WcUnpinnedSingles.al:112-146, al-runner issue 6322):
// a member name (also with a space), an ordinal (also past the members: Format shows `9`), an empty unquoted value (ordinal 0,
// Format shows `" "`); the same under language 1031; an unknown name raises InvalidOptionString naming the members.
#[test]
fn evaluate_into_a_record_id_reads_an_option_key_field() {
    all_pass(
        r#"[Test] procedure OptionKey() var L: Record "MM Doc"; Rid: RecordId; begin
             L."Document Type" := L."Document Type"::Invoice; L.No := 'D-001'; L.Insert();
             if Format(L.RecordId) <> 'MM Doc: Invoice,D-001' then Error('format %1', Format(L.RecordId));
             if not Evaluate(Rid, 'MM Doc: Invoice,D-001') then Error('name');
             if Rid <> L.RecordId then Error('name equal');
             if not Evaluate(Rid, 'MM Doc: 1,D-001') then Error('ordinal');
             if Rid <> L.RecordId then Error('ordinal equal');
             if not Evaluate(Rid, 'MM Doc: Credit Memo,D-001') then Error('space');
             if Format(Rid) <> 'MM Doc: Credit Memo,D-001' then Error('space %1', Format(Rid));
             if not Evaluate(Rid, 'MM Doc: ,D-001') then Error('blank');
             if Format(Rid) <> 'MM Doc: " ",D-001' then Error('blank %1', Format(Rid));
             if not Evaluate(Rid, 'MM Doc: 9,D-001') then Error('nine');
             if Format(Rid) <> 'MM Doc: 9,D-001' then Error('nine %1', Format(Rid));
             if Evaluate(Rid, 'MM Doc: Quote,D-001') then Error('quote');
             if GetLastErrorText() <> '''Quote'' is not an option. The existing options are:  ,Invoice,Credit Memo' then Error('text %1', GetLastErrorText());
             if GetLastErrorCode() <> 'InvalidOptionString' then Error('code %1', GetLastErrorCode());
             GlobalLanguage(1031);
             if not Evaluate(Rid, 'MM Doc: Invoice,D-001') then Error('name 1031');
             if Rid <> L.RecordId then Error('name 1031 equal');
             GlobalLanguage(1033);
         end;"#,
    );
}

// The shapes of an Option key value WcUnpinnedSingles.al does not pin are refused (docs/probes-needed.md).
#[test]
fn evaluate_into_a_record_id_refuses_the_unpinned_option_key_shapes() {
    for v in ["invoice", " Invoice", "\"\""] {
        let o = run(&format!(
            "[Test] procedure T1() var Rid: RecordId; begin if Evaluate(Rid, 'MM Doc: {v},D-001') then; end;"
        ));
        assert!(
            matches!(&o[0].1, Outcome::NotRun(why) if why.contains("Option key field")),
            "{v}: {:?}",
            o[0].1
        );
    }
}

// errors/lt-ms-refusals-128/P128EvalRecordId.al pins a single-field key, types/lt-evaluate-recordid/EvaluateRecordIdComposite.al a composite
// one (Library - Error Message.LoadErrorMessages): fewer values than key fields are kept, more are rejected.
#[test]
fn evaluate_into_a_record_id_of_a_composite_key_reads_one_value_per_field() {
    all_pass(
        "[Test] procedure Composite() var L: Record \"MM Line\"; Rid: RecordId; begin
             L.No := 'A'; L.Line := 10; L.Sub := 'X'; L.Insert();
             if not Evaluate(Rid, Format(L.RecordId)) then Error('eval');
             if Format(Rid) <> Format(L.RecordId) then Error('text %1', Format(Rid));
             if Rid <> L.RecordId then Error('equal');
             if not Evaluate(Rid, 'MM Line: A,10') then Error('too few');
             if Evaluate(Rid, 'MM Line: A,10,X,Y') then Error('too many');
             if Evaluate(Rid, 'MM Line: A,x,X') then Error('bad integer');
         end;",
    );
}
