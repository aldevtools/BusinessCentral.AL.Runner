ReportDatasetColumnLineBreaks => {
    id: "report-dataset-column-line-breaks",
    assumed: "The SaveAs XML dataset writes a CR, LF or CRLF inside text column data as one literal CRLF.",
    tests: "Local neutral Dataset.al probe measuring the raw XML bytes.",
    probe: "Add the XML dataset text column line break cases to the pinned corpus.",
}
