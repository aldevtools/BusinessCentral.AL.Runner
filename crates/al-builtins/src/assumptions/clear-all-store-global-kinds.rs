ClearAllStoreGlobalKinds => {
    id: "clear-all-store-global-kinds",
    assumed: "ClearAll() in a plain codeunit or a codeunit with a TableNo resets in place a global codeunit instance another variable shares, when that store is a plain codeunit without a TableNo whose globals are all of these kinds: Integer, BigInteger, Decimal, Boolean, Text, Code, Date, Time, DateTime, Guid, Option, Enum, Duration, Char, a record (temporary or not), RecordRef, FieldRef, KeyRef, Interface, Variant, a Label with text and no property, List of Integer, Text or an Interface, Dictionary of Integer to Text or to an Interface, JsonObject, JsonArray, TextBuilder, an array of one dimension of Integer, Text or such a codeunit, and a codeunit of such a store. Every global goes back to its declared value; a temporary record keeps its rows; a codeunit global, the codeunit an Interface global holds and each element of an array of codeunits is reset in place the same way and stays shared with the variables that hold it. In the plain owner the store was written before the ClearAll (a temporary table holds rows, or a global of it, or of a codeunit it holds, holds a write AL can read: a value other than its declared one, an open RecordRef, a FieldRef or KeyRef that points at a field or key, a JsonObject or JsonArray with a member, a TextBuilder with text); in the owner with a TableNo it was written or never called. No routine of the store runs, no BindSubscription binds it, and no record outside shares one of its temporary tables.",
    tests: "No Microsoft test passes on real BC through a shape the probe does not cover. Microsoft's \"Whse.-Activity-Post\" through \"Sales-Post\".ClearAllVariables and \"Reservation Management\" with \"Late Binding Management\" (al-runner issue 8964) are the shapes the probe models.",
    probe: "Merged corpus pull request 1587 (merge commit 6f3a204f), tests/al-language/codeunits/clear-instance/ClearSharedCodeunitGlobalKinds.al (86278, every global kind under five ways of sharing and two owners), ClearSharedCodeunitWhseRegister.al (the Sales-Post register shape) and ClearSharedCodeunitLateBinding.al (the Reservation Management shape), green on every BC version of its CI run. Not yet in the runner's corpus pin.",
    awaiting: Awaiting {
        issues: &[8964],
        files: &[
            "tests/al-language/codeunits/clear-instance/ClearSharedCodeunitGlobalKinds.al",
            "tests/al-language/codeunits/clear-instance/ClearSharedCodeunitWhseRegister.al",
            "tests/al-language/codeunits/clear-instance/ClearSharedCodeunitLateBinding.al",
        ],
        since: "2026-11-25",
    },
}
