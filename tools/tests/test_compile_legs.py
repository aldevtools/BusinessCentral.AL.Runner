#!/usr/bin/env python3
"""Test of tools/corpus-writer/compile-legs.sh with a fake compiler: python3 tools/tests/test_compile_legs.py

Each leg sets the symbols and runtime corpus CI uses (27 -> 16.1, 28 -> 17.1; 29 keeps app.json's), for the corpus app and
for every fixture app. The compiler pin and the fixture list (app_dirs) come from the "test" job of the corpus ci.yml; a
missing pin or list is exit 2. An error line (also one without ": " before it), a failing fixture, a missing compiler or a
bad EXTRA_PACKAGES folder is an error, not "ok". COMPILE_LEGS_TOOL overrides the script under test.
"""
import json, os, stat, subprocess, tempfile, unittest

TOOL = os.environ.get("COMPILE_LEGS_TOOL") or os.path.join(os.path.dirname(__file__), "..", "corpus-writer", "compile-legs.sh")
FIXTURES = ["al-language-internals-fixture", "al-language-xapp-a", "al-language-xapp-d", "al-language-xapp-b", "al-language-xapp-c",
            "al-language-xapp-n", "al-language-xapp-t", "al-language-xapp-e"]


def ci_yml(pin='"30.0.43.45776-beta"', dirs=FIXTURES):
    """A ci.yml with a decoy job before and after the "test" job: only the "test" job's facts count."""
    d = "".join("        tests/%s\n" % x for x in dirs) + "        tests/al-language\n" * 0
    app = "      app_dirs: |\n" + d + '      test_app_dirs: "tests/al-language"\n' if dirs else '      test_app_dirs: "tests/al-language"\n'
    pn = "      al_tool_version:      %s\n" % pin if pin else ""
    return ("jobs:\n  prepare:\n    with:\n      al_tool_version: \"1.1.1\"\n      app_dirs: |\n        tests/decoy\n"
            "  test:\n    with:\n" + app + pn + "  test-onprem:\n    with:\n      al_tool_version: \"2.2.2\"\n      app_dirs: |\n        tests/decoy2\n")

# The fake alc: records name, runtime and symbols of app.json per call, writes the /out file, lists the package cache of the
# corpus app's compile into FAKE_LS, then prints what FAKE_OUT says (FAKE_FIXTURE_OUT for the fixtures; FAKE_FAIL_FIXTURE names one).
ALC = """#!/usr/bin/env bash
python3 -c 'import json,sys; d=json.load(open("app.json")); print(d["name"], d["runtime"], ",".join(d.get("preprocessorSymbols", [])))' >> "$FAKE_LOG"
for a in "$@"; do case "$a" in /out:*) touch "${a#/out:}";; /packagecachepath:*) pk="${a#/packagecachepath:}";; esac; done
[ "$(basename "$PWD")" = "al-language" ] && ls "$pk" > "$FAKE_LS"
[ -n "$FAKE_FIXTURE_OUT" ] && [ "$(basename "$PWD")" != "al-language" ] && { printf '%s\n' "$FAKE_FIXTURE_OUT"; exit 0; }
[ -n "$FAKE_FAIL_FIXTURE" ] && case "$PWD" in *"$FAKE_FAIL_FIXTURE") printf 'error AL0001: boom\n'; exit 1;; esac
printf '%s\\n' "$FAKE_OUT"
exit "${FAKE_EXIT:-0}"
"""


def read(path):
    with open(path) as f:
        return f.read()


class T(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        app = os.path.join(self.d, "corpus", "tests", "al-language")
        os.makedirs(app)
        with open(os.path.join(app, "app.json"), "w") as f:
            json.dump({"name": "main", "runtime": "18.0", "preprocessorSymbols": []}, f)
        for fx in FIXTURES:
            os.makedirs(os.path.join(self.d, "corpus", "tests", fx))
            with open(os.path.join(self.d, "corpus", "tests", fx, "app.json"), "w") as f:
                json.dump({"name": fx, "publisher": "P", "runtime": "18.0"}, f)
        self.write_ci(ci_yml())
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

    def write_ci(self, text):
        wf = os.path.join(self.d, "corpus", ".github", "workflows")
        os.makedirs(wf, exist_ok=True)
        with open(os.path.join(wf, "ci.yml"), "w") as f:
            f.write(text)

    def run_tool(self, out, code=0, fixture_out="", bclinux=None, **extra):
        env = dict(os.environ, CORPUS=os.path.join(self.d, "corpus"), BCLINUX=bclinux or os.path.join(self.d, "bc-linux"),
                   FAKE_LOG=self.log, FAKE_OUT=out, FAKE_FIXTURE_OUT=fixture_out, FAKE_EXIT=str(code),
                   FAKE_LS=os.path.join(self.d, "ls"))
        env.update({"TMPDIR": self.d, **extra})
        return subprocess.run(["bash", TOOL], env=env, capture_output=True, text=True)

    def test_each_leg_gets_the_runtime_of_its_compiler(self):
        r = self.run_tool("Compilation ended successfully")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        with open(self.log) as f:
            lines = f.read().split("\n")
        main = [l for l in lines if l.startswith("main ")]
        self.assertEqual(main, ["main 16.1 BC27PLUS", "main 17.1 BC27PLUS,BC28PLUS", "main 18.0 BC27PLUS,BC28PLUS,BC29PLUS"])
        # every leg compiled the eight fixture apps of ci.yml first, in its order, with the symbols and runtime of the leg
        fx = [l for l in lines if l.startswith("al-language-")]
        self.assertEqual(len(fx), 24)
        self.assertEqual([l.split()[0] for l in fx], FIXTURES * 3)
        self.assertEqual([l.split()[1] for l in fx], ["16.1"] * 8 + ["17.1"] * 8 + ["18.0"] * 8)
        self.assertEqual([l.split()[2] for l in fx], ["BC27PLUS"] * 8 + ["BC27PLUS,BC28PLUS"] * 8 + ["BC27PLUS,BC28PLUS,BC29PLUS"] * 8)
        app = os.path.join(self.d, "corpus", "tests", "al-language", "app.json")
        self.assertEqual(json.loads(read(app))["runtime"], "18.0")

    def test_every_leg_uses_the_compiler_ci_pins(self):
        r = self.run_tool("Compilation ended successfully")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        with open(os.path.join(self.d, "install-args")) as f:
            self.assertEqual(f.read().split("\n")[:3], ["27.0 30.0.43.45776-beta", "28.0 30.0.43.45776-beta", "29.0 30.0.43.45776-beta"])

    def pins(self, pin):
        self.write_ci(ci_yml(pin=pin))
        r = self.run_tool("Compilation ended successfully")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        with open(os.path.join(self.d, "install-args")) as f:
            return f.read().split("\n")[:3]

    def test_a_bare_or_single_quoted_pin_is_passed_through(self):
        self.assertEqual(self.pins("30.0.43.45776-beta"), ["27.0 30.0.43.45776-beta", "28.0 30.0.43.45776-beta", "29.0 30.0.43.45776-beta"])
        os.remove(os.path.join(self.d, "install-args"))
        self.assertEqual(self.pins("'31.0.1'  # newest"), ["27.0 31.0.1", "28.0 31.0.1", "29.0 31.0.1"])

    def test_a_missing_pin_is_exit_2_before_any_leg(self):
        self.write_ci(ci_yml(pin=""))
        r = self.run_tool("Compilation ended successfully")
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        self.assertIn("no al_tool_version in", r.stdout)
        self.assertIn("cannot pick CI's compiler", r.stdout)
        self.assertNotIn(": ok", r.stdout)
        self.assertFalse(os.path.exists(os.path.join(self.d, "install-args")))
        self.assertEqual(json.loads(read(os.path.join(self.d, "corpus", "tests", "al-language", "app.json")))["runtime"], "18.0")
        self.assertFalse(os.path.exists(os.path.join(self.d, "corpus", "tests", "al-language", "app.json.legs-bak")))

    def test_a_missing_ci_yml_is_exit_2(self):
        os.remove(os.path.join(self.d, "corpus", ".github", "workflows", "ci.yml"))
        r = self.run_tool("Compilation ended successfully")
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        self.assertNotIn(": ok", r.stdout)

    def test_missing_app_dirs_is_exit_2(self):
        self.write_ci(ci_yml(dirs=[]))
        r = self.run_tool("Compilation ended successfully")
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        self.assertIn("no app_dirs", r.stdout)
        self.assertNotIn(": ok", r.stdout)

    def test_the_last_app_dir_failing_fails_every_leg(self):
        r = self.run_tool("Compilation ended successfully", FAKE_FAIL_FIXTURE="al-language-xapp-e")
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn("al-language-xapp-e", r.stdout)
        self.assertNotIn(": ok", r.stdout)

    def test_an_extra_packages_folder_without_apps_is_an_error(self):
        os.makedirs(os.path.join(self.d, "empty dir"))
        r = self.run_tool("Compilation ended successfully", EXTRA_PACKAGES=os.path.join(self.d, "empty dir") + ":" + os.path.join(self.d, "nope"))
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn("EXTRA_PACKAGES folder", r.stdout)
        self.assertNotIn(": ok", r.stdout)

    def test_an_extra_packages_folder_with_a_space_is_used(self):
        pd = os.path.join(self.d, "my pkgs")
        os.makedirs(pd)
        open(os.path.join(pd, "X.app"), "w").close()
        r = self.run_tool("Compilation ended successfully", EXTRA_PACKAGES=pd)
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertIn("X.app", read(os.path.join(self.d, "ls")))

    def test_the_installer_reason_is_shown(self):
        scripts = os.path.join(self.d, "empty", "scripts")
        os.makedirs(scripts)
        with open(os.path.join(scripts, "install-al-compiler.sh"), "w") as f:
            f.write("echo 'nuget is down' >&2\necho nothing\n")
        r = self.run_tool("Compilation ended successfully", bclinux=os.path.join(self.d, "empty"))
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn("nuget is down", r.stdout)

    def test_a_stale_fixture_package_does_not_survive_the_build(self):
        pk = os.path.join(self.d, "corpus", "tests", "al-language", ".alpackages")
        os.makedirs(pk)
        for n in ["P_al-language-xapp-a_0.9.0.0.app", "Microsoft_System_1.0.app"]:
            open(os.path.join(pk, n), "w").close()
        r = self.run_tool("Compilation ended successfully")
        self.assertEqual(r.returncode, 0, r.stdout)
        ls = read(os.path.join(self.d, "ls")).split()
        self.assertNotIn("P_al-language-xapp-a_0.9.0.0.app", ls)
        self.assertIn("al-language-xapp-a.app", ls)
        self.assertIn("Microsoft_System_1.0.app", ls)

    def test_a_failing_mktemp_leaves_no_backup_in_the_corpus(self):
        r = self.run_tool("Compilation ended successfully", TMPDIR=os.path.join(self.d, "no-such-dir"))
        self.assertEqual(r.returncode, 2, r.stdout + r.stderr)
        app = os.path.join(self.d, "corpus", "tests", "al-language")
        self.assertFalse(os.path.exists(os.path.join(app, "app.json.legs-bak")))
        self.assertEqual(json.loads(read(os.path.join(app, "app.json")))["runtime"], "18.0")

    def test_a_ci_yml_job_other_than_test_does_not_count(self):
        r = self.run_tool("Compilation ended successfully")
        self.assertEqual(r.returncode, 0, r.stdout)
        with open(os.path.join(self.d, "install-args")) as f:
            self.assertNotIn("1.1.1", f.read())

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
