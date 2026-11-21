# Recurring review findings

What review raised again and again; the count is hand-backs with the finding, of 34 up to 2026-11-12. The implementer
checks its diff against this list before hand-back; the reviewer checks it first. Examples and full lists, same
numbers: `recurring-findings-examples.md`. Read an item's examples when your change touches it. A finding seen in two
or more pull requests gets an item here and there, in the change that fixes it.

1. **A new rule runs on shapes no case pins** (29). One allowlist predicate at the owner: every pinned fact by equality, each cited. Flip one fact at a time, one test each, in a neighbor app run by `tools/neighbor-diff.sh`.
2. **A shape main answered now refuses or answers differently** (13). Neighbors keep main's result (a neighbor app and Rust tests). A refusal is only as wide as its reason.
3. **Text that does not match the code or CI** (comments 13, PR body or ADR 9). Read comments, PR body and area table against the final diff and the real CI run.
4. **More than one owner** (10). Put a rule at the one site its pin covers. Call the owner. Match by kind or raise site, not text.
5. **New refusals without a Rust test** (10). A Rust test or a corpus case reaches every new refusal.
6. **Assumption registry and probe rows** (registry or row 7, flag lost 5). Each entry clause has a gate line. Flag every pass. Marks survive every copy.
7. **State that outlives its shape** (6). Clear a flag on every move, close, host change and exit. Key values by their run or session.
7a. **A value stamped at a measured landing outlives the landing.** Keep it off the record except while the measured read runs.
8. **Citations** (6). `file:line` in the pinned corpus, same move and kinds. An unpinned probe is an assumption entry.
9. **Caps, budgets and ratchets** (5). Never raise one. Measure a hot-path cost before and after.
10. **Checks run on one major only, or on an old binary** (4). `--baseline-diff` on BC 27, 28 and 29 with the head binary.
11. **The answer says more than the pin** (4). No exact value where only the sign is pinned. No composed error text.
12. **Unknown reach counted as safe** (3). Unknown entries in a reach analysis fail a "nothing else sees this" check.
13. **Microsoft text in fixtures.** Write stand-ins from the pinned behavior. Only signatures may match the source.
14. **A shape the checker now lets compile, followed only to the checker.** Follow it into lowering and run it.
15. **"X members are pinned" while the gate takes the whole type.** Name the measured values and gate on them.
16. **A restore applied because code stopped.** Require the set-back in the stopped routine, or refuse the readers.
17. **A gate inside a trigger, checked by the frame stack only.** Require the starting method, RunTrigger and temporary state.
18. **Session state set by a routine the runner stopped.** Give it the new rule only when the routine returns Ok.
19. **An outside report's shape differs in several facts, and the gate reads one.** Require every differing fact.
20. **Rows the runner does not know have more readers than the test.** Refuse every kind of reader, in one owner.
21. **A mark set on the first read only.** Mark the first reader and every later test. A keeping write is a read.
22. **Code moved to a session of its own.** One predicate names every `Session` field (no `..`). Probe each.
23. **A workflow or action change without the Rust tests that read workflow text.** Grep `crates/al-test-cli/tests/suite/ci_*.rs` for the changed step text; run `cargo test -p al-test-cli --test suite ci_`.
24. **A corpus bump deletes an assumption entry, and its unmeasured shapes keep running.** Every shape the entry listed as assumed beyond or still to measure refuses after the bump, unless the new pin measures it.
25. **A gate compares names, not bodies.** A trigger, procedure, subscriber, codeunit or extension name says nothing about what runs. A gate that compares names also compares the bodies line for line with the pin (`pinned_body.rs`), callees included, or it refuses.
26. **A value kind a new lowering builds reaches every reader of the old kind.** Run it through the readers main used (field, Text[n], Code[n], list, filter, FieldRef, Evaluate, StrSubstNo) against a main binary. A new mid-run refusal is a regression.
27. **A gate reads a state field as "the start of the test".** A field that only some events change also reads "at start" after other events (a commit, a rollback, a set back to the start value). Gate on the pinned moment itself, and name every event that ends it.
28. **A tooling gate that reads a command's empty output as "nothing changed".** Require that the command exits 0 (and that its path is inside the repository) before an empty output relaxes a check. Example: `neighbor-diff.sh` stale-entry note.
