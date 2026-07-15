#!/usr/bin/env python3
# Regenerate the mdBook examples gallery (book/src/ex-*.md + examples.md +
# SUMMARY.md) from the section/description mapping below. Each example gets its
# real source (via {{#include}}) + its video placeholder. Add an example ->
# add a row -> rerun:  python3 scripts/gen-gallery.py
#
# This file is the SINGLE SOURCE OF TRUTH for the gallery pages and the
# SUMMARY nav — running it OVERWRITES them, so edit the data here, not the
# generated .md files. Each item is either ("name", "desc") — the ## header is
# the name — or ("name", "desc", "Custom header") when the heading differs from
# the include filename (e.g. the lesson pages). `desc` may span multiple lines.
from pathlib import Path
SRC = Path("book/src")

# Standard blurb under every section title.
SUB = ("Each block is the whole file — copy it into `x.manic` and run "
       "`manic x.manic` (live) or `--record out` (video).")
# The 3D page points readers at the Going 3D chapter for its vocabulary.
SUB_3D = SUB + " See the [Going 3D](3d.md) chapter for the words used here."

SECTIONS = [
 dict(slug="algorithms", title="Algorithms & data structures", intro="", sub=SUB, items=[
   ("bubble_sort", "Real sliding swaps; `array` + `compare` + `swap`."),
   ("two_pointer", "`lo`/`hi` index carets scanning inward on a sorted array."),
   ("stack_queue", "LIFO stack + FIFO queue, with action-point carets."),
   ("linked_list", "Singly / doubly / circular — classic node anatomy + pointer re-threading."),
   ("hashmap", "Hash a key to a bucket; collisions chain on (separate chaining)."),
 ]),
 dict(slug="graphs", title="Graphs", intro="", sub=SUB, items=[
   ("graph", "Labelled nodes + edges, drawn on; reflowing links."),
   ("graph_moving", "Drag a vertex and its incident edges follow."),
   ("bfs_dfs", "The same graph, queue vs stack, with live frontier readouts."),
   ("dijkstra", "Weighted edges, settling distances, a shortest-path tree."),
 ]),
 dict(slug="calculus", title="Calculus & functions", intro="", sub=SUB, items=[
   ("calculus-demo",
    "The flagship: two big ideas on one curve. A tangent slides along a bell curve\n"
    "with a live slope readout (flat at the peak), then the area sweeps open while the\n"
    "integral climbs to its true value — on properly numbered, scaled axes."),
   ("sine_wave", "`axes` + `plot`, a curve traced on, then vectors."),
   ("function_graph", "Plot an expression straight from a formula string."),
   ("area_under_curve", "Riemann rectangles sweeping to the integral."),
   ("riemann_rainbow", "Coloured Riemann rectangles revealed one by one."),
   ("riemann_readout", "Running sums shown as a live computed number."),
   ("tangent",
    "The tangent line to a curve, sliding along it — its tilt is read from the\n"
    "function itself, so it's always the true slope (flat at the peaks)."),
   ("analysis",
    "Ask one curve everything at once — tangent, a live slope number, the normal, the\n"
    "area sweeping open beneath it, and the integral climbing to its true value."),
   ("newton",
    "Newton's method, drawn as a zig-zag: from a first guess, slide down each tangent\n"
    "to the axis, back up to the curve, and watch the guesses walk to the root."),
   ("inverse-derivatives",
    "Why a function and its inverse have reciprocal slopes: `e^x` and `ln x` mirrored\n"
    "across `y = x`, with the slopes at matching points multiplying to 1."),
   ("spline",
    "Interpolation: one smooth curve drawn through a scattered set of points — it\n"
    "passes through every knot exactly."),
   ("trajectory",
    "A phase portrait: three paths flowing under a differential system, each\n"
    "spiralling into the sink at the origin."),
 ]),
 dict(slug="linalg", title="Linear algebra & tables", intro="", sub=SUB, items=[
   ("linear-algebra",
    "A guided lesson, not a feature demo: five chapters that build linear algebra as\n"
    "one connected story. Chapters 1–3 view the **same** matrix `[[2,1],[1,2]]`\n"
    "through three lenses — a transformation of space (`linmap`), the determinant as\n"
    "area scaling (`determinant`), and its eigenvectors / diagonalisation\n"
    "(`diagonalise`) — then it moves on to solving `Ax = b` (`linsolve` → `rref`) and\n"
    "projection / least-squares (`project`). Start here.",
    "linear-algebra — the whole subject in five ideas"),
   ("linear-map",
    "What a 2×2 matrix does to space: the grid deforms and the basis lands on its\n"
    "columns (`linmap`), the unit square's area becomes the determinant\n"
    "(`determinant`), and two directions only stretch — the eigenvectors (`eigen`)."),
   ("linear-system",
    "The geometry of solving and spanning, in three panels: a 2×2 system as two lines\n"
    "crossing at the solution (`linsolve`), two independent vectors reaching the whole\n"
    "plane, and two parallel vectors collapsing to a line — rank 1 (`span`)."),
   ("diagonalise",
    "`A = P D P⁻¹` made visual: every real-diagonalisable matrix has a basis — its\n"
    "eigenvectors — in which it does nothing but *stretch* each axis. The unit\n"
    "eigen-cell stretches by λ along each eigenvector, with no rotation or shear\n"
    "(`diagonalise`)."),
   ("rref",
    "Gaussian elimination, animated: an augmented matrix `[A | b]` is reduced to\n"
    "reduced row-echelon form one row operation at a time, the numbers transforming\n"
    "in place until the left block is the identity and the last column is the\n"
    "solution (`rref`)."),
   ("projection",
    "One idea, two faces: orthogonal **projection** drops a vector onto a subspace\n"
    "(the shadow is the closest point, the error meets the space at a right angle),\n"
    "and **least-squares** fits a line to data the same way — minimising the squared\n"
    "residuals (`project`, `leastsquares`)."),
   ("matrix", "A bracketed matrix, rows/columns addressable via tags."),
   ("matrix_addition", "Two matrices summed, cell by cell."),
   ("matrix_addition_plane", "The same sum, laid out on a coordinate plane."),
   ("linear_transform", "A 2x2 matrix shearing a grid + basis vectors."),
   ("table", "A ruled table; cells, rows, columns, labels all addressable."),
   ("table_braces", "A table annotated with braces."),
 ]),
 dict(slug="stats", title="Statistics & probability", intro="", sub=SUB, items=[
   ("statistics",
    "A guided lesson, not a feature demo: describe a dataset, meet the normal curve,\n"
    "then see *why* the bell is everywhere (the Central Limit Theorem). The stats\n"
    "companion to the linear-algebra lesson. Start here.",
    "statistics — the whole story in three ideas"),
   ("histogram",
    "The shape of a dataset: a list of numbers binned into bars that stagger in one\n"
    "at a time, with the mean marked and the range labelled (`histogram`). Paste your\n"
    "own numbers into the data string — grades, prices, heights, times."),
   ("summary",
    "Describe a dataset in one call: the numbers as dots on a number line, with the\n"
    "mean, median and mode marked, a ±1σ spread band, and readouts of the range,\n"
    "variance and standard deviation (`summary`). Central tendency and dispersion,\n"
    "together."),
   ("boxplot",
    "The five-number summary as a box-and-whisker: the box spans Q1→Q3 (its width is\n"
    "the interquartile range), a line marks the median, the whiskers reach the rest,\n"
    "and a value far outside is flagged as an outlier (`boxplot`)."),
   ("skew",
    "Which way does the tail point? A histogram with the mean and median marked and a\n"
    "labelled skewness — when the mean is dragged right of the median, the data is\n"
    "right-skewed (`skew`)."),
   ("bellcurve",
    "The normal (Gaussian) bell curve and the 68-95-99.7 rule: the bell draws in,\n"
    "then the ±1σ / ±2σ / ±3σ bands shade one at a time, showing that 68% of values\n"
    "fall within one standard deviation, 95% within two, and 99.7% within three\n"
    "(`bellcurve`, alias `gaussian`)."),
   ("clt",
    "The Central Limit Theorem — the flagship: however flat a single die is, the\n"
    "*average* of five dice, taken 1200 times, piles into a bell that hugs the normal\n"
    "curve (`clt`). Seeded, so it renders the same every time."),
   ("correlation",
    "Do two things move together? The scatter of paired data, the best-fit line, and\n"
    "the Pearson correlation `r` — near +1 a tight upward line, near −1 downward, near\n"
    "0 a shapeless blob (`correlation`)."),
   ("lln",
    "The Law of Large Numbers: flip a fair coin over and over and track the running\n"
    "proportion of heads. It swings wildly at first, then settles onto the true 0.5\n"
    "as the trials pile up (`lln`). Draw the curve in to watch it converge."),
   ("hypothesis",
    "Is a result surprising enough to be real? Under the null hypothesis the test\n"
    "statistic follows the standard normal; the observed z cuts off tails whose area\n"
    "is the p-value. Smaller than α, reject (`hypothesis`)."),
   ("covariance",
    "Covariance as signed area: a cross at the means, and a rectangle from each point\n"
    "to the centre — cyan where x and y agree, magenta where they disagree. Their\n"
    "balance is the covariance (`covariance`)."),
   ("bayes",
    "Bayesian updating: a prior belief about a coin's bias, the likelihood from the\n"
    "data, and the posterior that combines them — pulled toward the evidence and\n"
    "sharpening as it accumulates (`bayes`)."),
   ("probability",
    "A probability & sampling playground in four chapters: named distributions\n"
    "(uniform / exponential / binomial / Poisson), a confidence interval, a\n"
    "Monte-Carlo estimate of π, and a random walk (`distribution`, `confidence`,\n"
    "`montecarlo`, `randomwalk`)."),
 ]),
 dict(slug="vectors", title="Vectors, fields & coordinates", intro="", sub=SUB, items=[
   ("vector_field", "A magnitude-coloured vector field."),
   ("coordinates", "Axes, planes, number lines, polar & complex planes."),
   ("pie", "A pie chart built from sectors."),
 ]),
 dict(slug="geometry", title="Geometry (olympiad)",
   intro="Every construction is **live** — the derived points recompute as the inputs move.",
   sub=SUB, items=[
   ("equilateral", "Euclid I.1 — an equilateral triangle from two circles."),
   ("triangle", "A triangle with its centres and cevians."),
   ("incircle_tangents", "The incircle and its tangent points."),
   ("tangents", "Tangent lines from a point to a circle."),
   ("orthocenter", "The orthocentre from the three altitudes."),
   ("euler_line", "The Euler line through centroid, circumcentre, orthocentre."),
   ("nine_point", "The nine-point circle."),
   ("conics", "Ellipse, parabola, hyperbola."),
 ]),
 dict(slug="transforms", title="Transforms & morphing", intro="", sub=SUB, items=[
   ("transforms", "Apply a 2x2 matrix (ApplyMatrix) to a group."),
   ("transform_copy", "Duplicate an entity, then transform the copy."),
   ("morph", "A sampled-point shape morph from A to B."),
 ]),
 dict(slug="text", title="Text & UI", intro="", sub=SUB, items=[
   ("typewriter", "Text revealed character by character."),
   ("captions", "Karaoke / word-pop caption modes."),
   ("terminal_boot", "The neon terminal template booting up."),
   ("brace", "The curly-brace family."),
   ("banner", "The manic logo / banner reveal."),
 ]),
 dict(slug="generative", title="Generative & recursive", intro="", sub=SUB, items=[
   ("fractal_tree", "One recursive `def`, drawn to depth 12."),
   ("hue_wave", "An animated hue wave across a grid."),
   ("hill_run", "A little scene animated with the language layer."),
   ("equal_cuts", "A circle halved again and again (pizza cuts)."),
   ("archimedes_pi", "Bounding pi with inscribed / circumscribed polygons."),
   ("pieday",
    "A Pi Day card: a rainbow petal-flower built from a loop of circles, radial rays,\n"
    "the digits of π, and the definition `circumference / diameter = pi`."),
 ]),
 dict(slug="boolean", title="Boolean shapes", intro="", sub=SUB, items=[
   ("boolean", "Union / intersection / difference of shapes."),
 ]),
 dict(slug="3d", title="3D scenes", intro="", sub=SUB_3D, items=[
   ("three_d", "Cubes, spheres, arrows, a curve, a surface and solids together — the 3D basics on one stage."),
   ("solids3", "Filled, shaded solids: a prism, a cone, and a lathed vase."),
   ("param3", "Parametric surfaces a height field can't make — a torus, a sphere, and a Möbius strip."),
   ("extrude3", "Lifting flat shapes into solids, including a boolean cut-out (a plate with a hole) and an L-beam."),
   ("morph3", "Morphing across families — a cube into a sphere, a saddle into a bowl, a helix into a ring."),
   ("linear-algebra-3d",
    "The 3D companion to the `linear-algebra` lesson: one matrix\n"
    "`[[1,0,0],[0,3,1],[0,1,3]]` (det 8; eigenvalues 1, 2, 4) seen two ways on an\n"
    "orbiting stage — first as a transformation (the unit cube → a parallelepiped\n"
    "whose volume is the determinant), then through its eigenvectors (the invariant\n"
    "axes that only stretch). Start here for 3D.",
    "linear-algebra-3d — the essence, in 3D"),
   ("linear-map3",
    "Linear algebra in 3D: a 3×3 matrix deforms the unit cube into a parallelepiped,\n"
    "with basis arrows i/j/k landing on the matrix's columns and the enclosed volume\n"
    "labelled as the determinant (`linmap3`). The 3D echo of `linear-map`."),
   ("eigen3",
    "The real eigenvectors of a 3×3 matrix, in 3D: the invariant lines through the\n"
    "origin that only stretch (by λ) when the matrix acts (`eigen3`). The 3D echo of\n"
    "`eigen`. A symmetric matrix gives three perpendicular real eigen-axes; a rotation\n"
    "leaves one real axis and two complex eigenvalues."),
   ("matrix3", "A 3×3×3 block of cubes, with a shear matrix **M** and its inverse **M⁻¹** applied and undone."),
   ("double-integral3",
    "Multivariable calculus: the volume under a surface as a limit of finer and\n"
    "finer columns — a double integral, made solid. The coarse blocks refine until\n"
    "they hug the surface."),
 ]),
]

# Non-gallery nav entries, above and below the nested Examples-gallery list.
SUMMARY_PRE = [
 "- [Getting started](getting-started.md)",
 "- [Shapes — the cast](shapes.md)",
 "- [Verbs — bringing it to life](verbs.md)",
 "- [Timing — par, seq & stagger](timing.md)",
 "- [Colour & style](colour.md)",
 "- [The language layer](language-layer.md)",
 "- [Kits — math, geometry, algorithms](kits.md)",
 "- [Going 3D](3d.md)",
 "- [Examples gallery](examples.md)",
]
SUMMARY_POST = [
 "- [Troubleshooting](troubleshooting.md)",
]

# section pages
for sec in SECTIONS:
    lines = [f"# {sec['title']}\n"]
    if sec["intro"]:
        lines.append(sec["intro"] + "\n")
    lines.append(sec["sub"] + "\n")
    for item in sec["items"]:
        name, desc = item[0], item[1]
        header = item[2] if len(item) > 2 else name
        lines.append(f"## {header}\n")
        lines.append(desc + "\n")
        lines.append("```manic")
        lines.append(f"{{{{#include ../../examples/{name}.manic}}}}")
        lines.append("```\n")
        lines.append(f'<div class="manic-video" data-video="ex-{name}"></div>\n')
    (SRC / f"ex-{sec['slug']}.md").write_text("\n".join(lines))

# index
idx = ["# Examples gallery\n",
       "Every animation in `examples/`, by topic — **the code and the clip for each**. "
       "Run any of them with `manic examples/<name>.manic`. Project: <https://8gwifi.org/manic>.\n"]
for sec in SECTIONS:
    n = len(sec["items"])
    idx.append(f"- [{sec['title']}](ex-{sec['slug']}.md) — {n} example{'s' if n != 1 else ''}")
(SRC / "examples.md").write_text("\n".join(idx) + "\n")

# SUMMARY (nest sections under the gallery)
summ = ["# Summary\n", "[Introduction](introduction.md)\n"] + SUMMARY_PRE
for sec in SECTIONS:
    summ.append(f"    - [{sec['title']}](ex-{sec['slug']}.md)")
summ += SUMMARY_POST
(SRC / "SUMMARY.md").write_text("\n".join(summ) + "\n")

total = sum(len(s["items"]) for s in SECTIONS)
print(f"generated {len(SECTIONS)} section pages ({total} examples) + index + SUMMARY")
