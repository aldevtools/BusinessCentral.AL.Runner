InstallSessionCanCreateTask => {
    id: "install-session-can-create-task",
    assumed: "`TaskScheduler.CanCreateTask()` answers true while an app's install code runs in its install session (al-builtins `native/session_fns.rs` `can_create_task`, read by `Ctx::install_without_client`): in an `OnInstallAppPerCompany` and an `OnInstallAppPerDatabase` subscriber of `Extension Triggers` and the code they call, per company and per database. REFUSED (`Ctx::install_company_open`, set by al-testrun `install.rs` `company_open`): the call inside the Company Triggers `OnCompanyOpen` and `OnCompanyOpenCompleted` subscribers that the install session runs before the install code, and inside the System Initialization `OnAfterLogin` subscribers they raise; the answer there is not measured. Not covered, main's answer (false): Company-Initialize, a company build step and every session other than an app's install session; the test session's false is pinned (session-system/TestEnvironmentUserTenantContracts.al:235). Upgrade code is not measured and the runner runs none.",
    tests: "No Microsoft test. Corpus PR 1589 (merged in the corpus at 60d576c88, green on 27.5, 28.5, 29.0 and bc-linux; not in the runner's pin yet), tests/al-language/install-upgrade/IsInstallSessionTests.al:86, :91 (per company: `can create task=Yes`), :135 (per database), :169 (test session: `No`).",
    probe: "Corpus PR 1589, tests/al-language/install-upgrade/IsInstallSession.al: the install subscribers write `Format(TaskScheduler.CanCreateTask())` at their start. Still to measure, refused until then: `Format(TaskScheduler.CanCreateTask())` written from a Company Triggers `OnCompanyOpenCompleted` subscriber and from a System Initialization `OnAfterLogin` subscriber while `GetExecutionContext()` is `Install` (the `Note()` pattern of IsInstallSession.al).",
    awaiting: Awaiting {
        issues: &[9082],
        files: &["tests/al-language/install-upgrade/IsInstallSessionTests.al"],
        since: "2026-11-25",
    },
}
