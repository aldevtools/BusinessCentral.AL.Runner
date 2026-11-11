JsonRawTabInString => {
    id: "json-raw-tab-in-string",
    assumed: "JsonObject, JsonToken, JsonArray and JsonValue ReadFrom read a raw TAB (character 9, not escaped) inside a JSON string as part of the string, and return true. The other raw control characters inside a string still make ReadFrom return false here.",
    tests: "Microsoft's Sales Lines Suggestions test app: its install codeunit (SLS Test App Install) imports a JSONL dataset through Test Inputs Management, which calls JsonToken.ReadFrom per line; some lines hold a raw TAB in a string. al-runner issue 7812 (reporter's repro).",
    probe: "json/JsonReadFrom.al ObjectReadFromRawControlInString, TokenReadFromRawControlInString, ArrayReadFromRawControlInString, ValueReadFromRawControlInString (corpus pull request 1327).",
    awaiting: Awaiting {
        issues: &[7812],
        files: &[
            "tests/al-language/json/JsonReadFrom.al::ObjectReadFromRawControlInString",
            "tests/al-language/json/JsonReadFrom.al::TokenReadFromRawControlInString",
        ],
        since: "2026-11-12",
    },
}
