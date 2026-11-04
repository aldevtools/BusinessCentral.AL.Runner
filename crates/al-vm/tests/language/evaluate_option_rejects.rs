//! `Evaluate` into platform options and Option array elements whose texts are neither an ordinal nor a member name.

use al_builtins::native::Signal;

fn run(src: &str) -> String {
    al_testkit::text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v))
}

#[test]
fn evaluate_into_table_connection_type_reads_an_ordinal_and_rejects_one_past_the_members() {
    // format-evaluate/W5TableConnections.al:117-136 (TableConnectionType_Members).
    let src = "codeunit 50100 C { procedure F(): Text var K: TableConnectionType; i: Integer; M: Text; begin \
               for i := 0 to 10 do if Evaluate(K, Format(i)) then M += Format(i) + '=' + Format(K) + ';'; exit(M); end; }";
    assert_eq!(run(src), "0=CRM;1=ExternalSQL;2=Exchange;3=MicrosoftGraph;4=Query;");
}

#[test]
fn evaluate_into_execution_mode_reads_an_ordinal_and_rejects_two_to_twelve() {
    // format-evaluate/PnC11SystemEnums.al:13-52 (ExecutionMode_Members).
    let src = "codeunit 50100 C { procedure F(): Text var K: ExecutionMode; i: Integer; M: Text; begin \
               for i := 0 to 12 do if Evaluate(K, Format(i), 9) then M += Format(K) + ';' else M += 'no;'; exit(M); end; }";
    assert_eq!(run(src), "Standard;Debug;no;no;no;no;no;no;no;no;no;no;no;");
}

#[test]
fn evaluate_into_an_option_array_element_of_an_unknown_name_answers_false_and_keeps_the_element() {
    // types/arrays/PnC14OptionArray.al:51-78 (OptionArray_Evaluate_And_Out_Of_Range).
    let src = "codeunit 50100 C { procedure F(): Text var K: array[3] of Option \"None\",Released,FirmPlanned,Purchase; \
               ok: Boolean; R: Text; begin ok := Evaluate(K[1], 'FirmPlanned'); R := Format(ok) + Format(K[1]); \
               ok := Evaluate(K[1], 'nothing'); exit(R + '|' + Format(ok) + Format(K[1])); end; }";
    assert_eq!(run(src), "YesFirmPlanned|NoFirmPlanned");
}

#[test]
fn evaluate_into_a_platform_option_of_an_unpinned_number_is_refused() {
    for (ty, text) in [("TableConnectionType", "11"), ("ExecutionMode", "13")] {
        let src = format!(
            "codeunit 50100 C {{ procedure F(): Boolean var K: {ty}; begin exit(Evaluate(K, '{text}')); end; }}"
        );
        match al_testkit::run_with(&src, "C", "F", vec![]) {
            Err(Signal::Unsupported(_)) => {}
            other => panic!("{ty} {text}: {other:?}"),
        }
    }
}
