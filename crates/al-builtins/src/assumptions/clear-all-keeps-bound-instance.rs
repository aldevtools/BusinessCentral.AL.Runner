ClearAllKeepsBoundInstance => {
    id: "clear-all-keeps-bound-instance",
    assumed: "ClearAll() in a codeunit keeps a global codeunit variable that holds an instance bound by BindSubscription pointing at that instance, and the binding stays; the instance's own Text globals are reset. Served only when every global of the bound instance is Text and it has no TableNo record; an array of codeunits, a page extension's global, a TableNo record or a global of any other kind is refused.",
    tests: "Measured on real BC, newest major only (observe run of corpus branch al-runtime-rs-staging-6974): events/binding-senders/BindSubscriptionClearAll.al ClearAll_HolderWithBoundGlobal_Binding, ClearAll_TestCodeunitWithBoundGlobal_Binding; al-runner issue 6974. The older majors are not yet run.",
    probe: "events/binding-senders/BindSubscriptionClearAll.al on every supported major (corpus branch al-runtime-rs-staging-6974); also wanted: a bound instance in an array of codeunits, a bound instance with a TableNo record, and bound instances with globals of other kinds (records, temporary records, codeunits, numbers).",
    awaiting: Awaiting {
        issues: &[6974],
        files: &["tests/al-language/events/binding-senders/BindSubscriptionClearAll.al"],
        since: "2026-11-08",
    },
}
