RenameFollowsFilterConditionArms => {
    id: "rename-follows-filter-condition-arms",
    assumed: "A rename of a primary key rewrites a referencing field through a TableRelation arm whose conditions include `Field = filter(x)`, when the referencing row's field passes the filter, as it does for `const` conditions. A single `filter(<> X)` is pinned (record/lt-table-relation-arms/TableRelationArms.al:208); other filter forms (`< 0`, `<> A & <> B`) are assumed alike.",
    tests: "Tests-ERM price list tests that rename an Item, Resource, G/L Account or Item Variant (Assembly Header.\"Bin Code\" `Quantity = filter(< 0)`, Purchase Line.\"No.\" and \"Variant Code\" `Document Type = filter(<> \"Credit Memo\" & <> \"Return Order\")`); about 28 Tests-ERM tests.",
    probe: "A table with a field whose TableRelation is `if (Qty = filter(< 0)) Parent` and one with `if (Kind = filter(<> B & <> C)) Parent`: rename a Parent row with referencing rows inside and outside each filter; read the fields.",
}
