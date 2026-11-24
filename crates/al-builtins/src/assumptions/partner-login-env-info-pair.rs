PartnerLoginEnvInfoPair => {
    id: "partner-login-env-info-pair",
    assumed: "In the test session (a client, Codeunit isolation, the login not raised through Microsoft's code), codeunit 3702 Environment Information Impl. of a dependency app published by Microsoft, changed by exactly two \"System Initialization\".OnAfterLogin subscribers of the same partner dependency app, keeps what both left, for every later read (al-testrun `login_memo.rs` `env_info_pair`, called by `partner_login.rs` `kept`): the first in the run's subscriber order changed exactly IsSandboxConfig and IsSandboxInitialized, the second exactly IsSaaSConfig and IsSaasInitialized, both outside any Global Triggers raise. A later test reads from IsSandbox() and IsSaaS() what each subscriber got. Every read of one of its globals marks this entry. Not covered, every read refused: any other split of the globals, the reverse order, a change inside a Global Triggers raise, a third subscriber.",
    tests: "No Microsoft test. Outside report (al-runner issue 8945): a partner suite of about 5,500 tests that is not run at all.",
    probe: "Corpus PR 1571 (aldevtools/BusinessCentral.AL.Language.Tests, open), tests/al-language/events/platform-triggers/LmLoginCacheTests.al and tests/al-language-testlibraries/test-framework/login-caches/LmLoginCacheTlTests.al EnvInfo_AfterLogin (cell E1) with the two login subscribers of tests/al-language-xapp-a/LmXaLoginCaches.al in ascending id order (IsSandbox(), then IsSaaS()); measured on a local BC of the newest supported major, in the shape of the job with Microsoft's test libraries, both kinds of session alike: the test reads the values each subscriber got. The probe measures only that equality: whether its subscribers are the first to fill the memo is not controlled. Still to measure: the two older majors, the shape without Microsoft's test libraries.",
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
