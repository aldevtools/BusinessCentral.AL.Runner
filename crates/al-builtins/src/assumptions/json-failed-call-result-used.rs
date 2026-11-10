JsonFailedCallResultUsed => {
    id: "json-failed-call-result-used",
    assumed: "A Json method that fails with its return value used (assigned, in a condition, returned with exit, passed as an argument) answers false, raises nothing, leaves the last error and the JSON as they were: JsonObject.Add with a key that exists, and JsonArray.Insert, Set and RemoveAt with an index outside the array. Insert and Set that succeed with the result used answer true. Pinned are the statement forms, which raise (json/JsonObjectBasics.al:35-55 DuplicateJsonProperty; json/JsonArrayBasics.al:96-99, :155-158, :186-195 IndexOutOfBounds), and the used forms of Get, Replace, SelectToken and ReadFrom, which answer false.",
    tests: "No Microsoft test. A user's report: `exit(JsonObj.Add(Key, Value))` with a key that exists returns false on real BC (al-runner issue 6924).",
    probe: "json/json/JsonObjectUsedReturn.al and JsonArrayTokenUsedReturn.al (corpus pull request 1298): each failing call as a statement, assigned, in a condition, returned and as an argument, with the return value, GetLastErrorText, GetLastErrorCode and the JSON text afterwards; the return value of Insert, Set and RemoveAt that succeed.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &[
            "tests/al-language/json/json/JsonObjectUsedReturn.al",
            "tests/al-language/json/json/JsonArrayTokenUsedReturn.al",
        ],
        since: "2026-11-10",
    },
}
