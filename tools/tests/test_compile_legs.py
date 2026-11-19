#!/usr/bin/env python3
"""Test of tools/corpus-writer/compile-legs.sh with a fake compiler: python3 tools/tests/test_compile_legs.py

al-runner issue 8274: each leg sets the runtime its compiler supports (27 -> 16.1, 28 -> 17.1, as corpus CI does; 29
keeps app.json's), and an error line without ": " before it (`error AL1022`, a missing package) is an error, not "ok".
"""
import json, os, stat, subprocess, tempfile, unittest

TOOL = os.path.join(os.path.dirname(__file__), "..", "corpus-writer", "compile-legs.sh")

# The fake alc: records the runtime and the symbols of app.json per call, then prints what FAKE_OUT says.
ALC = """#!/usr/bin/env bash
python3 -c 'import json,sys; d=json.load(open("app.json")); print(d["name"], d["runtime"], ",".join(d.get("preprocessorSymbols", [])))' >> "$FAKE_LOG"
[ -n "$FAKE_FIXTURE_OUT" ] && [ "$(basename "$PWD")" != "al-language" ] && { printf '%s\n' "$FAKE_FIXTURE_OUT"; exit 0; }
printf '%s\\n' "$FAKE_OUT"
exit "${FAKE_EXIT:-0}"
"""


class T(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        app = os.path.join(self.d, "corpus", "tests", "al-language")
        os.makedirs(app)
        with open(os.path.join(app, "app.json"), "w") as f:
            json.dump({"name": "main", "runtime": "18.0", "preprocessorSymbols": []}, f)
        for fx in ["al-language-internals-fixture", "al-language-xapp-a", "al-language-xapp-d", "al-language-xapp-b", "al-language-xapp-c"]:
            os.makedirs(os.path.join(self.d, "corpus", "tests", fx))
            with open(os.path.join(self.d, "corpus", "tests", fx, "app.json"), "w") as f:
                json.dump({"name": fx, "runtime": "18.0"}, f)
        subprocess.run(["git", "init", "-q", os.path.join(self.d, "corpus")], check=True)
        tool = os.path.join(self.d, "alc")
        os.makedirs(tool)
        with open(os.path.join(tool, "alc"), "w") as f:
            f.write(ALC)
        os.chmod(os.path.join(tool, "alc"), stat.S_IRWXU)
        scripts = os.path.join(self.d, "bc-linux", "scripts")
        os.makedirs(scripts)
        with open(os.path.join(scripts, "install-al-compiler.sh"), "w") as f:
            f.write(f"echo \"$@\" >> {os.path.join(self.d, 'install-args')}\necho AL_TOOL_DIR={tool}\n")
        self.log = os.path.join(self.d, "log")

    def run_tool(self, out, code=0, fixture_out="", bclinux=None):
        env = dict(os.environ, CORPUS=os.path.join(self.d, "corpus"), BCLINUX=bclinux or os.path.join(self.d, "bc-linux"),
                   FAKE_LOG=self.log, FAKE_OUT=out, FAKE_FIXTURE_OUT=fixture_out, FAKE_EXIT=str(code), TMPDIR=self.d)
        return subprocess.run(["bash", TOOL], env=env, capture_output=True, text=True)

    def test_each_leg_gets_the_runtime_of_its_compiler(self):
        r = self.run_tool("Compilation ended successfully")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        with open(self.log) as f:
            lines = f.read().split("\n")
        main = [l for l in lines if l.startswith("main ")]
        self.assertEqual(main, ["main 16.1 BC27PLUS", "main 17.1 BC27PLUS,BC28PLUS", "main 18.0 BC27PLUS,BC28PLUS,BC29PLUS"])
        # every leg compiled the five fixture apps first, with the runtime of its compiler
        fx = [l for l in lines if l.startswith("al-language-")]
        self.assertEqual(len(fx), 15)
        self.assertEqual([l.split()[1] for l in fx], ["16.1"] * 5 + ["17.1"] * 5 + ["18.0"] * 5)
        app = os.path.join(self.d, "corpus", "tests", "al-language", "app.json")
        self.assertEqual(json.load(open(app))["runtime"], "18.0")

    def test_every_leg_uses_the_compiler_ci_pins(self):
        wf = os.path.join(self.d, "corpus", ".github", "workflows")
        os.makedirs(wf)
        with open(os.path.join(wf, "ci.yml"), "w") as f:
            f.write('jobs:\n  a:\n    with:\n      al_tool_version:      "30.0.43.45776-beta"\n')
        r = self.run_tool("Compilation ended successfully")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        with open(os.path.join(self.d, "install-args")) as f:
            self.assertEqual(f.read().split("\n")[:3], ["27.0 30.0.43.45776-beta", "28.0 30.0.43.45776-beta", "29.0 30.0.43.45776-beta"])

    def test_an_error_without_a_colon_before_it_is_an_error(self):
        r = self.run_tool("error AL1022: The package 'X' could not be found")
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertNotIn(": ok", r.stdout)

    def test_a_compiler_that_fails_without_an_error_line_is_an_error(self):
        r = self.run_tool("something went wrong", code=1)
        self.assertEqual(r.returncode, 1, r.stdout)

    def test_a_fixture_app_error_fails_the_leg_and_names_the_fixture(self):
        r = self.run_tool("Compilation ended successfully", fixture_out="error AL1043: runtime not supported")
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn("fixture app al-language-internals-fixture", r.stdout)
        self.assertNotIn(": ok", r.stdout)

    def test_a_missing_compiler_is_an_error_not_ok(self):
        scripts = os.path.join(self.d, "empty", "scripts")
        os.makedirs(scripts)
        with open(os.path.join(scripts, "install-al-compiler.sh"), "w") as f:
            f.write("echo nothing\n")
        r = self.run_tool("Compilation ended successfully", bclinux=os.path.join(self.d, "empty"))
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn("no AL compiler found", r.stdout)
        self.assertNotIn(": ok", r.stdout)

    def test_a_located_error_is_still_an_error(self):
        r = self.run_tool("src/X.al(1,1): error AL0118: The name 'Y' does not exist")
        self.assertEqual(r.returncode, 1, r.stdout)


if __name__ == "__main__":
    unittest.main()
