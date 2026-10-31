ClearWorkbookReader => {
    id: "clear-workbook-reader",
    assumed: "Clear of a DotNet variable that holds an OpenXml WorkbookReader (from WorkbookReader.Open) or a WorksheetReader it handed out, when no other AL variable holds the object, leaves the variable unassigned and changes nothing else: no error (Excel Buffer.CloseBook, run at the end of ReadSheet). Whether BC disposes the reader, and whether that closes the AL InStream it was opened over (already read to its end when handed to .NET), is not pinned.",
    tests: "Microsoft's Tests-ERM codeunit \"ERM Account Schedule II\" CreateNewExcelTemplate: Excel Buffer.OpenBookStream, then ReadSheet (which ends with CloseBook: Clear of the worksheet reader and the workbook reader), twice, then the rows are compared; it passes in Microsoft's CI.",
    probe: "OnPrem DotNet app: R := R.Open(InStr) over a workbook blob, S := R.GetWorksheetByName('Sheet1'), Clear(S); Clear(R): no error, IsNull Yes; then InStr.EOS and InStr.Read of the same InStream. Also S2 := S before Clear(S): is S2 still usable (S2.RowCount)?",
}
