TableRelationSeveralWhereNonKey => {
    id: "table-relation-several-where-non-key",
    assumed: "Inside an if() arm, a TableRelation check that fails, with several where() conditions none of which is on a primary key field of the related table, raises DB:NothingInsideFilter, as a plain relation does (record/table-relation/RelationWhereMulti.al:187-209). (Inside an if() arm a mix with key fields stays refused.)",
    tests: "Item Blocked Test and Item Variant Blocked Test (Item where(Blocked = const(false), \"Service Blocked\" = const(false)) and the sales and purchase variants), SCM Dedicated Bins; about 26 Tests-SCM tests. They compare the message only.",
    probe: "Inside an if() arm: a relation with two non-key where() conditions and a value that fails one of them, and one that is missing: GetLastErrorCode; the same with one key and one non-key condition (record/table-relation/RelationWhereKeyCoverage.al, corpus PR 1291).",
}
