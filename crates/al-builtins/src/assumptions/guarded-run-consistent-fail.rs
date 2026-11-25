GuardedRunConsistentFail => {
    id: "guarded-run-consistent-fail",
    assumed: "How a guarded Codeunit.Run ends with Consistent(false) marks in the shapes of corpus PR 1586 that guarded-run-consistent-end does not take (al-vm `consistent_marks.rs`, `fail_probe_end`). The run is Codeunit.Run(Id) without a record argument, not nested in another guarded run, with no mark when it starts and no Commit since it started. A mark that another database variable marks again with Consistent(false), after the setter's mark or after another variable's Consistent(true), stays marked when a database variable then calls Consistent(true) on it. The run fails with code Dialog and ends with two marks: held then marked again, released then held, or marked again then held: it returns false with the inner error and the marks are gone. The run fails with code NCLCSRTS:TableErrorStr (a FieldError) and ends with one held mark: it returns false with the inner error and the mark is gone. The run succeeds and ends with held then marked again, marked again then held, or one mark marked again: it raises DB:TableInConsistent naming the table marked first. Refused: any other list of marks, codes or run arguments, a nested run, a mark at the start, a Commit since the start, a Commit while a mark is marked again, a temporary variable in the sequence.",
    tests: "Microsoft's posting preview tests (the preview state table and a second table marked through several record variables inside the posting run, which then fails on purpose), warehouse and inventory pick previews, and a G/L test whose posting run marks the G/L Entry table and fails on a FieldError (al-runner issue 8970).",
    probe: "Corpus PR 1586 (merged as 091453a7), tests/al-language/errors/ConsistentGuardedRunFail.al, codeunit 88974 \"GRF Tests\", section (2): the cells named at `fail_probe_end`, green in the PR's CI on BC 27.5, 28.5 and 29.0.",
    awaiting: Awaiting {
        issues: &[8970],
        files: &["tests/al-language/errors/ConsistentGuardedRunFail.al"],
        since: "2026-11-25",
    },
}
