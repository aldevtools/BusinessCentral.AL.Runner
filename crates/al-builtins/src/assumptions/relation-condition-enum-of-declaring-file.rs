RelationConditionEnumOfDeclaringFile => {
    id: "relation-condition-enum-of-declaring-file",
    assumed: "A `Field = const(Member)` condition of a TableRelation that a table extension sets (by `modify` or on a field it adds), on an Enum field the table declares, whose enum the extension's file cannot name (its namespace has no `using` of the enum's), names a member of the enum the table's file names: the condition applies as it does when the extension's file can name the enum, for Validate and for a rename. Every other shape whose enum the setting file cannot name stays refused: a field another extension declares, a relation the table sets itself, a qualified `\"Enum\"::Member`, and a member the enum lacks.",
    tests: "Base Application's \"Mfg. Item Journal Line\" (`modify(\"Order No.\")`, `if (\"Order Type\" = const(Production)) \"Production Order\".\"No.\" where(Status = const(Released))`, namespace Microsoft.Inventory.Journal without a using of Microsoft.Foundation.Enums) and \"Mfg. Standard Cost Worksheet\" (`modify(\"No.\")`, `if (Type = const(\"Machine Center\")) \"Machine Center\"`, namespace Microsoft.Manufacturing.StandardCost without a using of Microsoft.Inventory.StandardCost): Library - Manufacturing's consumption journal helpers and a Machine Center rename. An outside reporter's repro passed 3/3 on Business Central 29.1 (al-runner issue 8297); 28 tests of a partner app.",
    probe: "Corpus PR 1437 (open), tests/al-language/table-objects/extensions/ExtRelationEnumNamespace_Tests.al: a table extension in a namespace without a using of the enums' sets conditional relations on two Enum fields of the table (one member from an enumextension), by `modify` and on a field of its own; Validate of a matching, missing and filtered-out value, an arm that does not apply, the table's own arm, and a rename of the related row. Measured 11/11 green on a local BC 29.1 OnPrem container.",
    awaiting: Awaiting {
        issues: &[8297],
        files: &["tests/al-language/table-objects/extensions/ExtRelationEnumNamespace_Tests.al"],
        since: "2026-11-18",
    },
}
