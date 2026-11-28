ApiVersionListCommaJoined => {
    id: "api-version-list-comma-joined",
    assumed: "An API page or query that declares two, three or four versions (`APIVersion = 'v1.0', 'v2.0', 'beta';`) holds them in the APIVersion column of Page Metadata or Query Metadata in declared order, joined by a comma without spaces; Format and FieldRef.Value give the same text. SetRange and SetFilter on that column match the joined text: equality on the whole list, wildcards and OR filters against it, not one version alone (al-vm platform/objects.rs `api_version_column`). Flagged: a read of such a row, and a filter on the APIVersion column. Five or more declared versions refuse.",
    tests: "No Microsoft test run on BC for this shape here. The reporter of al-runner issue 7818 read it from Microsoft's MCP module, which splits the column on ','; its tests MCP Config Test.TestGetHighestAPIVersionMultipleVersions and TestCreateAPIToolSetsAPIVersion are shipped as passing.",
    probe: "Corpus PR 1602 (merged on corpus master, not yet in the pin), tests/al-language/pages/properties/PageMetadataMultiApiVersion.al: page three versions :98 (PageMetadata_ThreeVersions_InDeclaredOrder), page two descending :112, page SetRange full list and first version :124, query three versions :140, query two versions :214 (QueryMetadata_TwoVersions), query Format and StrLen :230, query SetRange full list and one version :244, page SetFilter '*v2.0*', 'v1.0*', '*beta', '*v1.0', 'v2.0*' and OR filters :270, query SetFilter wildcards and OR :305, page Format, FieldRef.Value and FieldRef.Length :330, page four versions with SetRange and '*v3.0*' :350, query four versions with SetRange :374.",
    awaiting: Awaiting {
        issues: &[7818],
        files: &["tests/al-language/pages/properties/PageMetadataMultiApiVersion.al"],
        since: "2026-11-26",
    },
}
