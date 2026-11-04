//! Behaviours that corpus cases pinned and the runtime got wrong (branch corpus-fails-1). Each test cites its case.

use al_builtins::value::Value;

fn text(vars: &str, body: &str) -> String {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; }}");
    match al_testkit::run_in(&src, "C", "F") {
        Ok(Value::Text(t) | Value::Code(t)) => t.to_string(),
        Err(al_builtins::native::Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        other => format!("{other:?}"),
    }
}

#[test]
fn a_dictionary_key_added_after_a_remove_takes_the_freed_slot() {
    // collections/DictBasic.al:Dict_SetRemove
    let r = text(
        "D: Dictionary of [Text, Integer]; K: Text; R: Text;",
        "D.Add('a', 1); D.Add('b', 2); D.Add('c', 3); D.Remove('b'); D.Add('b', 22); foreach K in D.Keys() do R += K; exit(R + Format(D.Count()));",
    );
    assert_eq!(r, "abc3");
}

#[test]
fn copy_array_raises_when_the_length_is_not_the_destination_length() {
    // collections/Arrays.al:CopyArray_Forms
    let r = text(
        "S: array[5] of Integer; D: array[5] of Integer;",
        "CopyArray(D, S, 3); exit('no error');",
    );
    assert_eq!(
        r,
        "error:CopyArray: The length parameter 3 does not match the length of the destination array 5.|ArrayLengthMismatch"
    );
}

#[test]
fn an_unquoted_empty_option_member_shows_its_number_and_a_quoted_one_a_space() {
    // text/P69OptionInText.al:Option_StrSubstNo, types/decimal-integer/ScT70053.al:Option_Format
    let r = text(
        "O: Option Zero,,Two,\"\"; R: Text;",
        "O := 1; R := '[' + Format(O) + ']'; O := 3; exit(R + '[' + Format(O) + ']');",
    );
    assert_eq!(r, "[1][ ]");
}

#[test]
fn a_notification_assignment_copies_it() {
    // errors/errorinfo/EnTNotification.al:Copy_Semantics
    let r = text(
        "A: Notification; B: Notification;",
        "A.Message := 'one'; B := A; B.Message := 'two'; exit(A.Message() + B.Message());",
    );
    assert_eq!(r, "onetwo");
}

#[test]
fn the_isolation_level_option_shows_member_names_in_the_number_form() {
    // format-evaluate/PnAPlatformOptions.al:IsolationLevel_Members
    let r = text(
        "L: IsolationLevel;",
        "L := IsolationLevel::UpdLock; exit(Format(L, 0, 2));",
    );
    assert_eq!(r, "UpdLock");
}

#[test]
fn a_negative_decimal_too_small_to_show_reads_minus_zero_in_the_xml_form() {
    // streams/instream-outstream/PnAStreamDecimalBytes.al:Write_Decimal_Tiny `minus-1E-28`
    let r = text(
        "D: Decimal;",
        "Evaluate(D, '-0.0000000000000000000000000001', 9); exit(Format(D, 0, 9));",
    );
    assert_eq!(r, "-0");
}
