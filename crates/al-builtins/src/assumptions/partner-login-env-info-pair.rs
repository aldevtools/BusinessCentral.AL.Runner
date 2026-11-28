PartnerLoginEnvInfoPair => {
    id: "partner-login-env-info-pair",
    assumed: "In the test session (a client, Codeunit isolation, the login not raised through Microsoft's code), codeunit 3702 Environment Information Impl. of a dependency app published by Microsoft, changed by exactly two \"System Initialization\".OnAfterLogin subscribers of the same partner dependency app, keeps what both left, for every later read (al-testrun `login_memo.rs` `env_info_pair`, called by `partner_login.rs` `kept`): the first in the run's subscriber order changed exactly IsSandboxConfig and IsSandboxInitialized, the second exactly IsSaaSConfig and IsSaasInitialized, both outside any Global Triggers raise only (al-vm `raise_watch.rs`: each of their changed globals noted under no raise and under nothing else). A later test reads from IsSandbox() and IsSaaS() what each subscriber got. Every read of one of its globals marks this entry. Not covered, every read refused: any other split of the globals, the reverse order, a change inside a Global Triggers raise, a third subscriber.",
    tests: "No Microsoft test. Outside report (al-runner issue 8945): a partner suite of about 5,500 tests that is not run at all.",
    probe: "Corpus PR 1571 (aldevtools/BusinessCentral.AL.Language.Tests, merged 2026-11-24, not yet in the pinned corpus), tests/al-language/events/platform-triggers/LmLoginCacheTests.al and tests/al-language-testlibraries/test-framework/login-caches/LmLoginCacheTlTests.al EnvInfo_AfterLogin (cell E1) with the two login subscribers of tests/al-language-xapp-a/LmXaLoginCaches.al in ascending id order (IsSandbox(), then IsSaaS()): the test reads the values each subscriber got. Measured first on a local BC of the newest supported major; corpus CI at corpus commit 75a209af (workflow run 45829421054, the oracle of the next pin bump) passes it on BC 27, 28 and 29 in both jobs. The probe measures only that equality: whether its subscribers are the first to fill the memo is not controlled. The next corpus pin bump pins it and deletes this entry.",
    awaiting: Awaiting {
        issues: &[8945],
        files: &[
            "tests/al-language/events/platform-triggers/LmLoginCacheTests.al",
            "tests/al-language-testlibraries/test-framework/login-caches/LmLoginCacheTlTests.al",
            "tests/al-language-xapp-a/LmXaLoginCaches.al",
        ],
        since: "2026-11-24",
    },
}
