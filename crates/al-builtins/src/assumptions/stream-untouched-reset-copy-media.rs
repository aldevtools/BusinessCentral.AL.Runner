StreamUntouchedResetCopyMedia => {
    id: "stream-untouched-reset-copy-media",
    assumed: "On streams never created (not cleared): InStream.ResetPosition() as a statement raises \"Microsoft.Dynamics.Nav.Runtime.NavInStream variable not initialized.\" (NotInitialized); CopyStream(OutStream, InStream) with both never created and no byte count raises the same error as a statement and answers false with the result used; Media.ImportStream(InS, 'icon'), Media.ImportStream(InS, 'icon', 'image/png') and MediaSet.ImportStream(InS, 'icon') with the result used, into a field that holds no media of a record of a stored table that no Insert, Get or Find reached (the probe's record, passed from the call site), answer the empty Guid without an error. The record then refuses every read of its buffer, write of it and copy of it until a read fills it again: the probe did not read it after the import. Other arguments, a field that holds a media, an inserted or temporary record and the statement forms keep their answer before this assumption.",
    tests: "Outside report: al-runner issue 7760 (uncreated streams). Measured on every supported major by corpus PR 1408 (merged, not yet pinned).",
    probe: "Corpus PR 1408: tests/al-language/streams/instream-outstream/UhUntouchedStreams.al (codeunit 66994). What the record holds after the import (HasValue, MediaId, MediaSet Count, Insert then Get, an inserted and a temporary record): observe-form draft, codeunit 66995 in streams/instream-outstream/UhUntouchedStreamsAfter.al, not yet opened.",
    awaiting: Awaiting {
        issues: &[7760],
        files: &["tests/al-language/streams/instream-outstream/UhUntouchedStreams.al"],
        since: "2026-11-15",
    },
}
