ReadFromYamlFinalShapes => {
    id: "read-from-yaml-final-shapes",
    assumed: "JsonObject.ReadFromYaml (Text and InStream) of a text with a tag or a directive (a `!` or `%` in it) reads as the cells of corpus PR 1549 (JsonYamlFinalTags.al, JsonYamlFinalStructure.al) measured on BC 27.5, 28.5 and 29.0 (al-builtins `json/yaml/read/tags.rs`): percent escapes in a tag suffix, a verbatim tag and a %TAG prefix are decoded as UTF-8 before the tag resolves, and a bad escape is the YAML error; %TAG for `!`, `!!` or a named handle with any prefix, the same handle twice the YAML error; %YAML 1.0 to 1.3, a second one, 2.0, 1, 1.10 and none the YAML error; !!int as a decimal, 0x or 0b number up to 2147483647 (a leading zero octal under %YAML 1.1); !!float in .NET's shortest form with `.0`, `E+XX` from 1E+17 and below 1E-04, Infinity and NaN as texts; !!bool yes/no/on/off/y/n/true/false in lower, capitalised and upper case; !!timestamp only the five measured texts; !!null, !!binary, !!set, !!omap, !!pairs, !!merge, !!value, !!yaml, unknown core tags and local tags the YAML error; an empty or null value under !!str, !!int, !!bool, !!float and !!map in the measured spellings null; only !!map on a mapping, every other tag on a collection the YAML error; a tagged key written in .NET's text form (True, False, 7, 1.5, Infinity) and a !!map key as (Collection); a tagged !!int, !!bool or !!str document answers InvalidJsonType. Every other shape refuses: an underscore outside digit groups, `+0x`, a signed `0b`, an upper-case prefix, a leading zero that reads differently as octal, a negative number past -2147483647 or larger hex, a sign on .nan, -1e400, an underflow, a !!float whose exponent is -4 to -2 or 6 to 16, mixed case Boolean words, a quoted null word under !!str, a block scalar under a tag other than !!str, a null word under another kind, any tagged number, Boolean or timestamp under %YAML other than 1.1 and !!float or !!timestamp under 1.1, a collection under an unknown core or non-core global tag, two %TAG directives for different handles, another %YAML version, a tagged top scalar of another kind or under %YAML.",
    tests: "No Microsoft test. al-runner issues 8580 and 8856. Corpus PR 1549 (branch probe-yaml-final-8580, head 84db3dad, CI green on BC 27.5, 28.5 and 29.0), codeunits 79896 and 79897, observed and asserted on local Business Central of each major the corpus runs.",
    probe: "tests/al-language/json/JsonYamlFinalTags.al and JsonYamlFinalStructure.al of corpus PR 1549 (open, not yet pinned).",
    awaiting: Awaiting {
        issues: &[8856],
        files: &[
            "tests/al-language/json/JsonYamlFinalTags.al",
            "tests/al-language/json/JsonYamlFinalStructure.al",
        ],
        since: "2026-11-23",
    },
}
