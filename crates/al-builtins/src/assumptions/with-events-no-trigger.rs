WithEventsNoTrigger => {
    id: "with-events-no-trigger",
    assumed: "A global [WithEvents] DotNet variable of a codeunit that declares no `trigger Variable::Event` for it runs as a plain DotNet variable: no AL code runs on its events. A WithEvents variable whose object declares a trigger for it, a local one, and one of another object type stay refused.",
    tests: "Microsoft's Sales and Inventory Forecast Tests, codeunit 139540 TestMissingApiUriOpenSetup: it passes on real BC through Base App's Azure ML Connector.Initialize, which assigns the WithEvents variable AzureMLRequest; that codeunit declares no event trigger. al-runner issue 7832 (an outside report).",
    probe: "dotnet/ac/N4WithEventsNoRaise.al (observe form): a WithEvents Timer with no trigger, with only an Elapsed trigger, and with a Disposed trigger, assigned, reassigned, read and cleared, and through a helper instance that ends. Corpus branch al-runtime-rs-staging-natives4.",
    awaiting: Awaiting {
        issues: &[7832],
        files: &["tests/al-language-onprem-dotnet/dotnet/ac/N4WithEventsNoRaise.al"],
        since: "2026-11-12",
    },
}
