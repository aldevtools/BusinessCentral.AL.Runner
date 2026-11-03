TextDigitsToInteger => {
    id: "text-digits-to-integer",
    assumed: "A Text or Code of digits (optional sign, surrounding spaces ignored) converts into an Integer or BigInteger where a Variant meets a number, outside the key of Get (a variable-typed argument of a builtin other than Get); a text with anything else stays refused. (Pinned and not assumed: the key of Record.Get, misc/lt-y70-assumptions/Y70Probes.al RecordGet_TextOnIntegerKey and RecordGet_TextOnBigIntegerKey.)",
    tests: "Incoming Document.FindFromIncomingDocumentEntryNo (`Get(Format(FieldRef.Value))`): about 400 Tests-ERM tests behind the Incoming Doc. Attach. FactBox.",
    probe: "A Text of digits handed as a Variant to a builtin that takes a number other than Get (SetRange, Evaluate of a Variant, FieldRef.Value := Text): the number, and the error of a text that is none.",
}
