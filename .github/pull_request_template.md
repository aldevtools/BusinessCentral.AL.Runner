<!-- See CONTRIBUTING.md. If this fixes an issue, keep the first line as "Fixes #N". -->
Fixes #

## What changed

## Evidence from real Business Central (required)
<!-- The runner only gives answers that real BC was measured to give. A pull request that changes what a test
sees is merged only with this evidence. Fill in ONE of these, for every new or changed answer:
  1. A corpus probe: link to your pull request on aldevtools/BusinessCentral.AL.Language.Tests (preferred; its CI
     measures BC 27, 28 and 29), or
  2. A test of yours that passes on real BC and needs exactly this behavior: the BC version, and the AL code of the
     repro (or a link to the issue that has it).
A result described only in a docs file, or measured on one BC version only, is not enough on its own.
Until the corpus pins it, the new path is registered as an assumption (crates/al-builtins/src/assumptions/<id>.rs)
so tests through it are reported as flagged passes. -->
- Probe PR / repro:
- BC versions measured:

## Where the behavior is pinned
<!-- Corpus tests (file:line) that pin each behavior, and the assumption id you added, if any. -->

## Shapes that keep today's behavior
<!-- Neighbor shapes the evidence does not cover (other types, triggers, extensions, event subscribers, other call
forms) must keep main's result or refuse. List the ones you checked and the tests that show it. -->

## Checks
- [ ] A test that failed before this change and passes now
- [ ] Tests that neighbor shapes keep main's result
- [ ] `al-test-cli --root . --baseline-diff`: no regression; stale lines deleted (`--prune-not-passing`) (or: not run, and why)
- [ ] `./check.sh` (or: not run, and why)
- [ ] I signed the contributor agreement (see CONTRIBUTING.md)
- [ ] No machine names, URLs, credentials, tenant ids or customer code in the change or this text
