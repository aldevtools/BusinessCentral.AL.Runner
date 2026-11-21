LoginOwnMemoAlone => {
    id: "login-own-memo-alone",
    assumed: "In the test session, where the login is raised to the apps under test's own subscribers only (al-testrun login.rs, Login::Own), a login subscriber of an app under test that fills System Application's memos (codeunits 3702 Environment Information Impl. and 3703 Server Setting Impl. of a dependency app published by Microsoft) gets there, without Microsoft's login code before it, what it gets inside BC's login, and the globals it leaves there are what the login leaves (al-testrun `login_memo::filled_alone`, `partner_login::own_step`). Covered only when the subscribers' reach enters, of the facades 457 Environment Information and 6723 Server Setting of that app, only IsSaaS(), IsSandbox() and GetEnableMembershipEntitlement(), calls them only from codeunits of the apps under test, and reads no global of 3702 or 3703 other than through those calls. Every read of a global of 3702 or 3703 after an app under test's login subscriber changed it marks this entry. Not covered, as before: any other read of 3702 or 3703 that Microsoft's login code writes, or of another SingleInstance global or table it writes, gets the login raised through Microsoft's code; 3702 or 3703 changed by a partner subscriber before the own one refuses its reads.",
    tests: "No Microsoft test. The corpus probe of al-runner issue 8239 (TiXaIsSaaSLogin.al, an XApp A OnAfterLogin subscriber that calls IsSaaS(), measured on BC 27.5 and 28.5 with TiIsSaaSLoginMemo.al) pins only the values on a sandbox tier, which do not tell the login's memo apart from an empty one.",
    probe: "Corpus PR 1490 (aldevtools/BusinessCentral.AL.Language.Tests, open), tests/al-language/events/platform-triggers/P2LoginMemoTests.al P2A_06 and P2A_11 with the XApp A subscriber of tests/al-language-xapp-a/P2XaLoginMemos.al, measured on a local BC of each of the three supported majors: the test reads from 3702 and 3703 the IsSaaS(), IsSandbox() and GetEnableMembershipEntitlement() values the login subscriber saw. The probe's subscriber is in XApp A, which the corpus run compiles as an app under test. Still to measure (they keep the login raised through Microsoft's code): another procedure of the facades, a call from Microsoft's or a partner's code.",
    awaiting: Awaiting {
        issues: &[8239],
        files: &[
            "tests/al-language/events/platform-triggers/P2LoginMemoTests.al",
            "tests/al-language-xapp-a/P2XaLoginMemos.al",
        ],
        since: "2026-11-21",
    },
}
