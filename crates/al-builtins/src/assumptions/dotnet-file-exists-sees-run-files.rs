DotNetFileExistsSeesRunFiles => {
    id: "dotnet-file-exists-sees-run-files",
    assumed: "DotNet System.IO.File.Exists answers Yes for a file the run wrote into its in-memory file system (ADR-056): with AL's File type, a report's SaveAsXml, or XmlDocument.Save. The corpus pins No after a delete and for a path never written (onprem-dotnet lt-qx-dotnet/QxDotNetDemoShapes.al:328, :334), and AL's own File.Exists Yes for a written file (:323), but not .NET's Yes.",
    tests: "Every Microsoft report test that reads its dataset through Library - Report Dataset (LoadDataSetFile: SaveAsXml into FileManagement.ServerTempFileName, then LoadXMLDocumentFromFile checks the file with DotNet File.Exists); they pass in Microsoft's CI.",
    probe: "Write a file under TemporaryPath() with AL's File type, then DotNet System.IO.File.Exists of the same path; the same after Report.SaveAsXml into FileManagement.ServerTempFileName('xml').",
}
