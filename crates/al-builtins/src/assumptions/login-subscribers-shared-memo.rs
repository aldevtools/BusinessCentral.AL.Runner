LoginSubscribersSharedMemo => {
    id: "login-subscribers-shared-memo",
    assumed: "In the test session (a client, Codeunit isolation, the login not raised through Microsoft's code), a SingleInstance codeunit of a partner dependency app that three or ten \"System Initialization\".OnAfterLogin subscribers of partner dependency apps changed, in one run of the event, keeps what they left, for every later read (al-testrun `login_shared_memo.rs`). Covered, one shape per measured cell: three subscribers of the codeunit's own app, the codeunit holding two temporary records of tables of its own app, three Booleans, two Texts and two Integers (State One); one subscriber of each of three apps, the codeunit's own app first, each later app depending on every earlier one and the lowest subscriber codeunit id of each app above the one before, with the same globals (State Many); six subscribers of the codeunit's own app, then two of each of two more apps along such a chain, the codeunit holding one List of [Text] and one Text (Order). The event runs the subscribers of one app in ascending codeunit id and those of one codeunit in procedure name order, and the apps in dependency order. Not covered, every read refused as before: another number or order of subscribers or apps, a subscriber of an app under test, other globals, apps that are no such chain or whose lowest subscriber ids do not ascend, a codeunit that a Global Triggers event raised during the login changed (its first load can run in Microsoft's login code in some sessions), and a load-once cache that a Global Triggers ask loads (cell E; the shapes of corpus PR 1571, whose values depend on the load's body: F3 keeps the asking table and F4 watches \"User Login\"). Every read of one of its globals marks this entry.",
    tests: "No Microsoft test. Outside report (al-runner issue 8945): a partner suite whose test session refuses because more than two login subscribers change one SingleInstance codeunit.",
    probe: "Corpus PR 1567 (aldevtools/BusinessCentral.AL.Language.Tests, open), tests/al-language/events/platform-triggers/LsLoginSubscriberTests.al with tests/al-language-xapp-a/LsXaLoginSubscribers.al, tests/al-language-xapp-b/LsXbLoginSubscribers.al and tests/al-language-xapp-c/LsXcLoginSubscribers.al, cells A and B, measured on a local BC 29.0 and by its CI on 27 to 29. Still to measure: dependency order against publish order and lowest subscriber id (the probe's apps agree on all three), other subscriber counts and global types, and a load-once cache that a Global Triggers setup subscriber fills (cell E; corpus PR 1571, aldevtools/BusinessCentral.AL.Language.Tests, open, LmLoginMemoShapeTests.al F1 to F5, measures it: the runner refuses it until it reads the load's body).",
    awaiting: Awaiting {
        issues: &[8945],
        files: &[
            "tests/al-language/events/platform-triggers/LsLoginSubscriberTests.al",
            "tests/al-language-xapp-a/LsXaLoginSubscribers.al",
            "tests/al-language-xapp-b/LsXbLoginSubscribers.al",
            "tests/al-language-xapp-c/LsXcLoginSubscribers.al",
        ],
        since: "2026-11-24",
    },
}
