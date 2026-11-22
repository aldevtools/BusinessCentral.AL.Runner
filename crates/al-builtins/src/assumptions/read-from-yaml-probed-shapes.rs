ReadFromYamlProbedShapes => {
    id: "read-from-yaml-probed-shapes",
    assumed: "JsonObject.ReadFromYaml (Text and InStream) of a text other than the 24 texts the pinned corpus reads, and any read whose result is used or whose source is a stream, reads as the cells of the corpus probes of al-runner issue 8580 measured (al-builtins `json/yaml/read`), identical on BC 27.5, 28.5 and 29.0: corpus PR 1493 (JsonYamlBlankLines.al, blank lines, CRLF, block lists, block scalars, comments), PR 1515 (JsonYamlUtf8.al) and PR 1519 (JsonYamlPunctMatrix.al) with the characters U+00A7, U+00E9, U+00FC, U+00F8, U+20AC, U+1F600, U+0301, U+00A0, U+0080 and U+FEFF, and PR 1530 (JsonYamlRemaining.al, open) with a libyaml-style reading of block and flow mappings and lists, plain, single- and double-quoted scalars over several lines, the double-quote escapes, the block scalars `|` and `>` with `-`, `+` and an indentation indicator, comments, anchors and aliases, the tags `!!str`, `!!int`, `!!float`, `!!bool`, `!!map` and `!!timestamp` of a date, complex keys, a duplicate key (the last value), `---`, `...` and the `%YAML` and `%TAG` directives of one document, tabs as white space inside a line, a byte order mark (kept in a Text, dropped from an InStream), CRLF and mixed line ends. Scalars read as text and `null`, `Null`, `NULL`, `~` and an empty value as null. A YAML error answers false with `The data does not represent valid YAML.` as the last error (result used, empty object) or raises it as a statement; a document that is no mapping answers false with the InvalidJsonType error; a mapping of 101 to 301 containers (result used) answers InvalidJsonData, up to 51 containers it reads. Every other shape refuses: a character outside the list above, U+0085, U+2028 or U+2029 in a text that reads, a lone surrogate escape, a tab after indentation spaces, a tagged top scalar, a `!!int`, `!!float`, `!!bool` or `!!timestamp` text other than the measured forms, a core tag on the other kind of node, nesting of 52 to 100 or more than 301 containers, nesting too deep as a statement, a YAML error into an object with members, non-UTF-8 stream bytes.",
    tests: "No Microsoft test. al-runner issue 8580 (the reporter's AI eval datasets, .yaml app resources, import through ReadFromYaml and pass on real Business Central). Corpus PRs 1493, 1515 and 1519 (merged on corpus master, not yet pinned) and PR 1530 (open, head 4e08c074), codeunits 75620, 75621, 75623 and 79892, 140 tests observed and asserted on local Business Central 27.5, 28.5 and 29.0.",
    probe: "tests/al-language/json/JsonYamlBlankLines.al, JsonYamlUtf8.al, JsonYamlPunctMatrix.al and JsonYamlRemaining.al of corpus PRs 1493, 1515, 1519 and 1530 (not yet pinned).",
    awaiting: Awaiting {
        issues: &[8580],
        files: &[
            "tests/al-language/json/JsonYamlBlankLines.al",
            "tests/al-language/json/JsonYamlUtf8.al",
            "tests/al-language/json/JsonYamlPunctMatrix.al",
            "tests/al-language/json/JsonYamlRemaining.al",
        ],
        since: "2026-11-22",
    },
}
