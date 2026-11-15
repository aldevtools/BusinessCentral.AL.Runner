PublishedApplicationJpegLogo => {
    id: "published-application-jpeg-logo",
    assumed: "The Published Application Logo of an app whose manifest declares a baseline JPEG logo (start-of-frame marker FF C0) is a media with a value and the same id on every read, whose Media row holds the file's bytes unchanged, the picture's Height and Width, Mime Type `image/png` on the majors before and `image/jpeg` on the majors of the al-version gate ProbedValuesDifferFrom29, and every other column empty (Description, Company Name, Expiration Date, File Name, Security Token, Creating User; Prohibit Cache No), as for a PNG logo. Measured for one 8 x 8 baseline JFIF JPEG of 307 bytes; assumed alike for other sizes of a baseline JPEG. Another JPEG kind (progressive or other start-of-frame markers) and every other picture format stay refused.",
    tests: "Corpus PR 1412 (merged, not pinned), al-language-onprem/session-system/UsAppLogoJpeg.al:82-120 UsLogo_Jpeg_AppE (cross-app fixture E, tests/al-language-xapp-e/logo.jpg), measured on the three majors of its CI run; the login's registration of fixture E's assisted setup with that logo as its icon ran without an error (:122-135). Outside report: al-runner issue 8061.",
    probe: "Corpus PR 1412 (merged, not pinned): UsAppLogoJpeg.al. Wanted next: a larger and a progressive JPEG logo.",
    awaiting: Awaiting {
        issues: &[8061],
        files: &["tests/al-language-onprem/session-system/UsAppLogoJpeg.al"],
        since: "2026-11-15",
    },
}
