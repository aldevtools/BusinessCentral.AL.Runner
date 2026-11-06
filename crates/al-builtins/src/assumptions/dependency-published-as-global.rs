DependencyPublishedAsGlobal => {
    id: "dependency-published-as-global",
    assumed: "With `--dependencies-published-global` (serve: `dependenciesPublishedGlobal`), the \"Published As\" column of \"NAV App Installed App\" is Global (0) for a dependency app by another publisher than Microsoft, as for a dependency that a pipeline published as a compiled .app through the per-tenant or global route. Without the option such an app is Dev (2), as published through the development endpoint (record/lt-r7142-installed-app/R7169DepInstallPublishedAs.al). The option is a documented run setting: no corpus test pins Global for a non-Microsoft app.",
    tests: "No Microsoft test: an outside report (an app's install code filters NAV App Installed App on Published As for its dependencies).",
    probe: "A corpus probe that publishes a dependency app of another publisher the way such a pipeline does, and reads Published As in its install code and in a test. Not queued: how an outside pipeline publishes is not reproducible in the corpus CI, so the value stays a documented setting.",
}
