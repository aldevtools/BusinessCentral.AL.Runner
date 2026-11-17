# Hand-off: area-globallanguage-1117 (PR 8217) and area-globallanguage-1117b

## State
- area-globallanguage-1117: pushed, head e7318c6fb. Review round 3 fixes done. All gates green.
- area-globallanguage-1117b: pushed, head 3f08d8ade (from origin/main). Three wrong results fixed. All gates green. No pull request yet.

## Next steps
1. Update the PR 8217 body from /home/stefan/.cache/al-runtime/orchestration/body-8217-r3.md (gh pr edit was denied by the permission classifier).
2. Open a draft PR for area-globallanguage-1117b with the body in /home/stefan/.cache/al-runtime/orchestration/body-8217b.md.
3. After 8217 lands, merge main into 1117b. Expected conflicts:
   - crates/al-vm/src/records/measured_setfilter.rs (add/add): take 8217's file; it holds the same struct.
   - crates/al-vm/src/records/option_filter_error.rs (add/add): keep both functions in one file.
   - crates/al-builtins/src/date_formula_language.rs, evaluate(): put danish_letter_number_nine first, then 8217's checks.
   - crates/al-vm/src/records/op_inner.rs: keep both arms (OPTION_NOT_MEMBER and OPTION_NEGATIVE).
   - crates/al-vm/tests/language/global_language_kept_from_main.rs stays as is.
   - Then delete the PR 8217 table rows "kept from main, fixed in the second pull request".
4. Reword commit b8a575c62 at the 8217 train merge.

## Open
- 1117b: a compound assignment of a closing date and the Integer 1 reaches the closing-date entry (arith cannot see its caller).
- 1117b: 1T and 1J with number 9 under 1030 are measured false; main refuses them; not answered.
