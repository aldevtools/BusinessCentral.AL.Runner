# Checks by area

Read this when your change touches the areas named below. Moved out of `brief.md`.

- **One build, one result.** A corpus run must give each case the same outcome whatever the
  thread counts, the run order and HOME hold (`corpus_harness.rs`
  `outcomes_do_not_depend_on_threads_or_home` checks it). Before you call two runs "the same
  build", compare their `binary:` lines: a rebuild after a golden file was regenerated (for
  example `al-builtins/src/pinned_columns.rs`) changes what is refused.

3. Run the release-only tests that cover your change. CI runs them nightly:
   `cargo test --release --no-fail-fast -p al-syntax --test perf_parse_throughput -p al-test-cli --test perf_budget --test perf_vm --test corpus_resolve_deps -p al-data --lib -- --ignored`
   If the golden list of unresolved items changes, review the change and
   justify it in your report before you regenerate it with `UPDATE_GOLDEN=1`.
4. Run the demand/eager differential if you touched dependency loading,
   demand-driven compile, lowering, the checker or anything a Microsoft app
   reaches. When unsure, run it:
   `tools/demand_vs_eager.sh run`
   It takes about 3 minutes and needs `python3`. It compares a normal run with
   an eager run (`AL_EAGER_DEPENDENCIES=1`). Any difference fails, and
   `tools/cache_vs_uncached.sh run` does the same for the compile cache. Owner
   decision of 2026-10-14: there are zero differences.
   `tools/demand-eager-known-differences.txt` is empty and stays empty; never
   add a line to make a run pass. Find the cause: `AL_BLOCK_PATH=1` prints the
   call chain from a test to the routine that refuses it, and
   `AL_DUMP_REACH=FILE` writes what the demand-driven compile reached, `AL_STACK_OF=TEXT` prints the routines a not-run test was refused in (for a
   diff between two runs). A static refusal may depend on the sources only,
   never on what was compiled or on which other tests are selected.
   If you touched `al-testrun/src/observe.rs` or the baseline, also run
   `tools/baseline_rule_check.sh run`: the rule that skips the install baseline
   must give zero outcome differences. If you touched the lazy reading of dependency packages
   (`al-deps/src/lazy.rs`, `Workspace::load_apps_lazy`), run `al-runner test` on Tests-ERM twice, once with
   `ALRUNNER_EAGER_SOURCES=1`, and compare the two `--junit` files with `tools/compare_junit.py`: 0 differences.
5. No corpus case may newly fail. A case that was not run and now fails counts
   as wrong behavior (hard rule 4); the lists do not catch it (a listed case may fail or be refused). Run
   `al-test-cli --root . --bc-version 27 --junit FILE` on `origin/main` and on
   your branch, and list every case that fails on the branch but not on main.
   Fix each one or refuse its path. Before each commit, check
   `git submodule status` (the corpus must stay at the oracle's
   `corpus_commit`), and stage explicit paths: never `git add -A` or
   `git commit -a`.
