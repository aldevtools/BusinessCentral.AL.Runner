LoginOwnMemoAlone => {
    id: "login-own-memo-alone",
    assumed: "In the test session, where the login is raised to the apps under test's own subscribers only (al-testrun login.rs, Login::Own), a login subscriber of an app under test whose reach reads a global of codeunit 3702 or 3703 of a dependency app published by Microsoft (Environment Information Impl. and Server Setting Impl. of System Application, filled through EnvironmentInformation.IsSaaS(), IsSandbox() and ServerSetting.GetEnableMembershipEntitlement()) gets there, without Microsoft's login code before it, what it gets inside BC's login, and what it leaves there is what the login leaves (al-testrun `login::reads_microsofts_login`, `partner_login::own_step`). Every read of a global of 3702 or 3703 after such a subscriber changed it marks this entry. Not covered, as before: a subscriber whose reach reads any other SingleInstance global or any table that Microsoft's login code writes gets the login raised through Microsoft's code; 3702 or 3703 changed by a partner subscriber before the own one refuses its reads.",
    tests: "No Microsoft test. The corpus probe of al-runner issue 8239 (TiXaIsSaaSLogin.al, an XApp A OnAfterLogin subscriber that calls IsSaaS(), measured on BC 27.5 and 28.5 with TiIsSaaSLoginMemo.al) pins only the values on a sandbox tier, which do not tell the login's memo apart from an empty one.",
    probe: "Corpus PR 1490 (aldevtools/BusinessCentral.AL.Language.Tests, open), tests/al-language/events/platform-triggers/P2LoginMemoTests.al P2A_06 and P2A_11 with the XApp A subscriber of tests/al-language-xapp-a/P2XaLoginMemos.al, measured on a local BC of each of the three supported majors: the test reads from 3702 and 3703 the IsSaaS(), IsSandbox() and GetEnableMembershipEntitlement() values the login subscriber saw. Still to measure: a subscriber of the app under test itself (the probe's subscriber is in a dependency app that the corpus run compiles as an app under test), a global of 3702 or 3703 read through another procedure.",
    awaiting: Awaiting {
        issues: &[8239],
        files: &[
            "tests/al-language/events/platform-triggers/P2LoginMemoTests.al",
            "tests/al-language-xapp-a/P2XaLoginMemos.al",
        ],
        since: "2026-11-21",
    },
}
