#!/usr/bin/env python3
"""Write the new case totals into crates/al-test-cli/tests/corpus_harness.rs (step 7 of tools/corpus_bump.sh).

Only assertion code changes. Every pattern is anchored on the code around the number, so a number in a
comment (for example "tests of 25077 (PeVTests.al:750 ...)") is never touched.

  corpus_bump_harness.py ROOT PASS_RATE_DIR OLD_PIN NEW_PIN  (MAJOR 0 0 TOTAL ONPREM)x3
"""
import json, os, re, sys


def update(t, legs, onprem, old, new, areas=None):
    t = re.sub(r'\[\("27", \d+\), \("28", \d+\), \("29", \d+\)\]',
               f'[("27", {legs["27"]}), ("28", {legs["28"]}), ("29", {legs["29"]})]', t)
    t = re.sub(r'(json\("29"\);\n\s*assert_eq!\(r\["overall"\]\["total"\], )\d+\);', rf'\g<1>{legs["29"]});', t)
    t = re.sub(r'(json\("28"\);\n\s*assert_eq!\(r\["overall"\]\["total"\], )\d+\);', rf'\g<1>{legs["28"]});', t)
    t = re.sub(r'(get\("onprem"\)\["counts"\]\["total"\], )\d+\);', rf'\g<1>{onprem["28"]});', t)
    if old != new:
        # Only a line of code may carry the pin; comment lines keep the historical pins.
        t = "".join(l if l.lstrip().startswith("//") else l.replace(old, new) for l in t.splitlines(True))
    # The md assert only: `md.contains(" of N (")`.
    t = re.sub(r'(md\.contains\(" of )\d+( \(")', rf'\g<1>{legs["29"]}\2', t)
    if areas:
        for area in ("json", "record"):
            t = re.sub(rf'(assert_eq!\(r\["areas"\]\["{area}"\]\["total"\], )\d+\);',
                       rf'\g<1>{areas[area]["total"]});', t)
    return t


def main(argv):
    root, out, old, new, rest = argv[1], argv[2], argv[3], argv[4], argv[5:]
    legs = {rest[i]: rest[i + 3] for i in range(0, len(rest), 5)}
    onprem = {rest[i]: rest[i + 4] for i in range(0, len(rest), 5)}
    p = root + "/crates/al-test-cli/tests/corpus_harness.rs"
    f = os.path.join(out, "pass-rate-29.json")
    areas = json.load(open(f))["areas"] if os.path.exists(f) else None
    t = update(open(p).read(), legs, onprem, old, new, areas)
    open(p, "w").write(t)


if __name__ == "__main__":
    main(sys.argv)
