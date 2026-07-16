#!/usr/bin/env python3
# Publish the manic examples to the crypto-tool playground: regenerate its
# `examples/index.json` (grouped like the mdBook gallery) and copy every example
# `.manic` file. Single source of truth = the SECTIONS table in gen-gallery.py, so
# the playground list always matches the book gallery. Run after gen-gallery.py:
#     python3 scripts/gen-ui-index.py
import importlib.util
import json
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEST = Path("/Users/anish/git/crypto-tool/src/main/webapp/manic/examples")

# load gen-gallery.py by path (hyphenated name isn't importable directly)
spec = importlib.util.spec_from_file_location("gengallery", ROOT / "scripts" / "gen-gallery.py")
gg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gg)

# the playground has historically used "threed" for the 3D section slug
SLUG_MAP = {"3d": "threed"}


def short_title(name, header):
    base = header.split("—")[0].strip() if header else name
    base = base.replace("-", " ").replace("_", " ").strip()
    return base[:1].upper() + base[1:] if base else name


def short_desc(desc):
    d = " ".join(desc.split())                 # collapse newlines/indentation
    d = re.sub(r"`([^`]*)`", r"\1", d)          # strip code backticks
    first = d.split(". ")[0].rstrip(".")        # first sentence
    return (first + ".") if first else d


cats, count, copied, missing = [], 0, 0, []
for sec in gg.SECTIONS:
    examples = []
    for item in sec["items"]:
        name, desc = item[0], item[1]
        header = item[2] if len(item) > 2 else None
        examples.append({
            "name": name,
            "file": f"{name}.manic",
            "title": short_title(name, header),
            "desc": short_desc(desc),
        })
        count += 1
        src = ROOT / "examples" / f"{name}.manic"
        if src.exists():
            shutil.copy(src, DEST / f"{name}.manic")
            copied += 1
        else:
            missing.append(name)
    cats.append({
        "slug": SLUG_MAP.get(sec["slug"], sec["slug"]),
        "title": sec["title"],
        "examples": examples,
    })

(DEST / "index.json").write_text(json.dumps({"count": count, "categories": cats}, indent=2) + "\n")
print(f"published {count} examples across {len(cats)} categories; copied {copied} .manic files")
if missing:
    print(f"  WARNING: {len(missing)} gallery entries have no .manic file: {missing}")
