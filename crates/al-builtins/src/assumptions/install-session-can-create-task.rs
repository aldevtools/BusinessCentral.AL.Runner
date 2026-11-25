InstallSessionCanCreateTask => {
    id: "install-session-can-create-task",
    assumed: "`TaskScheduler.CanCreateTask()` answers true while an app's install code runs in its install session (al-builtins `native/session_fns.rs` `can_create_task`, read by `Ctx::install_without_client`): in an `OnInstallAppPerCompany` and an `OnInstallAppPerDatabase` subscriber of `Extension Triggers`, per company and per database. The same install session also runs the Company Triggers `OnCompanyOpen` and `OnCompanyOpenCompleted` subscribers before the install code (and through them System Initialization `OnAfterLogin`); the answer there is not measured at that point and is taken to be the session's answer, so a call there runs through this entry too. Not covered, main's answer (false): Company-Initialize, a company build step and every session other than an app's install session; the test session's false is pinned (session-system/TestEnvironmentUserTenantContracts.al:235). Upgrade code is not measured and the runner runs none.",
    tests: "No Microsoft test. Corpus PR 1589 (merged in the corpus at 60d576c88, green on 27.5, 28.5, 29.0 and bc-linux; not in the runner's pin yet), tests/al-language/install-upgrade/IsInstallSessionTests.al:86, :91 (per company: `can create task=Yes`), :135 (per database), :169 (test session: `No`).",
    probe: "Corpus PR 1589, tests/al-language/install-upgrade/IsInstallSession.al: the install subscribers write `Format(TaskScheduler.CanCreateTask())` at their start. Still to measure: the call inside a Company Triggers `OnCompanyOpenCompleted` or System Initialization `OnAfterLogin` subscriber during the install.",
    awaiting: Awaiting {
        issues: &[9082],
        files: &["tests/al-language/install-upgrade/IsInstallSessionTests.al"],
        since: "2026-11-25",
    },
}
