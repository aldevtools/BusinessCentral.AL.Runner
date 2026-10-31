FlowFieldEnumIntoOption => {
    id: "flowfield-enum-into-option",
    assumed: "An Option FlowField whose CalcFormula is a lookup of an Enum field holds the Enum member (Format shows the Enum's member name, the ordinal is the member's, also one the Option lacks; no row gives the Option's first member); a max of an Enum field gives an Option ordinal (Format shows the Option's member, or the number when the Option lacks it); a filter on the FlowField compares ordinals.",
    tests: "No Microsoft test: al-runner issue 6481, extensions whose Option FlowFields look up Base Application fields that became enums. Measured on BC by corpus writer Y20 (Y20FlowFieldLookupEnumIntoOption.al, SaaS sandbox, newest major).",
    probe: "Y20FlowFieldLookupEnumIntoOption.al on branch al-runtime-rs-staging-y20: Format, ordinal and Format(,0,9) of the lookup for members the Option has and lacks and no row; max per group; SetRange and SetFilter on the FlowField.",
    awaiting: Awaiting {
        issues: &[6481],
        files: &["tests/al-language/record/lt-flowfield-lookup-enum/Y20FlowFieldLookupEnumIntoOption.al"],
        since: "2026-10-31",
    },
}
