#!/usr/bin/env python3
"""Catch catalog-arity drift.

The `catalog_matches_registry` Rust test only checks builtin *names*, not arg
counts. When a catalog spec declares fewer params than the engine ctor actually
reads, the playground editor wrongly rejects valid calls with
"`X` takes at most N argument(s)". This static audit compares each catalog
`spec(...)` param count against the highest positional index its engine ctor
reads (`a.num(i)` / `a.opt_num(i)` / `a.pair(i)` / …), per file so duplicate fn
names (e.g. two `c_axes`) don't collide.

Run:  python3 scripts/audit-arity.py   (exit 1 if any drift found)
"""
import re, sys, glob, pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
cat = (ROOT / "crates/manic-lang/src/catalog.rs").read_text()

# catalog: name -> declared param count. `&[]` (empty) skips the arity check.
specs = {}
for m in re.finditer(
    r'spec\(\s*"([^"]+)"\s*,\s*\w+\s*,\s*"[^"]*"\s*,\s*"[^"]*"\s*,\s*&\[(.*?)\]\s*,?\s*\)',
    cat, re.S):
    name, params = m.group(1), m.group(2)
    if params.strip() == "":
        continue
    specs[name] = len(re.findall(r'\("[^"]+"\s*,\s*\w+\s*,\s*[RO]\)', params))


def fn_arity(src):
    """fn name -> highest positional arg index the ctor reads, + 1 (the count).

    We compare against the highest *literal* index (`a.text(4)` etc.). `a.len()`
    guards optional reads but the reads themselves carry the real max — so we do
    NOT treat `a.len()` as "variadic/skip" (that hid caption/vector/rotate). A
    genuinely open-ended ctor (reads `a.num(i)` in a loop) has an empty `&[]`
    catalog spec, which is already skipped above."""
    out = {}
    for fm in re.finditer(r'fn (c_\w+|m_\w+|v_\w+)\s*\([^)]*\)\s*->\s*Result[^{]*\{', src):
        fn, s = fm.group(1), fm.end()
        depth, i = 1, s
        while i < len(src) and depth > 0:
            depth += src[i] == '{'
            depth -= src[i] == '}'
            i += 1
        body = src[s:i]
        idx = [int(x) for x in re.findall(
            r'a\.(?:num|opt_num|pair|text|ident|triple|opt_\w+)\((\d+)\)', body)]
        out[fn] = max(idx) + 1 if idx else 0
    return out


drift = []
for path in glob.glob(str(ROOT / "src/kits/*.rs")):
    src = pathlib.Path(path).read_text()
    ar = fn_arity(src)
    for name, fn in re.findall(r'r\.(?:ctor|verb)\("([^"]+)",\s*(\w+)\)', src):
        if name in specs and fn in ar and ar[fn] > specs[name]:
            drift.append((name, specs[name], ar[fn], pathlib.Path(path).name))

if drift:
    print("CATALOG ARITY DRIFT — the engine reads more args than the catalog declares:")
    for name, declared, engine, f in sorted(drift):
        print(f"  {name:16} catalog={declared}  engine={engine}  ({f})")
    print("\nFix: widen the spec(...) in crates/manic-lang/src/catalog.rs, then rebuild the WASM.")
    sys.exit(1)

print(f"no catalog-arity drift ({len(specs)} specs with declared params audited) ✓")
