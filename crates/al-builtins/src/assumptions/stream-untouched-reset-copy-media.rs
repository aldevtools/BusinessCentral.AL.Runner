StreamUntouchedResetCopyMedia => {
    id: "stream-untouched-reset-copy-media",
    assumed: "On streams never created (not cleared): InStream.ResetPosition() as a statement raises \"Microsoft.Dynamics.Nav.Runtime.NavInStream variable not initialized.\" (NotInitialized); CopyStream(OutStream, InStream) with both never created and no byte count raises the same error as a statement and answers false with the result used; Media.ImportStream(InS, 'icon'), Media.ImportStream(InS, 'icon', 'image/png') and MediaSet.ImportStream(InS, 'icon') with the result used, into a field that holds no media, answer the empty Guid without an error and leave the field without a media (the probe did not read the field afterwards; that it holds no media is assumed too). The record that holds the field is not checked (the field value does not carry it; the probe used a record not inserted). Other arguments, a field that holds a media and the statement forms keep their answer before this assumption.",
    tests: "Outside report: al-runner issue 7760 (uncreated streams). Measured on BC 27.5, 28.5 and 29.0 by corpus PR 1408 (merged, not yet pinned).",
    probe: "Corpus PR 1408: tests/al-language/streams/instream-outstream/UhUntouchedStreams.al (codeunit 66994).",
    awaiting: Awaiting {
        issues: &[7760],
        files: &["tests/al-language/streams/instream-outstream/UhUntouchedStreams.al"],
        since: "2026-11-15",
    },
}
