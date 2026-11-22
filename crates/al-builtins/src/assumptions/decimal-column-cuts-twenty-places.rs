DecimalColumnCutsTwentyPlaces => {
    id: "decimal-column-cuts-twenty-places",
    assumed: "The SQL column behind a Decimal field of a database table holds 20 decimals. Only this field shape: a Normal Decimal field the table itself declares, outside the primary key, with exactly one key that holds it, a secondary key of its own (that field alone) that the table itself declares with no properties. An Insert() or a Modify() that AL code calls on a Record variable, without running the trigger, stores a value with more than 20 decimals cut toward zero after the 20th decimal; the variable keeps its own value, and Get and the Find methods read the cut value. A Count() that AL code calls on a Record variable compares the filter values of such a field cut the same way, for a filter set in filter group 0 by that variable's SetRange with one or two Decimal values or its SetFilter('<%1') or SetFilter('>%1') with one Decimal value; a Record variable assigned from it carries the same filter. Refused: on such a field, a value the column cuts written by any other writer (Insert(true), Modify(true), Rename, ModifyAll, a RecordRef or FieldRef write, a write no AL Record call started), and such a filter value read by any other reader (Find, FindSet, IsEmpty, CalcSums, a FlowField, DeleteAll, ModifyAll, a Count not called by AL on the variable) or set by any other setter (FieldRef, SetFilter text, a placeholder form other than '<%1' and '>%1', another filter group, CopyFilters). A temporary table and every other field shape keep the values as they are.",
    tests: "No Microsoft test: outside pull request 7866 (the contributor's measurement on a local Business Central of each of the three supported majors).",
    probe: "Open corpus pull request 1427, tests/al-language/record/crud/RcAGetFind.al:749-778 (Insert), :780-799 (Modify), :801-842 (SetRange and SetFilter filters with the full and the cut value, read by Count); field shape tests/al-language/record/crud/RcAObjects.al:24, :35 (field Amt with key ByAmt).",
    awaiting: Awaiting {
        issues: &[],
        files: &[
            "tests/al-language/record/crud/RcAGetFind.al::Record_Insert_HighScaleDecimal_TruncatesStoredValue",
            "tests/al-language/record/crud/RcAGetFind.al::Record_Modify_HighScaleDecimal_TruncatesStoredValue",
            "tests/al-language/record/crud/RcAGetFind.al::Record_SetFilter_HighScaleDecimal_UsesStoredPrecision",
        ],
        since: "2026-11-22",
    },
}
