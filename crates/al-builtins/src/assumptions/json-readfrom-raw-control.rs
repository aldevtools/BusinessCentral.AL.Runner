JsonReadfromRawControl => {
    id: "json-readfrom-raw-control",
    assumed: "JsonObject, JsonToken, JsonArray and JsonValue ReadFrom, of a Text and of an InStream, keep every raw control character from 1 to 31 inside a JSON string as part of it: in a value and in a key, in double and in single quotes. The codes, places and readers json/JsonReadFrom.al pins run clean; code 0 and the other readers (Evaluate, the runtime's own reads) are refused.",
    tests: "No Microsoft test: al-runner issue 7812 (the reporter's run on a BC 28.4 server: JsonObject.ReadFrom(Text) returns true for every code 0 to 31 in a double-quoted value; the Sales Lines Suggestions test app's install routine reads a dataset line holding U+0015).",
    probe: "PROBE_FILE (corpus pull request PROBE_PR): every code 0 to 31 in a value, a key and a single-quoted string, for the four types, from a Text and from an InStream.",
    awaiting: Awaiting {
        issues: &[7812],
        files: &["PROBE_FILE"],
        since: "2026-11-25",
    },
}
