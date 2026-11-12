TenantPermissionSetDeleteCascade => {
    id: "tenant-permission-set-delete-cascade",
    assumed: "A Delete or DeleteAll of a stored Tenant Permission Set row deletes the Tenant Permission rows with the same App ID and Role ID, after the row is deleted. Tenant Permission Set Rel. rows of the set are kept, as before (not measured).",
    tests: "System Application tests, Permission Import Export Tests.ExportImportTenantPermissionSetWithAdditionalPermissions (reads the permissions an import wrote after TenantPermissionSet.DeleteAll()), and the Permission Set page of System Application, which deletes only the set; both work on real BC (al-runner issue 7854).",
    probe: "permissions/permission-sets/TenantSetDeleteCascade.al: Delete, Delete(true) and DeleteAll of a Tenant Permission Set with Tenant Permission and Tenant Permission Set Rel. rows of its own and of another set; the rows left in both tables.",
    awaiting: Awaiting {
        issues: &[7854],
        files: &["tests/al-language/permissions/permission-sets/TenantSetDeleteCascade.al"],
        since: "2026-11-12",
    },
}
