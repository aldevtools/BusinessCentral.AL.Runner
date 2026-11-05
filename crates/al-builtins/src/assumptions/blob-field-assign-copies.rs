BlobFieldAssignCopies => {
    id: "blob-field-assign-copies",
    assumed: "`Target.Field := Source.Blob` (or a Variant that holds a Blob) copies the value: a later Clear of the source, or a new stream written to it, does not change the target, and the reverse. A record assignment copies its Blobs (record/PnABlobAssignRename.al:35-66 Assign_Temporary); a field assignment followed by a change of either side is not pinned.",
    tests: "No Microsoft test: an outside report (al-runner issue 7128), a test of a closed-source suite that reuses one buffer record per imported node, Clears its Blob when a node is missing and passes on BC 29 in the suite's CI.",
    probe: "Field := field of a Blob, then on the source: Clear, a new CreateOutStream write, a Modify; on the target: read back, also after Insert + Get + CalcFields; the same from a Variant and into a temporary record.",
    awaiting: Awaiting {
        issues: &[7128],
        files: &["tests/al-language/record/lt-blob-field-assign/BlobFieldAssignCopy.al"],
        since: "2026-11-05",
    },
}
