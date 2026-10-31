FeatureKeyEnabledModify => {
    id: "feature-key-enabled-modify",
    assumed: "Modify of a Feature Key with a changed Enabled (the only writable column) is kept for the rest of the run: later reads answer the new value, a rollback undoes it with the other writes.",
    tests: "Tests-ERM Initialize (`FeatureKey.Enabled := FeatureKey.Enabled::None; FeatureKey.Modify()` for ReminderTermsCommunicationTexts): 243 tests.",
    probe: "Get a Feature Key, change Enabled, Modify; Get again and read Enabled; FindSet and Count; the same inside a test with TestIsolation, then in the next test. Asked in corpus #648.",
}
