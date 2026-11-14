TenantSetDeleteCascade => {
    id: "tenant-set-delete-cascade",
    assumed: "The Delete, Delete(true), DeleteAll or DeleteAll(true) of a stored Tenant Permission Set row with App ID zero, through a Record or a RecordRef, deletes the Tenant Permission rows with its Role ID and the Tenant Permission Set Rel. rows that name its Role ID on either side, as its probe measured: every such Tenant Permission row has App ID zero, Object Type Table Data and Type Include; every such Rel. row has App ID zero, Related App ID zero, Related Scope Tenant and Type Include, and names another stored set with App ID zero on its other side; a DeleteAll selects one such set; no bound subscriber of the delete events of the three tables, no delete trigger of the set table, the database delete trigger known off for the three, and a session whose permission sets grant everything. Not told apart: the Object ID and the permission levels of the rows, the Security Filter, the number of rows, and a DeleteAll that also selects sets no row names. A Truncate of the set table and every other shape where such rows exist is refused.",
    tests: "System Application tests, Permission Import Export Tests (ExportImportTenantPermissionSet, ExportImportTenantPermissionSetWithAdditionalPermissions, ExportImportTenantPermissionSetUsingOverwriteOption) delete a tenant set with TenantPermissionSet.DeleteAll() and read its permissions back (al-runner issue 7854, the reporter's repro in its comment of 2026-11-13). The probe's values come from a local BC run on two majors and its green corpus CI on every supported major.",
    probe: "permissions/permission-sets/TenantSetDeleteCascade.al: Delete, Delete(true), DeleteAll, DeleteAll(true), RecordRef.Delete and RecordRef.DeleteAll of a set with its own Tenant Permission rows, a Rel. row to another set and one from another set; a set with another App ID; a bound Tenant Permission OnAfterDeleteEvent subscriber. Corpus pull request 1340.",
    awaiting: Awaiting {
        issues: &[7854],
        files: &["tests/al-language/permissions/permission-sets/TenantSetDeleteCascade.al"],
        since: "2026-11-14",
    },
}
