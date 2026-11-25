PartnerLoginMicrosoftCache => {
    id: "partner-login-microsoft-cache",
    assumed: "In the test session (a client, Codeunit isolation, the login not raised through Microsoft's code), a cache codeunit of a dependency app published by Microsoft that one \"System Initialization\".OnAfterLogin subscriber of a partner dependency app changed, and no other login subscriber changed, keeps what that subscriber left, for every later read (al-testrun `login_memo.rs` `microsoft_cache_measured`, called by `partner_login.rs` `kept`). Covered, by codeunit id and exactly these changed globals: Change Log Management 423, ChangeLogSetupRead and MonitorSensitiveFieldSetupRead, both changed inside a raise of the Global Triggers event OnDatabaseInsert only (so its setup Get found no row and its temporary records hold none); API Webhook Notification Mgt. 6153, CachedApiSubscriptionEnabled and UseCachedApiSubscriptionEnabled, both inside a raise of OnDatabaseInsert only; Conf./Personalization Mgt. 9170, CurrentAllProfile, CurrentAllProfileFetched and ProfileFound, outside any Global Triggers raise; Application Area Cache 9177, ApplicationAreaCache, outside any Global Triggers raise. Every read of one of its globals marks this entry. Not covered, every read refused: another Microsoft codeunit, another set of changed globals, a global changed inside another raise as well, a global of an outside-any-raise row changed inside a raise, a codeunit a second login subscriber changed. The gate compares the set of raises that changed each global: a global of an OnDatabaseInsert row that also changed outside any raise is not told apart from one changed only in the raise.",
    tests: "No Microsoft test. Outside report (al-runner issue 8945): a partner suite of about 5,500 tests that is not run at all.",
    probe: "Corpus PR 1571 (aldevtools/BusinessCentral.AL.Language.Tests, open), tests/al-language/events/platform-triggers/LmLoginCacheTests.al and tests/al-language-testlibraries/test-framework/login-caches/LmLoginCacheTlTests.al with the login subscribers of tests/al-language-xapp-a/LmXaLoginCaches.al: ChangeLog_AfterLogin (cell M3), ApiCache_CountingSubscriber and ApiCache_AnsweringSubscriber (M4), Profile_AfterLogin (M2), AppAreas_AfterLogin (M1), AllCaches_SecondCodeunit; measured on a local BC of the newest supported major, in the shape of the job with Microsoft's test libraries, both kinds of session alike. Still to measure: the two older majors, the shape without Microsoft's test libraries (423 then loads in the setup ask), the change log active at the load.",
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
