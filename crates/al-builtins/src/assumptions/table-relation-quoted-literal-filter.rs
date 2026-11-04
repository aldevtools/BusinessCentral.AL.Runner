TableRelationQuotedLiteralFilter => {
    id: "table-relation-quoted-literal-filter",
    assumed: "A TableRelation where() filter written as one AL string with a doubled quote in it is the filter its content spells: filter('<>''''') is <>'' (not empty), on a field of the related table and on one a tableextension adds, and filter('''''') is '' (the empty value). Other contents (filter('<>''') raises InvalidFilterExpression on BC) are refused. Measured on the newest major by the corpus writer's observe round for al-runner issue 6924; the pin is not on master yet.",
    tests: "No Microsoft test: a partner app's journal line whose Source No. relates to Location where an extension field is filter('<>''''') (28 tests, al-runner issue 6924).",
    probe: "TableRelationQuotes.al TRX_ConditionalArm_ExtFieldNotEmptyQuoted, TRX_Plain_ExtAndOwnFieldNotEmptyQuoted, TRX_Plain_ExtFieldOtherForms (objects TableRelationExtFieldFilter.Objects.al) on branch al-runtime-rs-staging-p6924c.",
    awaiting: Awaiting {
        issues: &[6924],
        files: &["tests/al-language/record/table-relation/TableRelationExtFieldFilter.Objects.al"],
        since: "2026-11-04",
    },
}
