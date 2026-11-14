DataCaptionRunAtOpen => {
    id: "data-caption-run-at-open",
    assumed: "AL code's static Page.Run(Id, Rec) with a PageHandler, or Page.RunModal(0, Rec) as a statement with a ModalPageHandler, opens a top-level page with its own DataCaptionExpression, no property but PageType, SourceTable, Caption, ApplicationArea, UsageCategory, DataCaptionExpression and DelayedInsert, only the triggers OnInit, OnOpenPage and OnQueryClosePage, no extension or subscriber, controls bound to fields of Rec or page variables without triggers, over a stored table handed with one filter of filter group 0. A Worksheet with an empty view and the page editable after OnOpenPage (DelayedInsert or not; RunModal only with DelayedInsert): the expression runs twice before the handler, which reads the globals it assigned and the caption with its value. A Worksheet with one row: it runs once, the handler reads the globals as they were and the caption with its value (CurrPage.Editable := false only measured with DelayedInsert). A Worksheet with an empty view and CurrPage.Editable := false in OnOpenPage: it does not run, the caption stands alone. A Card with DelayedInsert, editable, empty view or one row, Page.Run only: it runs once (not again at the handler's first call). After the open the handler may only read the caption and the controls; any other call is refused.",
    tests: "No Microsoft test: al-runner issue 6924 (an outside reporter's page: DataCaptionExpression fills page globals from Rec.GetFilter, a PageHandler reads the controls).",
    probe: "Corpus PR 1392, pages/properties/DataCaptionExprWorksheet.al: :412, :426, :439, :452, :465, :478, :491, :503, :515, :528, :541, :559, :572, :586, :599, :613, measured alike on BC 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/pages/properties/DataCaptionExprWorksheet.al"],
        since: "2026-11-14",
    },
}
