PermissionSetExclusionStaysInSet => {
    id: "permission-set-exclusion-stays-in-set",
    assumed: "The ExcludedPermissionSets of a permission set take only from that set's own content: not from a set included beside it by the same assigned set, and not from another assigned set (ADR-062), on every BC major, in the shapes corpus PR 1403 measured: (1) an assigned set that grants nothing itself, excludes nothing and includes exactly two sets, one that includes nothing and excludes exactly one set and grants no object that set grants (both sets grant explicit objects only, no wildcard), and one that includes and excludes nothing and declares what that excluded set declares (the excluded set itself, or a set with the same grants); the excluded set includes and excludes nothing; (2) the sets Permissions Mock.Set and Assign leave, 'All Objects' and two sets that include nothing, one of them excluding exactly the other, which excludes nothing, in either order of Set and Assign. The test is flagged when it assigns such a list. Every other include of a set that excludes sets, and every other list where one assigned set excludes what another grants, stays incomplete.",
    tests: "No Microsoft test. Corpus PR 1403 observed the cells on each major its CI runs (corpus CI run 43637000726): Insert and FindFirst of the table allowed, GetEffectivePermission Yes on Read, Insert, Modify and Delete.",
    probe: "Corpus PR 1403 (merged, not pinned; permissions/testpermissions/Y19PermExtAndCompose.al, test PC_A_Composition, ops 1-6). Wanted next: an excluding included set whose own grants overlap what it excludes, an including set with its own grants, deeper includes, two assigned sets that exclude each other, a set excluded by an included set that also includes it.",
    awaiting: Awaiting {
        issues: &[8092],
        files: &["tests/al-language/permissions/testpermissions/Y19PermExtAndCompose.al"],
        since: "2026-11-15",
    },
}
