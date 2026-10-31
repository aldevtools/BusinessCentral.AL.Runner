TableRelationQuotes => {
    id: "table-relation-quotes",
    assumed: "A filter(...) text of a TableRelation where() clause or arm condition that quotes an identifier with \" is read like a SetFilter text after the quotes are rewritten to '. (Pinned, no mark, on an Enum or Option field: members joined by |, one quoted member alone and <> one quoted member, record/lt-table-relation-quotes/TableRelationQuotes.al:41-46; a range with one quoted and one plain bound, A..\"B C\" and \"B C\"..D, two <> terms joined by & with one quoted, and on an Enum field \"B C\".., TableRelationQuotes.al:279-397; in the if (...) condition of an arm on an Option field, <> \"B C\" & <> D and \" \" | A, TableRelationQuotes.al QP_Arms_* and QP_BlankOr_*.) Assumed: other ranges and filter operators (..\"B C\", two quoted bounds, more & terms), a quoted blank member in a where() clause, a field of another type, and the other forms of an arm condition.",
    tests: "Microsoft relations with a quoted member in another form than the pinned ones.",
    probe: "TableRelation ... where(F = filter(..\"B C\")), filter(\" \" | A) and an arm if (F = filter(\"B C\")), Validate with a value of each kind.",
}
