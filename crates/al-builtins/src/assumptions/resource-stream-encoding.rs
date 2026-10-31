ResourceStreamEncoding => {
    id: "resource-stream-encoding",
    assumed: "NavApp.GetResource(Name, InStream, Encoding) with UTF16, Windows or MSDos gives an InStream that reads the resource's bytes as a stream created with that encoding does (UTF16: little endian, a leading byte order mark skipped), and without an encoding as MSDos. Measured on the newest BC major by the corpus writer (session/Y18NavAppResourceEncodings.al, merged on corpus master), not yet in the pinned corpus. UTF8 is pinned (session/TestNavAppGetResource.al:33).",
    tests: "No Microsoft test: al-runner issue 6492, an ISV test library that reads two UTF-16LE XML configuration resources with a byte order mark (about 100 tests).",
    probe: "Written: session/Y18NavAppResourceEncodings.al (ReadText, Read(Text) and CopyStream of UTF-16LE, UTF-8 and Windows-1252 resources with every encoding).",
    awaiting: Awaiting {
        issues: &[6492],
        files: &["tests/al-language/session/Y18NavAppResourceEncodings.al"],
        since: "2026-10-31",
    },
}
