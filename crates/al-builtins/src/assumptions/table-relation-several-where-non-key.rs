TableRelationSeveralWhereNonKey => {
    id: "table-relation-several-where-non-key",
    assumed: "A TableRelation check that fails, with several where() conditions none of which is on a primary key field of the related table, raises DB:NothingInsideFilter, as one condition on a non-key field does. (A mix with key fields stays refused.)",
    tests: "Item Blocked Test and Item Variant Blocked Test (Item where(Blocked = const(false), \"Service Blocked\" = const(false)) and the sales and purchase variants), SCM Dedicated Bins; about 26 Tests-SCM tests. They compare the message only.",
    probe: "A relation with two non-key where() conditions and a value that fails one of them, and one that is missing: GetLastErrorCode; the same with one key and one non-key condition.",
}
