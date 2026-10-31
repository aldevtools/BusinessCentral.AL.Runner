IsolatedStorageOverwriteTx => {
    id: "isolated-storage-overwrite-tx",
    assumed: "IsolatedStorage.SetEncrypted over a key that already exists, or Set over a key stored encrypted, outside a write transaction, opens the write transaction (Database.IsInWriteTransaction is true after it), as a plain Set over a plain key does (pinned: session/lt-cn-session-leads/CnIsolatedStorageLeads.al TO_IsoOverwrite_OutsideWriteTransaction).",
    tests: "No Microsoft test: a partner app's EFT tests overwrite a stored key with SetEncrypted after a Commit.",
    probe: "The SetEncrypted forms of TO_IsoOverwrite_OutsideWriteTransaction (left out of the corpus: the Linux CI containers have no encryption key), run on a Windows BC.",
}
