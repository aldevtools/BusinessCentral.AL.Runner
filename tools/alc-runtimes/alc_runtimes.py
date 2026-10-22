#!/usr/bin/env python3
"""Measure, with the real AL compiler, the runtime each builtin overload of the catalog needs.

Writes `crates/al-builtins/catalog/alc-runtimes.txt` (al-runner issue 5573). Nothing is decompiled: every number comes
from a diagnostic `alc` prints.

How it works
1. Reads the catalog: `crates/al-builtins/catalog/signatures.txt` with `overrides.txt` applied (the same lines
   `Catalog::load` uses).
2. For every overload and every argument count it accepts (each optional parameter left out or passed; a `T*` list
   passed once) writes one tiny probe file: one codeunit (or a page, report or xmlport when the method only exists on
   `CurrPage`, `CurrReport`, `CurrXMLport` or a request page) with one procedure that declares a receiver and one
   variable per argument and makes the call. A few shared objects (a table, a page with a part and an action, a report
   with a request page, a query, an xmlport, an enum) are in `shared.al`.
3. Finds the runtime values `alc` accepts (`runtime` in app.json, 1.0 up to the newest; AL1043 marks the rest).
4. Compiles the probe app once at the newest runtime. A probe with an error there has a second form (for example a
   page variable instead of `CurrPage`); one with an error in every form is listed as not expressible, with alc's
   first diagnostic.
5. Compiles the app at every accepted runtime and records, per probe file, whether it has a diagnostic.
6. Per catalog line and argument count: the lowest runtime from which the probe compiles at every later runtime, and
   what alc reports one runtime below it (`AL0666` with its "supported runtime versions" text, or another code).

Usage (compiles take about a minute each; run inside the memory slice on the owner's machine):

    systemd-run --user --scope -q --slice=al-work.slice -p MemoryMax=6G \
      python3 tools/alc-runtimes/alc_runtimes.py --alc <path to the `al` dotnet tool or `alc`> \
      --work ~/.cache/al-runtime/scratch/alc-runtimes

`--alc` is the latest AL compiler (NuGet `Microsoft.Dynamics.BusinessCentral.Development.Tools`, prerelease included,
installed with `dotnet tool install --tool-path DIR ... --version V`), the same one the corpus uses: V is the
`al_tool_version` of `corpus/.github/workflows/ci.yml`. Pass V as `--package V` so the header names it. The work folder holds the probe
app and the compiler logs; keep it outside the repository. `--out` defaults to the catalog file above.
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CATALOG = REPO / "crates/al-builtins/catalog"


# ---------------------------------------------------------------- catalog


def catalog_lines():
    """The catalog lines in `signatures.txt` order, with `overrides.txt` applied (removals in place, additions last)."""
    lines = [l for l in (CATALOG / "signatures.txt").read_text().splitlines() if l and not l.startswith("#")]
    for raw in (CATALOG / "overrides.txt").read_text().splitlines():
        if raw.startswith("- "):
            lines.remove(raw[2:])
        elif raw.startswith("+ "):
            lines.append(raw[2:])
    return lines


LINE = re.compile(r"^(\w+)\.(\w+) (inst|static) \((.*)\)(?: -> (.+?))?(?: @([\d.]+))?$")


def split_params(s):
    out, depth, cur = [], 0, ""
    for ch in s:
        if ch == "[":
            depth += 1
        elif ch == "]":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def parse(line):
    m = LINE.match(line)
    if not m:
        raise ValueError(f"bad catalog line: {line}")
    recv, name, kind, params, ret, _since = m.groups()
    ps = []
    for p in split_params(params):
        opt = p.startswith("?")
        p = p.lstrip("?")
        var = p.startswith("var ")
        if var:
            p = p[4:]
        pname, ty = p.split(": ", 1)
        rest = ty.endswith("*")
        ps.append({"name": pname, "ty": ty.rstrip("*"), "opt": opt, "var": var, "rest": rest})
    ret_opt = bool(ret) and ret.endswith("?")
    return {
        "recv": recv,
        "name": name,
        "kind": kind,
        "params": ps,
        "ret": (ret or "").rstrip("?") or None,
        "ret_opt": ret_opt,
    }


def arg_counts(sig):
    """Every argument count the overload accepts: all required, then each optional one more (a list counts once)."""
    req = sum(1 for p in sig["params"] if not p["opt"])
    return list(range(req, len(sig["params"]) + 1))


def passed(sig, n, prefix=False):
    """The parameters a call with `n` arguments passes: every required one, and the first optional ones in order
    (`IsolatedStorage.Get(Key, ?DataScope, var Value)` with two arguments passes Key and Value)."""
    if prefix:
        return list(enumerate(sig["params"][:n]))
    extra = n - sum(1 for p in sig["params"] if not p["opt"])
    out = []
    for i, p in enumerate(sig["params"]):
        if not p["opt"]:
            out.append((i, p))
        elif extra > 0:
            out.append((i, p))
            extra -= 1
    return out


# ---------------------------------------------------------------- AL types

# Generic parameters: List of [T] is a List of [Integer], Dictionary of [TKey, TValue] a Dictionary of [Integer, Text].
GENERIC = {"T": "Integer", "TKey": "Integer", "TValue": "Text"}
OBJ = {
    "Record": 'Record "ALR Probe Table"',
    "Codeunit": 'Codeunit "ALR Probe Codeunit"',
    "Page": 'Page "ALR Probe Page"',
    "Report": 'Report "ALR Probe Report"',
    "Query": 'Query "ALR Probe Query"',
    "Xmlport": 'XmlPort "ALR Probe Xmlport"',
    "TestPage": 'TestPage "ALR Probe Page"',
    "Enum": 'Enum "ALR Probe Enum"',
    "List": "List of [Integer]",
    "Dictionary": "Dictionary of [Integer, Text]",
    "Code": "Code[20]",
    "String": "Text",
    "Option": "Option A,B",
    "Any": "Variant",
    "Label": "Label 'L'",
    "TextConst": "TextConst ENU = 'L'",
}


def al_type(ty):
    """The declaration of a variable of catalog type `ty`, or None when no variable can have it."""
    if ty in GENERIC:
        return GENERIC[ty]
    if ty in OBJ:
        return OBJ[ty]
    m = re.match(r"^(List|Array|Dictionary) of \[(.*)\]$", ty)
    if m:
        inner = [al_type(t) for t in split_params(m.group(2))]
        if None in inner:
            return None
        if m.group(1) == "Array":
            return f"array[2] of {inner[0]}"
        return f"{m.group(1)} of [{', '.join(inner)}]"
    if ty in ("DotNet", "TestField", "TestFilterField", "Blob", "Media", "MediaSet", "TestRequestPage") or ty in CHAIN:
        return None
    return ty


# ---------------------------------------------------------------- probes

# Receivers that are a field of the probe table.
FIELD_RECV = {"Blob": '"Blob F"', "Media": '"Media F"', "MediaSet": '"MediaSet F"'}
# Receivers that only exist inside a test codeunit (the expression, given `TP: TestPage "ALR Probe Page"`).
TEST_RECV = {
    "TestPage": "TP",
    "TestPart": "TP.PartCtl",
    "TestField": "TP.NoCtl",
    "TestAction": "TP.ActCtl",
    "TestFilter": "TP.Filter",
    "TestRequestPage": "TRP",
}
# A TestField argument: a control of the receiver.
TEST_FIELD = {"TestPart": "TP.PartCtl.NoCtl", "TestRequestPage": "TRP.ReqCtl"}
TEST_ONLY = set(TEST_RECV) | {"TestHttpRequestMessage", "TestHttpResponseMessage"}


# The field of the probe table whose type takes a TestField value of a catalog type.
VALUE_FIELD = {
    "Boolean": '"Bool F"',
    "Code": '"Code F"',
    "Guid": '"Guid F"',
    "Text": '"Name"',
    "Label": '"Name"',
    "TextConst": '"Name"',
    "Decimal": '"Amount"',
    "BigInteger": '"BigInt F"',
    "Enum": '"Enum F"',
}


def field_arg(sig, p, i):
    """An expression for a parameter that names a field or column (`Field: Any` on Record, a query column ...), with
    the declarations it needs; None for an ordinary parameter."""
    recv, n = sig["recv"], sig["name"].lower()
    if p["ty"] != "Any":
        return None
    if recv == "Record" and re.match(r"^(Field|Fields|Field\d*|FromField)$", p["name"]):
        if n in ("calcfields", "setautocalcfields"):
            return '"Blob F"', []
        if n == "calcsums":
            return '"Amount"', []
        if n == "testfield" and len(sig["params"]) > 1:
            return VALUE_FIELD.get(sig["params"][1]["ty"], '"No."'), []
        return '"No."', []
    if recv == "Record" and p["name"] == "ToField":
        return 'R2."No."', [("R2", OBJ["Record"])]
    if recv == "Query" and p["name"] == "Column":
        return "X.No", []
    if p["name"] == "Field" and i > 0 and sig["params"][i - 1]["ty"] == "Record":
        return f'A{i - 1}."No."', []
    return None


def strategies(sig):
    """The forms a probe can take, in the order tried: (object kind, receiver expression, receiver declaration)."""
    r, kind = sig["recv"], sig["kind"]
    if kind == "static":
        if r == "System":
            return [("codeunit", "", None)]
        if r == "Enum":
            return [("codeunit", 'Enum::"ALR Probe Enum".', None)]
        return [("codeunit", f"{r}.", None)]
    if r in FIELD_RECV:
        return [("codeunit", f"R.{FIELD_RECV[r]}.", ("R", OBJ["Record"]))]
    if r in TEST_RECV:
        # A property (TestPage.Filter, a line of overrides.txt) is written without parentheses.
        return [("test", TEST_RECV[r] + ".", None), ("test-noparens", TEST_RECV[r] + ".", None)]
    if r == "RequestPage":
        return [("report", "RequestOptionsPage.", None)]
    if r == "Page":
        return [("codeunit", "X.", ("X", OBJ["Page"])), ("page", "CurrPage.", None), ("promptpage", "CurrPage.", None)]
    if r == "Report":
        return [("codeunit", "X.", ("X", OBJ["Report"])), ("report", "CurrReport.", None)]
    if r == "Xmlport":
        return [("codeunit", "X.", ("X", OBJ["Xmlport"])), ("xmlport", "currXMLport.", None)]
    if r == "Codeunit":
        # Run(var Record) needs a codeunit with a TableNo; Run() one without.
        return [("codeunit", "X.", ("X", OBJ["Codeunit"])), ("codeunit", "X.", ("X", 'Codeunit "ALR Probe Codeunit T"'))]
    t = al_type(r)
    if t is None:
        return []
    obj = "test" if r in TEST_ONLY else "codeunit"
    return [(obj, "X.", ("X", t))]


def args_for(sig, n, prefix=False):
    """(declarations, argument expressions) for the first `n` parameters; None when a type has no variable."""
    decls, exprs = [], []
    for i, p in passed(sig, n, prefix):
        f = field_arg(sig, p, i)
        if f:
            exprs.append(f[0])
            decls.extend(f[1])
            continue
        if p["ty"] == "TestFilterField":
            exprs.append('"No."')
            continue
        if p["ty"] == "TestField":
            exprs.append(TEST_FIELD.get(sig["recv"], "TP.NoCtl"))
            continue
        t = al_type(p["ty"])
        if t is None:
            return None
        decls.append((f"A{i}", t))
        exprs.append(f"A{i}")
    return decls, exprs


# Return types no variable can have: the probe calls a method of the result instead of assigning it.
CHAIN = {"TestAction": "Invoke()", "TestField": "Activate()", "TestFilter": "Ascending(true)", "TestPart": "New()"}


def ret_type(sig):
    if not sig["ret"] or sig["ret_opt"] or sig["ret"] in CHAIN:
        return None
    return al_type(sig["ret"]) or "Variant"


def probe_source(idx, sig, n, strat, obj_id):
    """The AL source of probe `idx`: one object with one procedure that makes the call."""
    kind, recv_expr, recv_decl, prefix = strat
    noparens = kind.endswith("-noparens")
    kind = kind.removesuffix("-noparens")
    a = args_for(sig, n, prefix)
    if a is None:
        return None
    decls, exprs = a
    if recv_decl:
        decls = [recv_decl] + decls
    head = "    procedure Probe()\n"
    if kind == "test":
        decls = [("TP", OBJ["TestPage"])] + decls
        # A TestRequestPage variable exists only as the parameter of a request page handler.
        head = '    [RequestPageHandler]\n    procedure Probe(var TRP: TestRequestPage "ALR Probe Report")\n'
    rt = ret_type(sig)
    if rt:
        decls.append(("Ret", rt))
    call = f"{recv_expr}{sig['name']}" + ("" if noparens else f"({', '.join(exprs)})")
    if rt:
        stmt = f"Ret := {call};"
    elif sig["ret"] in CHAIN and not sig["ret_opt"]:
        stmt = f"{call}.{CHAIN[sig['ret']]};"
    else:
        stmt = f"{call};"
    var = "".join(f"        {v}: {t};\n" for v, t in decls)
    body = head + (f"    var\n{var}" if var else "") + f"    begin\n        {stmt}\n    end;\n"
    name = f'"ALR Probe {idx}"'
    if kind in ("codeunit", "test"):
        sub = "    Subtype = Test;\n\n" if kind == "test" else ""
        return f"codeunit {obj_id} {name}\n{{\n{sub}{body}}}\n"
    if kind in ("page", "promptpage"):
        ptype = "    PageType = PromptDialog;\n" if kind == "promptpage" else ""
        return (
            f"page {obj_id} {name}\n{{\n{ptype}    SourceTable = \"ALR Probe Table\";\n\n"
            f"    layout {{ area(Content) {{ field(NoCtl; Rec.\"No.\") {{ }} }} }}\n\n{body}}}\n"
        )
    if kind == "report":
        return (
            f"report {obj_id} {name}\n{{\n    ProcessingOnly = true;\n\n"
            f"    dataset {{ dataitem(D; \"ALR Probe Table\") {{ }} }}\n\n"
            f"    requestpage {{ layout {{ area(Content) {{ }} }} }}\n\n{body}}}\n"
        )
    if kind == "xmlport":
        return (
            f"xmlport {obj_id} {name}\n{{\n    schema {{ textelement(Root) {{ tableelement(T; \"ALR Probe Table\") {{ }} }} }}\n\n"
            f"{body}}}\n"
        )
    raise ValueError(kind)


SHARED = """table 50000 "ALR Probe Table"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; "Amount"; Decimal) { }
        field(3; "Blob F"; Blob) { }
        field(4; "Media F"; Media) { }
        field(5; "MediaSet F"; MediaSet) { }
        field(6; "Name"; Text[50]) { }
        field(7; "Bool F"; Boolean) { }
        field(8; "Code F"; Code[20]) { }
        field(9; "Guid F"; Guid) { }
        field(10; "BigInt F"; BigInteger) { }
        field(11; "Enum F"; Enum "ALR Probe Enum") { }
    }
    keys { key(PK; "No.") { } }
}

page 50001 "ALR Probe Page"
{
    SourceTable = "ALR Probe Table";
    PageType = Card;

    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { }
            part(PartCtl; "ALR Probe Part") { }
        }
    }
    actions { area(Processing) { action(ActCtl) { trigger OnAction() begin end; } } }
}

page 50002 "ALR Probe Part"
{
    SourceTable = "ALR Probe Table";
    PageType = ListPart;

    layout { area(Content) { repeater(G) { field(NoCtl; Rec."No.") { } } } }
}

report 50003 "ALR Probe Report"
{
    ProcessingOnly = true;

    dataset { dataitem(D; "ALR Probe Table") { } }

    requestpage
    {
        layout { area(Content) { field(ReqCtl; N) { } } }
    }

    var
        N: Integer;
}

query 50004 "ALR Probe Query"
{
    elements { dataitem(D; "ALR Probe Table") { column(No; "No.") { } } }
}

xmlport 50005 "ALR Probe Xmlport"
{
    schema { textelement(Root) { tableelement(T; "ALR Probe Table") { } } }
}

enum 50006 "ALR Probe Enum"
{
    value(0; A) { }
    value(1; B) { }
}

codeunit 50007 "ALR Probe Codeunit"
{
}

codeunit 50008 "ALR Probe Codeunit T"
{
    TableNo = "ALR Probe Table";
}
"""

APP_ID = "8f3c0a1e-6b1d-4c55-9d0e-5a7b2c3d4e5f"


def write_app(folder, runtime, probes):
    """The probe app at `runtime`: app.json, shared.al and the probe files. Below runtime 4.0 the app cannot declare
    `target` (alc: "'OnPrem' is not available in runtime version '1.0'... '4.0' or greater"), so it is Cloud there."""
    if folder.exists():
        shutil.rmtree(folder)
    (folder / ".alpackages").mkdir(parents=True)
    major = int(runtime.split(".")[0])
    app = {"id": APP_ID, "name": "ALR Runtime Probes", "publisher": "AL Runner", "version": "1.0.0.0", "runtime": runtime}
    if major >= 4:
        app["target"] = "OnPrem"
    if major >= 3:
        app["idRanges"] = [{"from": 50000, "to": 99999}]
    (folder / "app.json").write_text(json.dumps(app, indent=2))
    shared = SHARED
    if major < 2:
        # alc: "'Enum Type' is not available in runtime version '1.0'. The supported runtime versions are: '2.0' or
        # greater." An error in an object declaration stops alc before it binds any method body, so the enum and
        # its field are left out there (a probe that uses them then fails at 1.0, as an app of runtime 1.0 would).
        shared = shared.replace('        field(11; "Enum F"; Enum "ALR Probe Enum") { }\n', "")
        shared = shared[: shared.index('enum 50006 "ALR Probe Enum"')] + shared[shared.index('codeunit 50007'):]
    (folder / "shared.al").write_text(shared)
    for name, src in probes.items():
        (folder / name).write_text(src)


DIAG = re.compile(r"^(?P<file>[^\s(]+\.al)\((?P<line>\d+),(?P<col>\d+)\): error (?P<code>AL\d+): (?P<msg>.*)$")


def compile_app(alc, folder, log):
    """Compiles the app; returns {file name: [(code, message)]} for every error alc reports in a .al file."""
    out = subprocess.run(
        [alc, "compile", f"/project:{folder}", f"/packagecachepath:{folder / '.alpackages'}", f"/out:{folder / 'p.app'}"],
        capture_output=True,
        text=True,
        timeout=3600,
    )
    text = out.stdout + out.stderr
    log.write_text(text)
    errs = {}
    for l in text.splitlines():
        m = DIAG.match(l.strip())
        if m:
            errs.setdefault(os.path.basename(m["file"]), []).append((m["code"], m["msg"], int(m["line"])))
    if "app.json" in text and re.search(r"app\.json.*error", text):
        raise RuntimeError(f"alc rejected app.json, see {log}")
    return errs


def compile_rounds(alc, folder, runtime, probes, log):
    """The probe files that do not compile at `runtime`, each with its diagnostics. An error in a declaration (an
    object, or a variable of a type the runtime lacks) stops alc before it binds any method body, so one compile does
    not show every error: the files with errors are removed and the rest compiled again, until a compile reports none.
    The files left then compile together, and so each on its own."""
    left, failed, rounds = dict(probes), {}, 0
    while True:
        write_app(folder, runtime, left)
        errs = compile_app(alc, folder, log.with_name(f"{log.stem}-{rounds}.log"))
        rounds += 1
        if "shared.al" in errs:
            raise RuntimeError(f"shared.al does not compile at runtime {runtime}: {errs['shared.al'][:3]}")
        if not errs:
            return failed
        for f, e in errs.items():
            failed[f] = e
            left.pop(f, None)


def accepted_runtimes(alc, work):
    """Every `major.minor` (minor 0 to 9) for which alc does not report AL1043, oldest first."""
    folder = work / "rt"
    ok = []
    for major in range(1, 40):
        for minor in range(0, 10):
            rt = f"{major}.{minor}"
            if folder.exists():
                shutil.rmtree(folder)
            (folder / ".alpackages").mkdir(parents=True)
            (folder / "app.json").write_text(
                json.dumps({"id": APP_ID, "name": "R", "publisher": "p", "version": "1.0.0.0", "runtime": rt})
            )
            out = subprocess.run(
                [alc, "compile", f"/project:{folder}", f"/packagecachepath:{folder / '.alpackages'}", f"/out:{folder / 'p.app'}"],
                capture_output=True,
                text=True,
                timeout=600,
            )
            if "AL1043" not in out.stdout + out.stderr:
                ok.append(rt)
    return ok


def alc_version(alc):
    out = subprocess.run([alc, "compile", "/project:/nonexistent"], capture_output=True, text=True, timeout=600)
    m = re.search(r"AL Compiler version (\S+)", out.stdout + out.stderr)
    return m.group(1) if m else "unknown"


def rt_key(r):
    a, b = r.split(".")
    return (int(a), int(b))


SUPPORTED = re.compile(r"The supported runtime versions are: (.*?)\.?$")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--alc", required=True, help="the `al` dotnet tool (called as `<alc> compile ...`)")
    ap.add_argument("--work", required=True, type=Path, help="scratch folder outside the repository")
    ap.add_argument("--out", type=Path, default=CATALOG / "alc-runtimes.txt")
    ap.add_argument("--reuse", action="store_true", help="write the table from the last run's state.json, compiling nothing")
    ap.add_argument("--runtimes", help="comma-separated runtime list instead of asking alc (for a quick check)")
    ap.add_argument(
        "--package",
        help="the NuGet package version of the compiler, written to the header (the corpus's `al_tool_version` in "
        "corpus/.github/workflows/ci.yml, for example 30.0.42.60748-beta)",
    )
    args = ap.parse_args()
    work = args.work.expanduser().resolve()
    work.mkdir(parents=True, exist_ok=True)
    alc = os.path.expanduser(args.alc)

    state = json.loads((work / "state.json").read_text()) if args.reuse else None
    version = state["version"] if state else alc_version(alc)
    runtimes = state["runtimes"] if state else args.runtimes.split(",") if args.runtimes else accepted_runtimes(alc, work)
    runtimes.sort(key=rt_key)
    newest = runtimes[-1]
    print(f"alc {version}; runtimes {runtimes[0]} .. {newest} ({len(runtimes)})", file=sys.stderr)

    lines = catalog_lines()
    # Probe slots: (catalog line index, argument count) -> list of candidate sources (one per strategy).
    slots, unexpressible = {}, {}
    for i, line in enumerate(lines):
        sig = parse(line)
        for n in arg_counts(sig):
            # Each form, first with the required parameters plus the first optional ones, then (when that is a
            # different choice) with the first `n` parameters.
            srcs = []
            modes = [False] + ([True] if [i for i, _ in passed(sig, n)] != list(range(n)) else [])
            for prefix in modes:
                for s in strategies(sig):
                    srcs.append((*s, prefix))
            if not srcs:
                unexpressible[(i, n)] = f"no variable can hold a `{sig['recv']}`"
                continue
            slots[(i, n)] = srcs

    call_line = {}

    def probes_for(choice):
        out, names = {}, {}
        for k, (slot, strat_i) in enumerate(sorted(choice.items())):
            i, n = slot
            sig = parse(lines[i])
            src = probe_source(k, sig, n, slots[slot][strat_i], 60000 + k)
            if src is None:
                continue
            fname = f"p{k:05}.al"
            out[fname] = src
            names[fname] = slot
            # The line of the call: the statement before the procedure's `end;`.
            lines_ = src.splitlines()
            call_line[fname] = max(i for i, l in enumerate(lines_) if l == "    end;")
        return out, names

    # Newest runtime: pick, per slot, the first strategy that compiles. Only AL0666 there means the method was
    # removed in an older runtime (`'3.0' until, but not including '13.0'`): the probe is right, so it stays.
    choice = {slot: 0 for slot in slots}
    first_err = {}
    if state:
        choice = {tuple(k): v for k, v in state["choice"]}
        unexpressible.update({tuple(k): v for k, v in state["unexpressible"]})
    for attempt in range(0 if state else 4):
        probes, names = probes_for(choice)
        for slot in slots:
            if slot not in names.values() and slot not in unexpressible:
                unexpressible[slot] = "a parameter type has no variable (DotNet)"
        errs = compile_rounds(alc, work / "app", newest, probes, work / f"newest-{attempt}.log")
        changed = False
        for fname, slot in names.items():
            if fname in errs and any(d[0] != "AL0666" for d in errs[fname]):
                first_err.setdefault(slot, errs[fname][0])
                if choice[slot] + 1 < len(slots[slot]):
                    choice[slot] += 1
                    changed = True
                else:
                    e = first_err[slot]
                    unexpressible[slot] = f"{e[0]}: {e[1]}"
        if not changed:
            break
    good = {slot: s for slot, s in choice.items() if slot not in unexpressible}
    probes, names = probes_for(good)
    by_slot = {v: k for k, v in names.items()}

    # Every runtime: which probe files have an error.
    results = json.loads((work / "results.json").read_text()) if state else {}
    for rt in [] if state else runtimes:
        errs = compile_rounds(alc, work / "app", rt, probes, work / f"rt-{rt}.log")
        results[rt] = errs
        print(f"runtime {rt}: {len(errs)} probe files with errors", file=sys.stderr)
    if not state:
        (work / "results.json").write_text(json.dumps(results))
        (work / "state.json").write_text(
            json.dumps(
                {
                    "version": version,
                    "runtimes": runtimes,
                    "choice": [[list(k), v] for k, v in choice.items()],
                    "unexpressible": [[list(k), v] for k, v in unexpressible.items()],
                }
            )
        )

    measured = {}
    for slot, fname in sorted(by_slot.items()):
        ok = [fname not in results[rt] for rt in runtimes]
        if not any(ok):
            e = results[newest][fname][0]
            unexpressible[slot] = f"{e[0]}: {e[1]}"
            continue
        # The runtimes the call compiles at, as ranges `from..to` (`to` the first runtime it fails at again).
        ranges, k = [], 0
        while k < len(runtimes):
            if ok[k]:
                j = k
                while j < len(runtimes) and ok[j]:
                    j += 1
                ranges.append(f"{runtimes[k]}..{runtimes[j] if j < len(runtimes) else ''}")
                k = j
            else:
                k += 1
        # What alc reports for the call at each runtime it fails at, in runs of equal outcome: `AL0666 <text>` for
        # an AL0666 on the line of the call, another code there, or `declaration` when only a variable declaration
        # fails (a type the runtime lacks), so alc says nothing about the call itself.
        def outcome(rt):
            at_call = [d for d in results[rt][fname] if d[2] == call_line[fname]]
            if not at_call:
                return "declaration"
            code, msg = at_call[0][0], at_call[0][1]
            m = SUPPORTED.search(msg)
            return f"AL0666 {m.group(1)}" if code == "AL0666" and m else code

        runs, k = [], 0
        while k < len(runtimes):
            if ok[k]:
                k += 1
                continue
            o, j = outcome(runtimes[k]), k
            while j < len(runtimes) and not ok[j] and outcome(runtimes[j]) == o:
                j += 1
            runs.append(f"{runtimes[k]}..{runtimes[j] if j < len(runtimes) else ''} {o}")
            k = j
        fails = "; ".join(runs)
        measured[slot] = (",".join(ranges), fails)

    header = [
        f"# GENERATED by tools/alc-runtimes/alc_runtimes.py with AL compiler {version}. Do not edit; rerun it.",
        *(
            [f"# Compiler package: NuGet Microsoft.Dynamics.BusinessCentral.Development.Tools {args.package}."]
            if args.package
            else []
        ),
        f"# Runtimes compiled: {runtimes[0]} to {newest} ({len(runtimes)} values this compiler accepts).",
        "# <catalog line>\t<argument count>\t<runtimes>\t<what alc reports at the other runtimes>",
        "#   runtimes: where a call with that many arguments compiles, as ranges `from..to` (to: the first runtime it",
        "#   fails at again; empty: it compiles up to the newest).",
        "#   other runtimes: runs `from..to <outcome>` separated by `; `: `AL0666 <alc's text after 'The supported",
        "#   runtime versions are: '>` for an AL0666 on the call, another code on the call, or `declaration` (only a",
        "#   variable declaration fails there: a type that runtime lacks, so alc says nothing about the call).",
        "# <catalog line>\t<argument count>\t-\t<why no probe compiles at any runtime>",
    ]
    rows = []
    for (i, n), (ranges, fails) in measured.items():
        rows.append((lines[i], n, f"{lines[i]}\t{n}\t{ranges}\t{fails}"))
    for (i, n), why in unexpressible.items():
        rows.append((lines[i], n, f"{lines[i]}\t{n}\t-\t{why}"))
    rows.sort(key=lambda r: (r[0], r[1]))
    args.out.write_text("\n".join(header + [r[2] for r in rows]) + "\n")
    print(f"wrote {args.out}: {len(measured)} measured, {len(unexpressible)} not expressible", file=sys.stderr)

if __name__ == "__main__":
    main()
