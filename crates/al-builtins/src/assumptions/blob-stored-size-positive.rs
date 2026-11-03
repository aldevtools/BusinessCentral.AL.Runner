BlobStoredSizePositive => {
    id: "blob-stored-size-positive",
    assumed: "The stored size `Blob.Length` gives before CalcFields is positive for every non-empty stored content whenever HasValue is Yes (any size or shape, Compressed true or false). Used only for a comparison of `Length` with the literal 0, which observes its sign; the number itself stays refused for contents whose size is not pinned.",
    tests: "No Microsoft test: an outside report (al-runner issue 6896), EDI import code that checks `File Content.Length = 0` before CalcFields on XML files of 600 to 3 800 bytes in a Compressed Blob field. Pinned sizes: misc/lt-probes-needed-1/PnABlobLength.al:39-60 (all positive for 1 to 100000 bytes), streams/lt-blob-length-integer/BlobLengthInteger.al:95.",
    probe: "record/lt-blob-calcfields-length/BlobStoredSizeSign.al on corpus branch al-runtime-rs-staging-r6896 (contents of 1 byte to 5 MB, Compressed true and false, Get and FindSet, Modify, Clear).",
    awaiting: Awaiting {
        issues: &[6896],
        files: &["tests/al-language/record/lt-blob-calcfields-length/BlobStoredSizeSign.al"],
        since: "2026-11-03",
    },
}
