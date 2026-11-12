TenantMediaDescriptionInsert => {
    id: "tenant-media-description-insert",
    assumed: "Record.Insert(false) stores a Tenant Media row with a nonzero GUID, description, and default remaining fields.",
    tests: "Local neutral probe DirectTenantMediaInsert.al, measured on all supported BC majors.",
    probe: "Add the measured case to the pinned corpus.",
    awaiting: Awaiting {
        issues: &[7780],
        files: &["tests/al-language/streams/media/Y48TenantMediaRows.al::Record_Insert_TenantMediaDescriptionOnly_ReadsBack"],
        since: "2026-11-11",
    },
}
