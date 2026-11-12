TenantPermissionSetDeleteCascade => {
    id: "tenant-permission-set-delete-cascade",
    assumed: "A Delete, Delete(true), DeleteAll, DeleteAll(true), RecordRef.Delete or RecordRef.DeleteAll of a stored Tenant Permission Set row with App ID zero deletes its Tenant Permission rows (Object Type Table Data) and the Tenant Permission Set Rel. rows of the set and of other sets that include it (App ID and Related App ID zero, Related Scope Tenant, Type Include), in the store after the set row's delete; a DeleteAll of a set whose App ID is not zero deletes no permission. Other row kinds, a Delete of a set whose App ID is not zero with permissions, and delete events or database triggers on Tenant Permission, Tenant Permission Set Rel. or the set's OnAfterDeleteEvent are refused.",
    tests: "The probe's result on a local BC 27.5, 28.5 and 29.0 (TSDC_* tests of TenantSetDeleteCascade.al); System Application tests, Permission Import Export Tests (PermissionImportExportTests.Codeunit.al:282, :383 with the asserts after them) need it (al-runner issue 7854).",
    probe: "permissions/permission-sets/TenantSetDeleteCascade.al: Delete, Delete(true), DeleteAll, DeleteAll(true), RecordRef.Delete and RecordRef.DeleteAll of a Tenant Permission Set with Tenant Permission and Tenant Permission Set Rel. rows of its own and of another set; DeleteAll of a set whose App ID is not zero; a bound Tenant Permission OnAfterDeleteEvent subscriber during a set DeleteAll. Corpus pull request PRNUM.",
    awaiting: Awaiting {
        issues: &[7854],
        files: &["tests/al-language/permissions/permission-sets/TenantSetDeleteCascade.al"],
        since: "2026-11-12",
    },
}
