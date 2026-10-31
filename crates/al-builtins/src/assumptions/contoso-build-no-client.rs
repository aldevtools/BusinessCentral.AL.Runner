ContosoBuildNoClient => {
    id: "contoso-build-no-client",
    assumed: "While the Contoso company is built (`--recipe microsoft`, IntegrationTest and Uncategorized lanes; Company-Initialize and every Contoso step), GuiAllowed answers false: BCApps' CI runs each step through Invoke-NavContainerCodeunit (build/scripts/ImportTestDataInBcContainer.ps1 of releases/29.0, Invoke-ContosoDemoTool :307-326), a session without a client (owner decision 2026-10-23). The tests themselves keep GuiAllowed true. Message is captured and Confirm refused as in any company build.",
    tests: "Every test of Microsoft's IntegrationTest and Uncategorized lanes: their company is built by the Contoso Demo Tool, and with a client the all-data build stops at Sales Header.ConfirmKeepExistingDimensions (SalesHeader.Table.al:5163, guarded by GuiAllowed); they pass in Microsoft's CI.",
    probe: "A recipe fact, not a language rule: GuiAllowed in a session started by Invoke-NAVCodeunit on the Windows nightly. The corpus pins GuiAllowed false only for a page background task (bgtasks/lt-bj-no-gui/BjNoGuiDialogs.al:8-9).",
}
