FeatureKeyEnabledModify => {
    id: "feature-key-enabled-modify",
    assumed: "Modify of a Feature Key with a changed Enabled (the only writable column) is kept for the rest of the run, and a rollback undoes it with the other writes. (Pinned and not assumed: later reads in the same test answer the new value, by Get, FindFirst and a filter on Enabled, and an unchanged Modify keeps the value: record/Y70Probes.al FeatureKey_Modify.)",
    tests: "Tests-ERM Initialize (`FeatureKey.Enabled := FeatureKey.Enabled::None; FeatureKey.Modify()` for ReminderTermsCommunicationTexts): 243 tests.",
    probe: "Change a Feature Key's Enabled and Modify, then in the same test an asserterror that rolls back, and read Enabled; then read it in the next test of the codeunit and after a Commit.",
}
