JsonTextNumberShapes => {
    id: "json-text-number-shapes",
    assumed: "JsonValue.AsDecimal of a JSON string: a comma in the whole part is dropped (`4,5` reads 45, `1,000.5` 1000.5), `+3`, `.5` and blanks around a number with a point read as the number; a text with a sign outside the number alphabet (`n/a`, `NaN`, `0x10`), blanks between digits or two points raises the conversion error; a number beyond the Decimal range raises the range error. The other getters raise their conversion error for such a text. Measured on SaaS (newest major) by corpus writer Y65; not on the pinned corpus yet.",
    tests: "No Microsoft test: al-runner issue 6676 (AsDecimal of `n/a` under asserterror).",
    probe: "json/JsonValueGetters.al Y65_NotANumberText_EveryGetter, Y65_DecimalShapedText_AsDecimal, Y65_DecimalShapedText_AsDecimal_2, Y65_Reporter_AsDecimalOfTokenValue (corpus PR 1012).",
    awaiting: Awaiting {
        issues: &[6676],
        files: &["tests/al-language/json/JsonValueGetters.al::Y65_NotANumberText_EveryGetter"],
        since: "2026-11-01",
    },
}
