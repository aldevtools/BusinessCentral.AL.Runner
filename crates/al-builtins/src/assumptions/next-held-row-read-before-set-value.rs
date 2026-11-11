NextHeldRowReadBeforeSetValue => {
    id: "next-held-row-read-before-set-value",
    assumed: "After TestPage.Next() onto a row a Worksheet already holds, which runs no read trigger on a page whose only read trigger is OnAfterGetRecord (events/extension-events/PageExtensionSender.al:643), a SetValue on a repeater control bound to a page variable with an OnValidate first reads the row again: OnAfterGetRecord runs for it before the control takes the value, so the page's per-line variables belong to that row.",
    tests: "Subscription Billing: Contract Renewal Test.RenewalTermEnteredValueIsRetainedOnPage (scenario 645198).",
    probe: "A Worksheet (and a List, a ListPart) with only OnAfterGetRecord, 2 and 30 rows, logging R:<id> and V:<id>/<per-line var>: First, SetValue on a page-variable control with OnValidate (with and without CurrPage.Update(false)), Next, SetValue again; the same on a Rec field control and on a control without OnValidate; Next twice then SetValue; Previous then SetValue.",
}
