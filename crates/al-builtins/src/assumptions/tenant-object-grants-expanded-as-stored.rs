TenantObjectGrantsExpandedAsStored => {
    id: "tenant-object-grants-expanded-as-stored",
    assumed: "The Expanded Permission rows of a tenant permission set list its own Tenant Permission rows of the object types Table, Report, Codeunit, XMLport, Page, Query and System as they are stored, beside its Table Data rows, ordered by object type and id, on every BC major, in the shape corpus PR 1403 measured: rows of Type Include, without a Security Filter, with every permission column Yes (Read, Insert, Modify, Delete and Execute), for an object of that type in the program or System object 1350; the set has only its own rows (it includes and excludes no set, has no row of Type Exclude) and no other set includes it. Every other tenant grant of an object type other than Table Data is refused.",
    tests: "No Microsoft test. Corpus PR 1403 observed the rows on each major its CI runs (corpus CI run 43637000726).",
    probe: "Corpus PR 1403 (open; permissions/testpermissions/Y19PermExtAndCompose.al, test PC_D_ExpandedPermissionOfTenantObjectGrants). Wanted next: other levels (blank, Indirect) on object rows, object id 0, objects not in the program, other System ids, object rows in included and excluded sets.",
    awaiting: Awaiting {
        issues: &[7854],
        files: &["tests/al-language/permissions/testpermissions/Y19PermExtAndCompose.al"],
        since: "2026-11-15",
    },
}
