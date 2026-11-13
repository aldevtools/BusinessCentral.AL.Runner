FieldrefUnassignedFilters => {
    id: "fieldref-unassigned-filters",
    assumed: "On a FieldRef that holds no field (never assigned, or after Clear), FieldRef.SetRange with no argument or one non-empty Text, FieldRef.SetFilter with one non-empty Text and no placeholder value, FieldRef.GetFilter and FieldRef.GetRangeMin raise 'Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized.', code NotInitialized (al-vm refs.rs `unassigned_filter_member_measured`). Every other filter member or argument on such a FieldRef (SetRange with an Integer or two values, SetFilter('') or with placeholder values, GetRangeMax) keeps the refusal.",
    tests: "No Microsoft test. Outside report: al-runner issue 7964 item 2 (`Fld.SetRange('A')` on a FieldRef never assigned, Microsoft's Subcontracting tests).",
    probe: "Corpus PR 1382, merged on corpus master as 23337cd2, not pinned yet (recordref-fieldref/fieldref/FieldRefUnassignedFilters.al:88, :106, :127): the error on every BC major of the probe for every member above, never assigned and after Clear.",
    awaiting: Awaiting {
        issues: &[7964],
        files: &["tests/al-language/recordref-fieldref/fieldref/FieldRefUnassignedFilters.al"],
        since: "2026-11-13",
    },
}
