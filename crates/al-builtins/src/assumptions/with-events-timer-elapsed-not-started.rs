WithEventsTimerElapsedNotStarted => {
    id: "with-events-timer-elapsed-not-started",
    assumed: "A global [WithEvents] System.Timers.Timer variable of a codeunit, a page, a table or a report whose one trigger is `Variable::Elapsed` runs as a plain DotNet variable while the code only constructs it without arguments, reads or assigns Interval, reads Enabled, and passes it to IsNull or Clear: the timer never starts, so the Elapsed trigger never runs. A report with such a variable runs (the trigger is no report trigger that the run calls). Any other use (Start, Enabled := true, passing or copying the variable, another member, ClearAll in the object or in an extension of it) is refused; a Disposed or any other trigger, and an xmlport or query, keep the refusal.",
    tests: "Corpus PR 1365 (open), tests/al-language-onprem-dotnet/dotnet/ac/N4WithEventsNoRaise.al: a codeunit (assigned, reassigned, read, cleared, used through a local instance), a page opened with TestPage.OpenView, a table procedure on two record variables and a ProcessingOnly report, on every BC major the corpus CI runs; Elapsed ran 0 times in every case. al-runner issue 7832 (an outside report).",
    probe: "N4WithEventsNoRaise.al (the cells above). Not measured: Start and the Elapsed raise, a Disposed trigger on a page, table or report, WithEvents on an xmlport or query.",
    awaiting: Awaiting {
        issues: &[7832],
        files: &["tests/al-language-onprem-dotnet/dotnet/ac/N4WithEventsNoRaise.al"],
        since: "2026-11-12",
    },
}
