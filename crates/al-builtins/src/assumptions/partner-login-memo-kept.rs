PartnerLoginMemoKept => {
    id: "partner-login-memo-kept",
    assumed: "In the test session (a client, Codeunit isolation, the login not raised through Microsoft's code), a SingleInstance codeunit that a \"System Initialization\".OnAfterLogin subscriber of a partner dependency app changed keeps what the login left, for every later read: by the test, by session-open code, by OnInsert of Insert(true), by the later tests of the codeunit and by the next codeunit (al-testrun `partner_login.rs` `kept`). Covered: a codeunit of a dependency app, the subscriber's own app or an app it depends on, whose globals are all Boolean, Integer, Text or List of [Text], or codeunit 3702 or 3703 of an app published by Microsoft (Environment Information Impl. and Server Setting Impl. of System Application, filled through EnvironmentInformation.IsSaaS(), IsSandbox() and ServerSetting.GetEnableMembershipEntitlement()); changed by one partner subscriber, or by subscribers of different apps. Every read of one of its globals marks this entry. Not covered, every read refused: a codeunit of the app under test, of an app the subscriber's app does not depend on, with a global of any other type, or changed by a second subscriber of the same app. The one codeunit of the login's only partner subscriber's app with two Integers, a List of [Text] and a Text is pinned (PlXaLoginSingles.al:12-27) and does not pass through this entry.",
    tests: "No Microsoft test. Outside reports (al-runner issue 8239): a partner suite of 11212 tests on BC 29.0 BE that is not run at all, and an invented two-app repro on BC 29 W1 where the login's IsSaaS() values equal the test's.",
    probe: "Corpus PR 1490 (aldevtools/BusinessCentral.AL.Language.Tests, open), tests/al-language/events/platform-triggers/P2LoginMemoTests.al with the memos and subscribers of tests/al-language-xapp-a/P2XaLoginMemos.al and tests/al-language-xapp-b/P2XbLoginMemos.al, measured on BC 27.5, 28.5 and 29.0, the same on all three (P2A_01 to P2A_14, P2B_01). Still to measure: a memo of the app under test, two subscribers of the same app, the order of two subscribers that change the same memo.",
    awaiting: Awaiting {
        issues: &[8239],
        files: &[
            "tests/al-language/events/platform-triggers/P2LoginMemoTests.al",
            "tests/al-language-xapp-a/P2XaLoginMemos.al",
            "tests/al-language-xapp-b/P2XbLoginMemos.al",
        ],
        since: "2026-11-21",
    },
}
