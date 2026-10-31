TextDigitsToInteger => {
    id: "text-digits-to-integer",
    assumed: "A Text or Code of digits (optional sign, surrounding spaces ignored) converts into an Integer or BigInteger where a Variant meets a number, such as the key of Get; a text with anything else stays refused.",
    tests: "Incoming Document.FindFromIncomingDocumentEntryNo (`Get(Format(FieldRef.Value))`): about 400 Tests-ERM tests behind the Incoming Doc. Attach. FactBox.",
    probe: "Rec.Get(Format(5)) on an Integer key; Get('5'); Get('x'); Get('') and Get(' 5 ') on an Integer and a BigInteger key. Asked in corpus #644.",
}
