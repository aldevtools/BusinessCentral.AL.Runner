ResourceFileNameCase => {
    id: "resource-file-name-case",
    assumed: "A File.Open, File.Exists or read of a resource file of the company data (`--company-data demotool`) whose name differs from the file on disk only in case finds the file, as on the Windows server BCApps' CI builds the DemoTool company on (the DemoTool opens `CurrencyData.txt`, BCApps ships `CurrencyData.TXT`). A name that matches exactly does not take this path.",
    tests: "Every test of Microsoft's Legacy test lane on a release whose artifact has no RapidStart package: their company is built by the DemoTool (Create Demonstration Data.OnRun opens `CurrencyData.txt`), and they pass in Microsoft's CI.",
    probe: "Not measurable from the corpus: the case rule is the server's file system, and the corpus CI runs BC on Linux. On the Windows nightly: File.Exists and File.Open of a file in the server's folder by a name in other case.",
}
