ApiVersionListCommaJoined => {
    id: "api-version-list-comma-joined",
    assumed: "An API page or query that declares two or three versions (`APIVersion = 'v1.0', 'v2.0', 'beta';`) holds them in the APIVersion column of Page Metadata or Query Metadata in declared order, joined by a comma without spaces, and a filter on that column matches the joined text, not one version alone (al-vm platform/objects.rs `api_version_column`). Flagged: a read of such a row, and a filter on the APIVersion column. Four or more declared versions refuse.",
    tests: "No Microsoft test run on BC for this shape here. The reporter of al-runner issue 7818 read it from Microsoft's MCP module, which splits the column on ','; its tests MCP Config Test.TestGetHighestAPIVersionMultipleVersions and TestCreateAPIToolSetsAPIVersion are shipped as passing.",
    probe: "Corpus PR 1602, tests/al-language/pages/properties/PageMetadataMultiApiVersion.al: three versions on a page (PageMetadata_ThreeVersions_InDeclaredOrder), two in descending order (PageMetadata_DescendingVersions_InDeclaredOrder), a filter on the full list and on the first version (PageMetadata_FilterOnFullList), three versions on a query (QueryMetadata_ThreeVersions_InDeclaredOrder).",
    awaiting: Awaiting {
        issues: &[7818],
        files: &["tests/al-language/pages/properties/PageMetadataMultiApiVersion.al"],
        since: "2026-11-26",
    },
}
