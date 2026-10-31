PageRowDeleteEventNotRaised => {
    id: "page-row-delete-event-not-raised",
    assumed: "A page whose table has a subscriber to OnDeleteRecordEvent opens; the test client has no operation that deletes the page's row, so the event is never raised.",
    tests: "Base Application subscribes on the General Journal and other journals; 88 Tests-General Journal tests.",
    probe: "Invoke the standard Delete action on a page row of a table with such a subscriber: is the subscriber run. Asked in corpus #651.",
}
