AppLogoJpegSizes => {
    id: "app-logo-jpeg-sizes",
    assumed: "On majors 27 and 29, the Published Application row of an app (on a row that rests on no other assumption) whose app.json logo is a JPEG with a JFIF header of version 1.01, density units 0, density 1 x 1 and no thumbnail, built as the measured logos (baseline, 3 components, precision 8, of 535 x 534, 16 x 8 or 400 x 300; or progressive, 3 components, precision 8, of 535 x 534, without restart markers), has a Logo with a value and the same media id on every read, and its Media row holds the file's bytes unchanged, Mime Type image/jpeg, Height and Width of the picture, every other column empty, as for the measured 8 x 8 logo. Major 28, other sizes, other JFIF headers and other encodings stay refused.",
    tests: "No Microsoft test. Corpus PR 1594 (tests/al-language-onprem/session-system/UsAppLogoJpegSizes.al, UsLogo_Jpeg535x534_AppG, UsLogo_Jpeg16x8_AppH, UsLogo_Jpeg400x300_AppJ, UsLogo_JpegProgressive535x534_AppK): green on the Windows containers of majors 27 (minor 5) and 29 (minor 0), Mime Type image/jpeg on both. Corpus CI runs BC on Linux, which gave image/png on majors 27 and 28 (a Linux suspect, the probe ignores the Mime Type), so major 28 is not measured on Windows.",
    probe: "Corpus PR 1594: the four logos above on majors 27, 28 and 29; the Mime Type on Windows major 28; and logos with another JFIF header (units 1, 96 dpi as System.Drawing writes it), other sizes, and other encodings.",
    awaiting: Awaiting {
        issues: &[9111],
        files: &["tests/al-language-onprem/session-system/UsAppLogoJpegSizes.al"],
        since: "2026-11-28",
    },
}
