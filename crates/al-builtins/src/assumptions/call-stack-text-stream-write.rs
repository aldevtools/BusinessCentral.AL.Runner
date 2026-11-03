CallStackTextStreamWrite => {
    id: "call-stack-text-stream-write",
    assumed: "OutStream.WriteText of the call stack text of a collected error (it holds the runtime's untracked-frame mark) is not refused, on a stream of any encoding, and the text reads back equal from a stream of the same encoding. Its stream bytes are not pinned. Pinned only for OutStream.Write into a Windows and a UTF-8 stream (error-handling/lt-wd/WdCallStackToBlob.al).",
    tests: "No Microsoft test: an outside report (al-runner issue 6853), an app that stores the call stack of collected errors, as JSON text or directly, in a Blob field with WriteText.",
    probe: "A copy of WdCallStackToBlob.al WD_CallStack_Collected_ToWindowsBlob that writes with WriteText and reads with ReadText, once per TextEncoding and once with the default encoding, observing the read-back equality and the stream Length.",
    awaiting: Awaiting {
        issues: &[6853],
        files: &["tests/al-language/error-handling/lt-wd/WdCallStackWriteText.al"],
        since: "2026-11-03",
    },
}
