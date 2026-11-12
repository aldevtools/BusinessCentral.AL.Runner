TenantPermissionSetDeleteCascade => {
    id: "tenant-permission-set-delete-cascade",
    assumed: "TenantPermissionSet.DeleteAll() on a Record variable, without the trigger, with exactly one filter, on Role ID, deletes for each deleted set whose App ID is zero the Tenant Permission rows with the same App ID and Role ID, in the store after the set row's delete: no Tenant Permission delete event runs for them, and the set's Tenant Permission Set Rel. rows stay. Delete, DeleteAll(true), RecordRef.DeleteAll, other filters and another App ID keep main's result (only the set rows go).",
    tests: "System Application tests, Permission Import Export Tests (PermissionImportExportTests.Codeunit.al:195, :282, :383; asserts :300-301, :402): they read the permissions an import wrote after SetFilter(\"Role ID\", NewRoleId) and DeleteAll() of a set with App ID zero, and pass on real BC (al-runner issue 7854).",
    probe: "permissions/permission-sets/TenantSetDeleteCascade.al: Delete, Delete(true), DeleteAll, DeleteAll(true), RecordRef.Delete and RecordRef.DeleteAll of a Tenant Permission Set with Tenant Permission and Tenant Permission Set Rel. rows of its own and of another set; DeleteAll of a set whose App ID is not zero; a bound Tenant Permission OnAfterDeleteEvent subscriber during a set DeleteAll. Corpus pull request PRNUM.",
    awaiting: Awaiting {
        issues: &[7854],
        files: &["tests/al-language/permissions/permission-sets/TenantSetDeleteCascade.al"],
        since: "2026-11-12",
    },
}
