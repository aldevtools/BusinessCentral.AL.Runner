NavAppInstalledCompatibilityColumns => {
    id: "nav-app-installed-compatibility-columns",
    assumed: "NAV App Installed App (2000000153) of every installed app: Compatibility Major, Minor, Build and Revision (columns 9 to 12) are 0, Hash Algorithm (14) is 1 and Extension Type (15) is 2. Content Hash (13) is not blank on BC but its value is not known: it stays refused.",
    tests: "No Microsoft test: an outside report (al-runner issue 7142: install code builds an app's version text from Version Major, Version Minor, Compatibility Build and Compatibility Revision).",
    probe: "Pinned by R7142InstalledAppCompat.al:25-63 and :113-123 (corpus PR 1156, branch al-runtime-rs-staging-r7142), measured on the 27.5, 28.5 and 29.0 CI containers for the corpus app, its fixture app, System Application and Base Application: retire when the pin includes it.",
    awaiting: Awaiting {
        issues: &[7142],
        files: &["tests/al-language/record/lt-r7142-installed-app/R7142InstalledAppCompat.al"],
        since: "2026-11-05",
    },
}
