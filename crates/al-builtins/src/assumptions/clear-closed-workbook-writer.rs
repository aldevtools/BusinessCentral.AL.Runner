ClearClosedWorkbookWriter => {
    id: "clear-closed-workbook-writer",
    assumed: "Clear of a DotNet variable that holds an OpenXml WorkbookWriter whose Close already ran, or a WorksheetWriter that writer handed out, leaves the variable unassigned and changes nothing else: no error, and the file Close wrote keeps its bytes (Excel Buffer.CloseBook: Close, then Clear of the worksheet writer and of the workbook writer). Whether BC calls Dispose on the object is not pinned; after Close a Dispose has nothing left to write.",
    tests: "Microsoft's Tests-ERM codeunit \"ERM Analysis View Excel Export\" (AnalysisViewGeneralInfo and the other tests that export through Export Analysis View): Excel Buffer.CloseBook runs the two Clears, then the test reads the workbook file; they pass in Microsoft's CI.",
    probe: "OnPrem DotNet app: W := W.Create(Stream) of a WorkbookWriter, S := W.FirstWorksheet, S.SetCellValueText(...), W.Close(); Clear(S); Clear(W): no error, IsNull of both Yes, and the stream's bytes the same length before and after the Clears. Also Clear(S) before W.Close(): does the sheet still hold the value after Close (a second WorkbookReader read of the cell)?",
}
