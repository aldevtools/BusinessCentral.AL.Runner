BlobStoredSizePositive => {
    id: "blob-stored-size-positive",
    assumed: "The stored size `Blob.Length` gives before CalcFields is positive for every non-empty stored content whenever HasValue is Yes (any size or shape, Compressed true or false). Used only for a comparison of `Length` with the literal 0, which observes its sign; the number itself stays refused for contents whose size is not pinned.",
    tests: "No Microsoft test: an outside report, EDI import code that checks `File Content.Length = 0` before CalcFields on XML files of 600 to 3 800 bytes in a Compressed Blob field. Pinned sizes: record/PnABlobLength.al:39-60 (all positive for 1 to 100000 bytes), streams/blob-tempblob/BlobLengthInteger.al:95.",
    probe: "record/flowfields/BlobStoredSizeSign.al (in the pin): the sign is pinned for a Compressed field, the exact size only for Compressed = false; the stored size of a Compressed field as a number is host-dependent and stays refused.",
}
