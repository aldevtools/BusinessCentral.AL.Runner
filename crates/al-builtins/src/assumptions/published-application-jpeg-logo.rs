PublishedApplicationJpegLogo => {
    id: "published-application-jpeg-logo",
    assumed: "The Published Application Logo of an app whose manifest declares a JPEG logo built as the measured one (SOI, an APP0 JFIF segment first, then only quantization tables, one baseline start-of-frame segment FF C0 of precision 8 and 3 components, Huffman tables, a start-of-scan segment of 3 components, at least one byte of coded data with no marker in it, then FF D9 as the last two bytes; width and height at least 1), on a row that rests on no other assumption, is a media with a value and the same id on every read, whose Media row holds the file's bytes unchanged, the picture's Height and Width, Mime Type `image/png` on the majors before and `image/jpeg` on the majors of the al-version gate ProbedValuesDifferFrom29, and every other column empty (Description, Company Name, Expiration Date, File Name, Security Token, Creating User; Prohibit Cache No), as for a PNG logo. Measured for one 8 x 8 baseline JFIF JPEG of 307 bytes; Height and Width are read from the file's frame for other sizes. Every other file structure (progressive or other start-of-frame markers, other component counts or precision, other segments such as EXIF, no coded data, restart or other markers in the coded data, bytes after FF D9, a width or height of 0) and every other picture format stay refused. A scan cut short that still ends in FF D9 is not told apart without decoding and is assumed alike.",
    tests: "Corpus PR 1412 (merged, not pinned), al-language-onprem/session-system/UsAppLogoJpeg.al:82-120 UsLogo_Jpeg_AppE (cross-app fixture E, tests/al-language-xapp-e/logo.jpg), measured on the three majors of its CI run; the login's registration of fixture E's assisted setup with that logo as its icon ran without an error (:122-135). Outside report: al-runner issue 8061.",
    probe: "Corpus PR 1412 (merged, not pinned): UsAppLogoJpeg.al. Wanted next: a larger non-square JPEG logo, a progressive JPEG logo.",
    awaiting: Awaiting {
        issues: &[8061],
        files: &["tests/al-language-onprem/session-system/UsAppLogoJpeg.al"],
        since: "2026-11-15",
    },
}
