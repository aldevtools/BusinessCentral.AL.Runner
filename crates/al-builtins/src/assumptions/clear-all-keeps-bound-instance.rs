ClearAllKeepsBoundInstance => {
    id: "clear-all-keeps-bound-instance",
    assumed: "ClearAll() in a codeunit keeps a global codeunit variable that holds an instance bound by BindSubscription pointing at that instance, and the binding stays; the instance's own globals are reset to their defaults (its record of a TableNo stays). An array of codeunits, a page extension's global, and a bound instance whose globals hold a temporary record or another bound instance are refused.",
    tests: "Measured on real BC 29.0 (observe run of corpus branch al-runtime-rs-staging-6974): events/binding-senders/BindSubscriptionClearAll.al ClearAll_HolderWithBoundGlobal_Binding, ClearAll_TestCodeunitWithBoundGlobal_Binding; al-runner issue 6974 (the reporter's tests pass on BC 29). BC 27.5 and 28.5 not yet run.",
    probe: "events/binding-senders/BindSubscriptionClearAll.al on BC 27.5, 28.5 and 29.0 (corpus branch al-runtime-rs-staging-6974); also wanted: a bound instance in an array of codeunits and a codeunit with a TableNo as the bound instance.",
    awaiting: Awaiting {
        issues: &[6974],
        files: &["tests/al-language/events/binding-senders/BindSubscriptionClearAll.al"],
        since: "2026-11-08",
    },
}
