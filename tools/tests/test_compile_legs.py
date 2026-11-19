#!/usr/bin/env python3
"""Test of tools/corpus-writer/compile-legs.sh with a fake compiler: python3 tools/tests/test_compile_legs.py

al-runner issue 8274: each leg sets the runtime its compiler supports (27 -> 16.1, 28 -> 17.1, as corpus CI does; 29
keeps app.json's), and an error line without ": " before it (`error AL1022`, a missing package) is an error, not "ok".
"""
import json, os, stat, subprocess, tempfile, unittest

TOOL = os.path.join(os.path.dirname(__file__), "..", "corpus-writer", "compile-legs.sh")

# The fake alc: records the runtime and the symbols of app.json per call, then prints what FAKE_OUT says.
ALC = """#!/usr/bin/env bash
python3 -c 'import json,sys; d=json.load(open("app.json")); print(d["runtime"], ",".join(d["preprocessorSymbols"]))' >> "$FAKE_LOG"
printf '%s\\n' "$FAKE_OUT"
exit "${FAKE_EXIT:-0}"
"""


class T(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        app = os.path.join(self.d, "corpus", "tests", "al-language")
        os.makedirs(app)
        with open(os.path.join(app, "app.json"), "w") as f:
            json.dump({"runtime": "18.0", "preprocessorSymbols": []}, f)
        subprocess.run(["git", "init", "-q", os.path.join(self.d, "corpus")], check=True)
        tool = os.path.join(self.d, "alc")
        os.makedirs(tool)
        with open(os.path.join(tool, "alc"), "w") as f:
            f.write(ALC)
        os.chmod(os.path.join(tool, "alc"), stat.S_IRWXU)
        scripts = os.path.join(self.d, "bc-linux", "scripts")
        os.makedirs(scripts)
        with open(os.path.join(scripts, "install-al-compiler.sh"), "w") as f:
            f.write(f"echo AL_TOOL_DIR={tool}\n")
        self.log = os.path.join(self.d, "log")

    def run_tool(self, out, code=0):
        env = dict(os.environ, CORPUS=os.path.join(self.d, "corpus"), BCLINUX=os.path.join(self.d, "bc-linux"),
                   FAKE_LOG=self.log, FAKE_OUT=out, FAKE_EXIT=str(code), TMPDIR=self.d)
        return subprocess.run(["bash", TOOL], env=env, capture_output=True, text=True)

    def test_each_leg_gets_the_runtime_of_its_compiler(self):
        r = self.run_tool("Compilation ended successfully")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        with open(self.log) as f:
            lines = f.read().split("\n")
        self.assertEqual(lines[:3], ["16.1 BC27PLUS", "17.1 BC27PLUS,BC28PLUS", "18.0 BC27PLUS,BC28PLUS,BC29PLUS"])
        app = os.path.join(self.d, "corpus", "tests", "al-language", "app.json")
        self.assertEqual(json.load(open(app))["runtime"], "18.0")

    def test_an_error_without_a_colon_before_it_is_an_error(self):
        r = self.run_tool("error AL1022: The package 'X' could not be found")
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertNotIn(": ok", r.stdout)

    def test_a_compiler_that_fails_without_an_error_line_is_an_error(self):
        r = self.run_tool("something went wrong", code=1)
        self.assertEqual(r.returncode, 1, r.stdout)

    def test_a_located_error_is_still_an_error(self):
        r = self.run_tool("src/X.al(1,1): error AL0118: The name 'Y' does not exist")
        self.assertEqual(r.returncode, 1, r.stdout)


if __name__ == "__main__":
    unittest.main()
