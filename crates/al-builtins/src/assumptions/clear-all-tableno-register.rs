ClearAllTableNoRegister => {
    id: "clear-all-tableno-register",
    assumed: "ClearAll() in a codeunit with a TableNo, or in a plain codeunit, resets in place a global codeunit instance another variable shares, when that store is a codeunit with a TableNo that is not SingleInstance and is not the register of clear-shared-codeunit, whose declared globals besides Rec are all of these kinds: a record that is not temporary, a Label with text and no property, a Label with text and a Comment and no other property, Integer, BigInteger, Decimal, Boolean, Text, Code, Date, Time, DateTime, Guid, Option, Enum, Duration, Char, and a codeunit that is not SingleInstance, has no TableNo and holds only Integers and such codeunits, nested at most four levels below the store, where a codeunit one level below the store may also hold the store itself. Every global goes back to its declared value; a codeunit global is reset in place the same way and stays shared, and a codeunit global that holds an instance already being reset keeps it as it is (the back link stays). In the owner with a TableNo the store was written, only read or never called before the ClearAll; in the plain owner it was written or never called. Its records read or wrote no row, no other variable shares its nested codeunit instances, no routine of the store runs, and no BindSubscription binds it.",
    tests: "No Microsoft test passes on real BC through a shape the probe does not cover. Microsoft's \"Whse.-Activity-Post\" through \"Sales-Post\".ClearAllVariables with the shared \"Whse. Jnl.-Register Line\" (al-runner issue 8964) is the shape the probe models.",
    probe: "Merged corpus pull request 1596 (merge commit d2552ce6), tests/al-language/codeunits/clear-instance/ClearSharedCodeunitTableNoRegister.al (test codeunit 86307: a TableNo register shared through a setter, written, only read, never called and run before the share, nested codeunits two to four levels deep, a cycle, owners with and without a TableNo), green on every BC version of its CI run. Not yet in the runner's corpus pin.",
    awaiting: Awaiting {
        issues: &[8964],
        files: &[
            "tests/al-language/codeunits/clear-instance/ClearSharedCodeunitTableNoRegister.al",
        ],
        since: "2026-11-26",
    },
}
