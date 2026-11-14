GlobalFilterEndsWithInstallSession => {
    id: "global-filter-ends-with-install-session",
    assumed: "A filter that a Record variable sets with SetFilter or SetRange in filter group 1 (the global group), in the per-company part of an app's install session in the session company, in the install codeunit's OnInstallAppPerCompany trigger or in a \"System Initialization\".OnAfterLogin subscriber that the session's Company Triggers raise, is gone when the next app's install session begins (a dependency app's or an app under test's) and when the test session follows the install sessions. An OnAfterLogin subscriber sets it again in each session, as main runs it. A group 1 filter set any other way (through a FieldRef or RecordRef, by a Company Triggers or \"Extension Triggers\" subscriber, in the per-database part, in another company, by Company-Initialize or a company build step) stays as main keeps it. Code that then does what main refused on the table relies on this, and so does a test that touches a table install code wrote after the filter ended.",
    tests: "No Microsoft test. Outside report: al-runner issue 8071 (the reporter's repro: an OnAfterLogin subscriber of one app under test sets a group 1 filter; the next app's install session refused it).",
    probe: "Written: corpus PR 1405, tests/al-language/install-upgrade/G1sInstallSessionTests.al:52-82 and :84-102 with tests/al-language-xapp-a/G1sXaInstall.al:88-119 and tests/al-language-xapp-b/G1sXbInstall.al:14-27, measured alike on all three supported BC majors. Not measured: a filter set through a FieldRef or a RecordRef, by a Company Triggers or \"Extension Triggers\" subscriber, in the per-database part, in another company under --companies, by Company-Initialize or a company build step, and what the next session sees after those.",
    awaiting: Awaiting {
        issues: &[8071],
        files: &["tests/al-language/install-upgrade/G1sInstallSessionTests.al"],
        since: "2026-11-14",
    },
}
