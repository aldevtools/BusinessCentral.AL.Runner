FormatCodeSpelling => {
    id: "format-code-spelling",
    assumed: "In a Format string `Filler Char` stands for `Filler Character`. (Any case of the other code names, and blanks after the comma, are pinned by types/lt-format-codes-textbuilder-pagerun-labels/P144FormatCodes.al.)",
    tests: "The Data Exchange tests' GetNodeID (`<Integer,4><Filler Char,0>`, about 30 tests).",
    probe: "Format(5, 0, '<Integer,4><Filler Char,0>') next to '<Integer,4><Filler Character,0>', for an Integer, a Decimal and a Text. Asked in corpus #649.",
}
