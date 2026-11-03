#!/usr/bin/env python3
"""Rewrite corpus path citations after the corpus restructure moved files (plan section F, runner bump 1).

  python3 tools/rewrite_paths.py --map corpus/.github/restructure/path-map.tsv [--check] [PATH ...]

PATH: files or folders to rewrite (default: crates docs tools oracle CLAUDE.md README.md). Local orchestration files can be
passed too. Three citation forms are rewritten, with any `:line` suffix kept (the move changes no line):
  - full:          [corpus/]tests/<app>/<path>
  - app folder:    <app>/<path>                (for example al-language-onprem-dotnet/lt-qx-dotnet/X.al)
  - app-relative:  <path>                      (for example record/lt-y66-flow/X.al), only when it names one target
A folder is rewritten when all its files went to one folder. Nothing else is touched. The report lists the app-relative
citations that are ambiguous (left as they are) and the corpus citations that resolve neither before nor after the move.
--check writes nothing and exits 1 when a file would change.
"""
import argparse
import collections
import os
import re
import sys

DEFAULT = ['crates', 'docs', 'tools', 'oracle', 'CLAUDE.md', 'README.md']
SKIP_DIRS = {'target', '.git', '.worktrees', 'corpus', 'node_modules', '__pycache__'}
TOKEN = re.compile(r'(?<![A-Za-z0-9_./-])[A-Za-z0-9_][A-Za-z0-9_./-]*')
DELETE = '(delete: build output)'


def load(path):
    files, dirs = {}, collections.defaultdict(set)
    for line in open(path, encoding='utf-8'):
        f = line.rstrip('\n').split('\t')
        if len(f) < 2 or f[0] == 'old_path':
            continue
        old, new = f[0], f[1]
        if new == DELETE:
            continue
        files[old] = new
        dirs[os.path.dirname(old)].add(os.path.dirname(new))
    kept = {os.path.dirname(o) for o, n in files.items() if o == n}
    moves = {o: n for o, n in files.items() if o != n}
    # a folder moves when no file stays in it and all its files went to one folder
    dmoves = {d: next(iter(t)) for d, t in dirs.items() if len(t) == 1 and d not in kept and next(iter(t)) != d}
    known = set(files) | set(files.values()) | set(dirs) | {os.path.dirname(n) for n in files.values()}
    return moves, dmoves, known


def app_split(p):
    parts = p.split('/')
    return parts[1], '/'.join(parts[2:])


def build(moves, dmoves):
    full = dict(moves)
    full.update(dmoves)
    by_app = {}
    rel = collections.defaultdict(set)
    for o, n in full.items():
        app, r = app_split(o)
        _, nr = app_split(n)
        by_app[f'{app}/{r}'] = f'{app}/{nr}'
        rel[r].add(nr)
    rel_unique = {r: next(iter(t)) for r, t in rel.items() if len(t) == 1}
    ambiguous = {r for r, t in rel.items() if len(t) > 1}
    return full, by_app, rel_unique, ambiguous


def rewrite_text(text, tables, known, report):
    full, by_app, rel_unique, ambiguous = tables

    def sub(m):
        t = m.group(0)
        core = t.rstrip('./')
        tail = t[len(core):]
        if core.startswith('corpus/tests/') or core.startswith('tests/'):
            pre = 'corpus/' if core.startswith('corpus/') else ''
            key = core[len(pre):]
            if key in full:
                return pre + full[key] + tail
            if key.endswith('.al') and key not in known:
                report['unresolved'].add(key)
            return t
        if core in by_app:
            return by_app[core] + tail
        if core in rel_unique:
            return rel_unique[core] + tail
        if core in ambiguous:
            report['ambiguous'].add(core)
        return t
    return TOKEN.sub(sub, text)


def walk(paths):
    for p in paths:
        if os.path.isfile(p):
            yield p
        elif os.path.isdir(p):
            for d, subdirs, fs in os.walk(p):
                subdirs[:] = [s for s in subdirs if s not in SKIP_DIRS]
                for f in fs:
                    yield os.path.join(d, f)


def main(argv):
    ap = argparse.ArgumentParser()
    ap.add_argument('--map', required=True)
    ap.add_argument('--check', action='store_true')
    ap.add_argument('paths', nargs='*')
    a = ap.parse_args(argv)
    moves, dmoves, known = load(a.map)
    tables = build(moves, dmoves)
    report = {'unresolved': set(), 'ambiguous': set()}
    changed, cites = [], 0
    for f in walk(a.paths or [p for p in DEFAULT if os.path.exists(p)]):
        try:
            text = open(f, encoding='utf-8').read()
        except (UnicodeDecodeError, OSError):
            continue
        new = rewrite_text(text, tables, known, report)
        if new != text:
            changed.append(f)
            cites += sum(1 for x, y in zip(TOKEN.findall(text), TOKEN.findall(new)) if x != y)
            if not a.check:
                open(f, 'w', encoding='utf-8').write(new)
    print(f'{"would change" if a.check else "changed"} {len(changed)} files, {cites} citations')
    for f in changed:
        print(f'  {f}')
    for kind in ('ambiguous', 'unresolved'):
        print(f'{kind}: {len(report[kind])}')
        for k in sorted(report[kind]):
            print(f'  {kind} {k}')
    return 1 if a.check and changed else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
