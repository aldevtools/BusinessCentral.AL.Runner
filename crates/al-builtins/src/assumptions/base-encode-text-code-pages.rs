BaseEncodeTextCodePages => {
    id: "base-encode-text-code-pages",
    assumed: "ToBase64 of a Text with an encoding and a code page (`Base64Convert.ToBase64(Text, Boolean, TextEncoding, Integer)`, direct, or through System Application's \"Base64 Convert\" on the majors whose System Application calls it; on the oldest major the corpus runs it converts through DotNet and does not reach this entry), in the pairs corpus PR 1414 measured: TextEncoding::UTF8 with code page 1252 or 28591 writes the UTF-8 bytes; MSDos with 1252 writes Windows-1252 and with 28591 Latin-1; Windows or MSDos with 65001 writes UTF-8. Measured with the text 'äöü', and for UTF8 with 1252 also 'ÿÿ' and the euro sign. The single-byte table is the one Windows with 1252 and 28591 already uses, which still refuses a character the two tables encode differently. Every other pair of encoding and code page (437, 850, 1200, UTF16 with a code page, ...) is refused.",
    tests: "Corpus PR 1414 (merged, not pinned yet): tests/al-language/types/PVBase64ToCodePages.al, codeunit 99984, the same text on every major of its CI run. Outside report: al-runner issue 8069 (ToBase64(Text, false, UTF8, 1252) of 'ÿÿ').",
    probe: "Written: corpus PR 1414, tests/al-language/types/PVBase64ToCodePages.al:38-63. Not measured yet: ToBase64 with code pages 437, 850 or 1200, UTF16 with a code page, and other characters through MSDos with 1252 or 28591.",
    awaiting: Awaiting {
        issues: &[8069],
        files: &["tests/al-language/types/PVBase64ToCodePages.al"],
        since: "2026-11-15",
    },
}
