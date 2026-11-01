//! A JsonValue set with a Date, Time or DateTime and read as one of the other two raises a Conversion error, and a Time or
//! DateTime with milliseconds reads as the text it is written as (json/lt-json-overloads/P88DateTimeReadback.al:78-210).

use al_builtins::native::Signal;

fn run(set: &str, read: &str) -> Result<al_builtins::value::Value, Signal> {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var V: JsonValue; begin V.SetValue({set}); exit(Format(V.{read}(), 0, 9)); end; }}"
    );
    al_testkit::run_in(&src, "C", "F")
}

const PREFIX: &str =
    "Unable to convert from Microsoft.Dynamics.Nav.Runtime.NavJsonValue to Microsoft.Dynamics.Nav.Runtime.";

#[test]
fn datetime_read_as_date_or_time_is_a_conversion_error() {
    let dt = "CreateDateTime(DMY2Date(15, 3, 2024), 143015T)";
    assert_eq!(
        al_testkit::error_message(run(dt, "AsDate")),
        format!("{PREFIX}NavDate.")
    );
    assert_eq!(
        al_testkit::error_message(run(dt, "AsTime")),
        format!("{PREFIX}NavTime.")
    );
    assert_eq!(
        al_testkit::error_message(run("0DT", "AsDate")),
        format!("{PREFIX}NavDate.")
    );
}

#[test]
fn date_read_as_time_or_datetime_is_a_conversion_error() {
    let d = "DMY2Date(15, 3, 2024)";
    assert_eq!(al_testkit::error_message(run(d, "AsTime")), format!("{PREFIX}NavTime."));
    assert_eq!(
        al_testkit::error_message(run(d, "AsDateTime")),
        format!("{PREFIX}NavDateTime.")
    );
}

#[test]
fn time_read_as_date_or_datetime_is_a_conversion_error() {
    assert_eq!(
        al_testkit::error_message(run("143015T", "AsDate")),
        format!("{PREFIX}NavDate.")
    );
    assert_eq!(
        al_testkit::error_message(run("0T", "AsDateTime")),
        format!("{PREFIX}NavDateTime.")
    );
}

#[test]
fn time_and_datetime_with_milliseconds_read_as_text() {
    let ms = |v: &str| {
        let src = format!(
            "codeunit 50100 C {{ procedure F(): Text var V: JsonValue; begin V.SetValue({v}); exit(V.AsText()); end; }}"
        );
        al_testkit::text(al_testkit::run_in(&src, "C", "F"))
    };
    assert_eq!(ms("143015T + 123"), "14:30:15.123");
    assert_eq!(
        ms("CreateDateTime(DMY2Date(15, 3, 2024), 143015T + 123)"),
        "2024-03-15T14:30:15.1230000Z"
    );
}

#[test]
fn an_undefined_time_reads_back_as_midnight() {
    // P88DateTimeReadback.al:121-163 Time_Readback `time_blank_astime`: 0T is written `"00:00:00"` and reads back as 000000T.
    assert_eq!(al_testkit::text(run("0T", "AsTime")), "00:00:00");
    assert_eq!(al_testkit::text(run("0D", "AsDate")), "");
}
