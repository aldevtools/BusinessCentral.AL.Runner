LoginOwnRegistrationReadsAlone => {
    id: "login-own-registration-reads-alone",
    assumed: "Where the login is not raised for a test codeunit, the login events are raised to the apps under test's own subscribers only, before the first test (al-testrun login.rs, Login::Own). Their raise, order and session are pinned (events/platform-triggers/LgnLoginChecklistTests.al:131-147); assumed: they leave alone what they leave inside the login, where the dependency apps' subscribers to the same events run around them. Own is used only where their reach reads no table and no SingleInstance global the dependency apps' login code writes, except through calls of Guided Experience or Checklist keyed on a constant object of an app under test. Marked when a test reads or writes a table they wrote.",
    tests: "No Microsoft test. Outside reporters' repros: al-runner issues 7605 and 7717 (an app's OnRegisterAssistedSetup subscriber registers its wizard; the test reads the row), which pass on BC 29.0.",
    probe: "events/platform-triggers/LgnLoginRegistrationAlone.al (queued), next to LgnLoginChecklistTests.al: an OnRegisterAssistedSetup subscriber that registers its own wizard while Microsoft's subscribers register theirs, and reads its own row and Microsoft's; a test codeunit that inserts its own Guided Experience row first and reads what the subscriber stored.",
    awaiting: Awaiting {
        issues: &[7605, 7717],
        files: &["tests/al-language/events/platform-triggers/LgnLoginRegistrationAlone.al"],
        since: "2026-11-11",
    },
}
