DotNetGenericResultType => {
    id: "dotnet-generic-result-type",
    assumed: "A DotNet call to a method that is generic only, whose one type parameter is its return type, runs closed over the declared type of the DotNet variable the result is assigned to (`S := R.GetWorksheetByName(Name)` with S: DotNet WorksheetReader calls GetWorksheetByName<WorksheetReader>). OpenXml's method converts the sheet to T, so on real BC no other type argument returns the sheet; how BC picks it is not pinned. A call whose result is not assigned straight to a DotNet variable is refused.",
    tests: "Microsoft's Tests-ERM codeunit \"ERM Account Schedule II\" CreateNewExcelTemplate: Excel Buffer.OpenBookStream assigns WorkbookReader.GetWorksheetByName to its WorksheetReader variable, then ReadSheet; it passes in Microsoft's CI.",
    probe: "OnPrem DotNet app: a WorkbookReader over a workbook with a sheet, `S := R.GetWorksheetByName('Sheet1')` with S of type WorksheetReader, then S.Name; the same assigned to a DotNet variable of another type (System.Object, Variant) and used without assignment (`if IsNull(R.GetWorksheetByName(...))`): error text or result.",
}
