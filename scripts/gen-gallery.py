#!/usr/bin/env python3
# Regenerate the mdBook examples gallery (book/src/ex-*.md + examples.md +
# SUMMARY.md) from the section/description mapping below. Each example gets its
# real source (via {{#include}}) + its video placeholder. Add an example ->
# add a row -> rerun:  python3 scripts/gen-gallery.py
from pathlib import Path
SRC = Path("book/src")
SECTIONS = [
 ("algorithms","Algorithms & data structures","",[
   ("bubble_sort","Real sliding swaps; `array` + `compare` + `swap`."),
   ("two_pointer","`lo`/`hi` index carets scanning inward on a sorted array."),
   ("stack_queue","LIFO stack + FIFO queue, with action-point carets."),
   ("linked_list","Singly / doubly / circular — classic node anatomy + pointer re-threading."),
   ("hashmap","Hash a key to a bucket; collisions chain on (separate chaining)."),
 ]),
 ("graphs","Graphs","",[
   ("graph","Labelled nodes + edges, drawn on; reflowing links."),
   ("graph_moving","Drag a vertex and its incident edges follow."),
   ("bfs_dfs","The same graph, queue vs stack, with live frontier readouts."),
   ("dijkstra","Weighted edges, settling distances, a shortest-path tree."),
 ]),
 ("calculus","Calculus & functions","",[
   ("sine_wave","`axes` + `plot`, a curve traced on, then vectors."),
   ("function_graph","Plot an expression straight from a formula string."),
   ("area_under_curve","Riemann rectangles sweeping to the integral."),
   ("riemann_rainbow","Coloured Riemann rectangles revealed one by one."),
   ("riemann_readout","Running sums shown as a live computed number."),
 ]),
 ("linalg","Linear algebra & tables","",[
   ("matrix","A bracketed matrix, rows/columns addressable via tags."),
   ("matrix_addition","Two matrices summed, cell by cell."),
   ("matrix_addition_plane","The same sum, laid out on a coordinate plane."),
   ("linear_transform","A 2x2 matrix shearing a grid + basis vectors."),
   ("table","A ruled table; cells, rows, columns, labels all addressable."),
   ("table_braces","A table annotated with braces."),
 ]),
 ("vectors","Vectors, fields & coordinates","",[
   ("vector_field","A magnitude-coloured vector field."),
   ("coordinates","Axes, planes, number lines, polar & complex planes."),
   ("pie","A pie chart built from sectors."),
 ]),
 ("geometry","Geometry (olympiad)","Every construction is **live** — the derived points recompute as the inputs move.",[
   ("equilateral","Euclid I.1 — an equilateral triangle from two circles."),
   ("triangle","A triangle with its centres and cevians."),
   ("incircle_tangents","The incircle and its tangent points."),
   ("tangents","Tangent lines from a point to a circle."),
   ("orthocenter","The orthocentre from the three altitudes."),
   ("euler_line","The Euler line through centroid, circumcentre, orthocentre."),
   ("nine_point","The nine-point circle."),
   ("conics","Ellipse, parabola, hyperbola."),
 ]),
 ("transforms","Transforms & morphing","",[
   ("transforms","Apply a 2x2 matrix (ApplyMatrix) to a group."),
   ("transform_copy","Duplicate an entity, then transform the copy."),
   ("morph","A sampled-point shape morph from A to B."),
 ]),
 ("text","Text & UI","",[
   ("typewriter","Text revealed character by character."),
   ("captions","Karaoke / word-pop caption modes."),
   ("terminal_boot","The neon terminal template booting up."),
   ("brace","The curly-brace family."),
   ("banner","The manic logo / banner reveal."),
 ]),
 ("generative","Generative & recursive","",[
   ("fractal_tree","One recursive `def`, drawn to depth 12."),
   ("hue_wave","An animated hue wave across a grid."),
   ("hill_run","A little scene animated with the language layer."),
   ("equal_cuts","A circle halved again and again (pizza cuts)."),
   ("archimedes_pi","Bounding pi with inscribed / circumscribed polygons."),
 ]),
 ("boolean","Boolean shapes","",[
   ("boolean","Union / intersection / difference of shapes."),
 ]),
]

# section pages
for slug,title,intro,items in SECTIONS:
    lines=[f"# {title}\n"]
    if intro: lines.append(intro+"\n")
    lines.append("Each block is the whole file — copy it into `x.manic` and run "
                 "`manic x.manic` (live) or `--record out` (video).\n")
    for name,desc in items:
        lines.append(f"## {name}\n")
        lines.append(desc+"\n")
        lines.append("```manic")
        lines.append(f"{{{{#include ../../examples/{name}.manic}}}}")
        lines.append("```\n")
        lines.append(f'<div class="manic-video" data-video="ex-{name}"></div>\n')
    (SRC/f"ex-{slug}.md").write_text("\n".join(lines))

# index
idx=["# Examples gallery\n",
     "Every animation in `examples/`, by topic — **the code and the clip for each**. "
     "Run any of them with `manic examples/<name>.manic`. Project: <https://8gwifi.org/manic>.\n"]
for slug,title,intro,items in SECTIONS:
    idx.append(f"- [{title}](ex-{slug}.md) — {len(items)} example{'s' if len(items)!=1 else ''}")
(SRC/"examples.md").write_text("\n".join(idx)+"\n")

# SUMMARY (nest sections under the gallery)
summ=["# Summary\n","[Introduction](introduction.md)\n",
 "- [Getting started](getting-started.md)",
 "- [Shapes — the cast](shapes.md)",
 "- [Verbs — bringing it to life](verbs.md)",
 "- [Timing — par, seq & stagger](timing.md)",
 "- [Colour & style](colour.md)",
 "- [The language layer](language-layer.md)",
 "- [Kits — math, geometry, algorithms](kits.md)",
 "- [Examples gallery](examples.md)"]
for slug,title,_,_ in SECTIONS:
    summ.append(f"    - [{title}](ex-{slug}.md)")
(SRC/"SUMMARY.md").write_text("\n".join(summ)+"\n")
print(f"generated {len(SECTIONS)} section pages + index + SUMMARY")