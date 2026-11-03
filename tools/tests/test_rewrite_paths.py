#!/usr/bin/env python3
"""Test of tools/rewrite_paths.py on a hand-made path map: python3 tools/tests/test_rewrite_paths.py"""
import os, subprocess, sys, tempfile, unittest

TOOL = os.path.join(os.path.dirname(__file__), "..", "rewrite_paths.py")

MAP = """old_path\tnew_path\thow
tests/al-language/record/lt-y66-flow/Y66Flow.al\ttests/al-language/record/flowfields/Y66Flow.al\tmap
tests/al-language/record/lt-y66-flow/Y66Fix.al\ttests/al-language/record/flowfields/Y66Fix.al\tmap
tests/al-language/handlers/TestModal.al\ttests/al-language/ui-handlers/TestModal.al\tmap
tests/al-language/handlers/TestPagePart.al\ttests/al-language/testpage/TestPagePart.al\tmap
tests/al-language-onprem/handlers/TestModal.al\ttests/al-language-onprem/reports/TestModal.al\tmap
tests/al-language-onprem-dotnet/lt-qx-dotnet/QxShapes.al\ttests/al-language-onprem-dotnet/dotnet/qx/QxShapes.al\tmap
tests/al-language/record/TestRecordInsert.al\ttests/al-language/record/TestRecordInsert.al\tmap
tests/al-language/bc-full-results.json\t(delete: build output)\trule
tests/al-language/codeunit/Cu.al\ttests/al-language/codeunits/Cu.al\tmap
"""


class T(unittest.TestCase):
    def run_tool(self, d, *extra):
        return subprocess.run([sys.executable, TOOL, "--map", os.path.join(d, "path-map.tsv"), *extra],
                              capture_output=True, text=True, cwd=d)

    def setUp(self):
        self.d = tempfile.mkdtemp()
        open(os.path.join(self.d, "path-map.tsv"), "w").write(MAP)
        os.makedirs(os.path.join(self.d, "crates/x/src"))
        os.makedirs(os.path.join(self.d, "docs"))

    def write(self, rel, text):
        open(os.path.join(self.d, rel), "w").write(text)

    def read(self, rel):
        return open(os.path.join(self.d, rel)).read()

    def test_full_app_and_relative_forms(self):
        self.write("crates/x/src/a.rs",
                   "// pinned by corpus/tests/al-language/record/lt-y66-flow/Y66Flow.al:42 and\n"
                   "// tests/al-language/record/lt-y66-flow/Y66Fix.al:7\n"
                   'files: &["al-language-onprem-dotnet/lt-qx-dotnet/QxShapes.al", "record/lt-y66-flow/Y66Flow.al"],\n'
                   "// folder corpus/tests/al-language/record/lt-y66-flow/ and record/lt-y66-flow\n")
        r = self.run_tool(self.d, "crates")
        self.assertEqual(r.returncode, 0, r.stderr)
        a = self.read("crates/x/src/a.rs")
        self.assertIn("corpus/tests/al-language/record/flowfields/Y66Flow.al:42", a)
        self.assertIn("tests/al-language/record/flowfields/Y66Fix.al:7", a)
        self.assertIn('"al-language-onprem-dotnet/dotnet/qx/QxShapes.al"', a)
        self.assertIn('"record/flowfields/Y66Flow.al"', a)
        self.assertIn("folder corpus/tests/al-language/record/flowfields/ and record/flowfields\n", a)
        self.assertNotIn("lt-y66", a)

    def test_ambiguous_relative_path_is_left_and_reported(self):
        # handlers/TestModal.al exists in two apps with different targets: no guess
        self.write("docs/b.md", "see handlers/TestModal.al:3 and tests/al-language-onprem/handlers/TestModal.al\n")
        r = self.run_tool(self.d, "docs")
        b = self.read("docs/b.md")
        self.assertIn("see handlers/TestModal.al:3", b)
        self.assertIn("tests/al-language-onprem/reports/TestModal.al", b)
        self.assertIn("ambiguous", r.stdout)
        self.assertIn("handlers/TestModal.al", r.stdout)

    def test_split_folder_is_not_rewritten(self):
        # handlers/ goes to two areas: a citation of the folder itself has no single target
        self.write("docs/c.md", "files in corpus/tests/al-language/handlers/ moved\n")
        self.run_tool(self.d, "docs")
        self.assertEqual(self.read("docs/c.md"), "files in corpus/tests/al-language/handlers/ moved\n")

    def test_no_partial_match_and_unchanged_rows(self):
        text = ("tests/al-language/record/lt-y66-flow/Y66Flow.alx\n"
                "xtests/al-language/record/lt-y66-flow/Y66Flow.al\n"
                "tests/al-language/record/TestRecordInsert.al:9\n")
        self.write("docs/d.md", text)
        self.run_tool(self.d, "docs")
        self.assertEqual(self.read("docs/d.md"), text)

    def test_unresolved_citations_are_reported(self):
        self.write("docs/e.md", "stale corpus/tests/al-language/record/Gone.al:1\n")
        r = self.run_tool(self.d, "docs")
        self.assertIn("unresolved", r.stdout)
        self.assertIn("tests/al-language/record/Gone.al", r.stdout)

    def test_plain_words_are_not_folders(self):
        # the top folder codeunit/ moved as a whole to codeunits/: the English word must stay
        text = "a call in a codeunit; see codeunit/Cu.al\n"
        self.write("docs/h.md", text)
        self.run_tool(self.d, "docs")
        self.assertEqual(self.read("docs/h.md"), "a call in a codeunit; see codeunits/Cu.al\n")

    def test_check_mode_writes_nothing_and_fails(self):
        self.write("docs/f.md", "tests/al-language/handlers/TestPagePart.al\n")
        r = self.run_tool(self.d, "--check", "docs")
        self.assertEqual(r.returncode, 1)
        self.assertEqual(self.read("docs/f.md"), "tests/al-language/handlers/TestPagePart.al\n")

    def test_second_run_changes_nothing(self):
        self.write("docs/g.md", "tests/al-language/handlers/TestPagePart.al and record/lt-y66-flow/Y66Fix.al\n")
        self.run_tool(self.d, "docs")
        once = self.read("docs/g.md")
        r = self.run_tool(self.d, "--check", "docs")
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertEqual(self.read("docs/g.md"), once)
        self.assertEqual(once, "tests/al-language/testpage/TestPagePart.al and record/flowfields/Y66Fix.al\n")


if __name__ == "__main__":
    unittest.main()
