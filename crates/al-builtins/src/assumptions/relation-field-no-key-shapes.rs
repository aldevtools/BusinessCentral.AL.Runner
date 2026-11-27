RelationFieldNoKeyShapes => {
    id: "relation-field-no-key-shapes",
    assumed: "Field.RelationFieldNo is 0 (RelationTableNo the target table) for a TableRelation that names a table and no field, in these shapes only, each with no where, the target table of the source's app, a source table whose primary key is (Integer, Code[20]) as in the probe, and every other fact of the pinned no-field shapes (one unconditional arm, a Normal field the table itself declares, default ObsoleteState, Enabled and TestTableRelation, two different tables of TableType Normal): a field outside that primary key, of type Code[20] onto a target keyed by (Code[20], Text[30]), (Integer, Code[20]) or (Code[20], Integer), or of type Integer onto one keyed by (Integer, Code[20]) or (Code[20], Integer) (FieldRelationFieldNo.al:393-447); a Code[20] field that is the second field of its table's (Integer, Code[20]) primary key, onto a target keyed by one Code[20] field (:382). Refused, as not measured: every other length (a Code[10] or Code[30] source, a Code[10] or Text[50] key field), every other source type (Text, BigInteger, ...), a key of three or more fields, a where on a two-field key, a field inside its own key in any other shape or onto a two-field key, and a source table keyed otherwise (on one field, by other types or in another order: a probe cell of such a source table onto a two-field key would widen it).",
    tests: "No Microsoft test. An outside report: al-runner issue 8592 (a Code field related to a table keyed by an Integer and a Code field), with the reporter's own measurement on a local BC 29.",
    probe: "Corpus PR 1546 (merged 2026-11-25, not yet pinned), tests/al-language/metadata/platform-tables/FieldRelationFieldNo.al:371-449 (codeunit 75930; tables :516-591), measured on local BC 29.0 and on BC 27.5, 28.5 and 29.0 by the corpus CI.",
    awaiting: Awaiting {
        issues: &[8592],
        files: &[
            "tests/al-language/metadata/platform-tables/FieldRelationFieldNo.al::FieldTable_NoField_SourceInsideOwnPrimaryKey",
            "tests/al-language/metadata/platform-tables/FieldRelationFieldNo.al::FieldTable_NoField_CodeToCodeTextKey",
            "tests/al-language/metadata/platform-tables/FieldRelationFieldNo.al::FieldTable_NoField_IntegerToIntegerCodeKey",
            "tests/al-language/metadata/platform-tables/FieldRelationFieldNo.al::FieldTable_NoField_CodeToIntegerCodeKey",
            "tests/al-language/metadata/platform-tables/FieldRelationFieldNo.al::FieldTable_NoField_CodeToCodeIntegerKey",
            "tests/al-language/metadata/platform-tables/FieldRelationFieldNo.al::FieldTable_NoField_IntegerToCodeIntegerKey",
        ],
        since: "2026-11-27",
    },
}
