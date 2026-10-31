DateToDateTime => {
    id: "date-to-datetime-is-midnight",
    assumed: "A Date assigned or converted to a DateTime is that day at midnight; the blank Date is the blank DateTime.",
    tests: "Microsoft's production order and routing code (Starting Date-Time := Starting Date): Tests-SCM-Manufacturing, about 20 tests.",
    probe: "DT := D for a defined and for a blank Date; Format(DT, 0, 9); DT := ClosingDate(D). Asked in corpus #649.",
}
