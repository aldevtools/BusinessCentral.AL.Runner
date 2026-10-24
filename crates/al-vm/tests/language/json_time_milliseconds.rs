//! The JSON text of a Time with milliseconds is `Format(T, 0, 9)`: trailing zeros of the milliseconds are dropped, leading
//! zeros kept (al-runner issue 5794; corpus misc/lt-outside-format-json-list/RwJsonTimeMs.al:116 RW_JsonTime_InMemory_
//! Milliseconds, :62 RW_JsonTime_StoredField_Reporter).

use al_testkit::{run_in, text};

fn json_of(ms: i32) -> String {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var T: Time; O: JsonObject; A: JsonArray; V: JsonValue; Txt: Text; Tok: JsonToken; begin \
        T := 100000T + {ms}; V.SetValue(T); V.WriteTo(Txt); O.Add('t', T); A.Add(O); \
        O.ReadFrom(Format(O)); O.Get('t', Tok); \
        exit(Txt + '|' + Format(A) + '|' + Format(Tok.AsValue().AsTime() = T)); end; }}"
    );
    text(run_in(&src, "C", "F"))
}

#[test]
fn trailing_zeros_of_the_milliseconds_are_dropped() {
    for (ms, t) in [
        (1, "10:00:00.001"),
        (10, "10:00:00.01"),
        (100, "10:00:00.1"),
        (120, "10:00:00.12"),
        (123, "10:00:00.123"),
        (500, "10:00:00.5"),
        (990, "10:00:00.99"),
    ] {
        assert_eq!(json_of(ms), format!("\"{t}\"|[{{\"t\":\"{t}\"}}]|Yes"), "{ms}");
    }
}
