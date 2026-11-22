#!/usr/bin/env python3
"""Packs the dependency apps of a neighbor app into source packages (tests/neighbors/README.md, "Dependency apps").

Usage: pack_deps.py APP_FOLDER OUT_FOLDER

Each folder APP_FOLDER/deps/<name>/ holds an app.json and src/*.al. It becomes
OUT_FOLDER/<publisher>_<name>_<version>.Source.zip with app.json at the archive root, the form the runner loads as a
dependency with source (`--dep OUT_FOLDER`). A plain folder in .alpackages would be compiled into the app itself, and
a committed .zip or .app is refused by the legal guard, so the package is made at run time. Prints the number packed.
"""
import json
import os
import re
import sys
import zipfile


def main(argv):
    if len(argv) != 3:
        print(__doc__)
        return 2
    app, out = argv[1], argv[2]
    deps = os.path.join(app, "deps")
    names = sorted(os.listdir(deps)) if os.path.isdir(deps) else []
    os.makedirs(out, exist_ok=True)
    n = 0
    for name in names:
        folder = os.path.join(deps, name)
        manifest = os.path.join(folder, "app.json")
        if not os.path.isfile(manifest):
            print(f"pack_deps: {folder} has no app.json", file=sys.stderr)
            return 2
        with open(manifest, encoding="utf-8-sig") as f:
            j = json.load(f)
        stem = "_".join(re.sub(r"[^\w.-]", "_", str(j[k])) for k in ("publisher", "name", "version"))
        with zipfile.ZipFile(os.path.join(out, stem + ".Source.zip"), "w", zipfile.ZIP_DEFLATED) as z:
            for dirpath, _, files in os.walk(folder):
                for fn in sorted(files):
                    p = os.path.join(dirpath, fn)
                    z.write(p, os.path.relpath(p, folder).replace(os.sep, "/"))
        n += 1
    print(n)
    return 0


sys.exit(main(sys.argv))
