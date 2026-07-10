//! The **builtin catalog** — machine-readable specs for every manic builtin,
//! plus the fixed vocabularies (colors, easings, presets, reserved vars,
//! keywords). This is what powers editor **highlighting** (classify a name as a
//! builtin), **autocomplete** (offer names + type-appropriate values), and
//! **quick-fixes** (nearest-name suggestions).
//!
//! The engine keeps this honest: a test in the `manic` crate asserts that the
//! catalog's name set equals `Registry::builtins()`, so a new builtin can't ship
//! without a catalog entry (and vice-versa) — no drift.

/// What a builtin is: a constructor/modifier (t=0), a timeline verb, or a
/// stateful (mutating) verb.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Ctor,
    Verb,
    MutVerb,
}

/// The argument type an editor should offer/validate for a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    /// A fresh entity id the author invents (a ctor's first arg).
    Name,
    /// An existing entity id or tag (a verb's target, a geo reference point).
    Ident,
    Num,
    Str,
    /// A `(x, y)` point literal.
    Point,
    /// A palette colour name.
    Color,
    /// An easing name.
    Ease,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: &'static str,
    pub ty: Ty,
    pub optional: bool,
}

#[derive(Debug, Clone)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub kind: Kind,
    /// Which kit provides it (`std` / `math` / `geo` / `algo` / `brand`).
    pub kit: &'static str,
    pub summary: &'static str,
    pub params: Vec<Param>,
}

fn spec(
    name: &'static str,
    kind: Kind,
    kit: &'static str,
    summary: &'static str,
    params: &[(&'static str, Ty, bool)],
) -> BuiltinSpec {
    BuiltinSpec {
        name,
        kind,
        kit,
        summary,
        params: params
            .iter()
            .map(|&(name, ty, optional)| Param { name, ty, optional })
            .collect(),
    }
}

/// Every builtin, with signatures for the common ones (the long tail carries
/// name/kind/kit/summary — enough for highlighting + name completion — and gains
/// params over time).
pub fn catalog() -> Vec<BuiltinSpec> {
    use Kind::*;
    use Ty::*;
    // (name, ty, optional)
    const O: bool = true;
    const R: bool = false;
    vec![
        // ---- std: shapes ----
        spec("circle", Ctor, "std", "a circle", &[("id", Name, R), ("center", Point, R), ("r", Num, R)]),
        spec("rect", Ctor, "std", "a rectangle", &[("id", Name, R), ("center", Point, R), ("w", Num, R), ("h", Num, R)]),
        spec("line", Ctor, "std", "a line segment", &[("id", Name, R), ("from", Point, R), ("to", Point, R)]),
        spec("arrow", Ctor, "std", "an arrow", &[("id", Name, R), ("from", Point, R), ("to", Point, R)]),
        spec("dot", Ctor, "std", "a small filled dot", &[("id", Name, R), ("at", Point, R), ("r", Num, O)]),
        spec("text", Ctor, "std", "a text label", &[("id", Name, R), ("at", Point, R), ("text", Str, R)]),
        spec("label", Ctor, "std", "a label pinned to an entity", &[]),
        spec("caption", Ctor, "std", "a row of caption words (karaoke/pop)", &[("id", Name, R), ("at", Point, R), ("words", Str, R)]),
        spec("counter", Ctor, "std", "a live numeric readout", &[("id", Name, R), ("at", Point, R), ("value", Num, R), ("decimals", Num, R), ("prefix", Str, R), ("suffix", Str, R)]),
        spec("cursor", Ctor, "std", "give a text entity a typewriter cursor", &[("id", Ident, R)]),
        spec("morph", Ctor, "std", "sampled-point shape morph a->b", &[]),
        spec("copy", Ctor, "std", "duplicate an entity", &[("new", Name, R), ("src", Ident, R)]),
        // ---- std: modifiers (t=0) ----
        spec("color", Ctor, "std", "set fill/stroke colour", &[("id", Ident, R), ("color", Color, R)]),
        spec("outline", Ctor, "std", "set outline colour", &[("id", Ident, R), ("color", Color, R)]),
        spec("size", Ctor, "std", "set text size", &[("id", Ident, R), ("size", Num, R)]),
        spec("stroke", Ctor, "std", "set stroke width", &[("id", Ident, R), ("width", Num, R)]),
        spec("glow", Ctor, "std", "set neon glow amount", &[("id", Ident, R), ("amount", Num, R)]),
        spec("opacity", Ctor, "std", "set opacity 0..1", &[("id", Ident, R), ("value", Num, R)]),
        spec("hue", Ctor, "std", "drive colour by an HSL hue", &[("id", Ident, R), ("degrees", Num, R), ("s", Num, O), ("l", Num, O)]),
        spec("rot", Ctor, "std", "set rotation (degrees)", &[("id", Ident, R), ("degrees", Num, R)]),
        spec("z", Ctor, "std", "set draw order", &[("id", Ident, R), ("z", Num, R)]),
        spec("tag", Ctor, "std", "add a group tag", &[("id", Ident, R), ("tag", Name, R)]),
        spec("bold", Ctor, "std", "use the bold mono font", &[("id", Ident, R)]),
        spec("display", Ctor, "std", "mark visible", &[("id", Ident, R)]),
        spec("hidden", Ctor, "std", "start hidden (opacity 0)", &[("id", Ident, R)]),
        spec("filled", Ctor, "std", "turn the fill on", &[("id", Ident, R)]),
        spec("outlined", Ctor, "std", "turn the outline on", &[("id", Ident, R)]),
        spec("untraced", Ctor, "std", "start undrawn, ready for draw-on", &[("id", Ident, R)]),
        // ---- std: boolean ops ----
        spec("union", Ctor, "std", "boolean union of two shapes", &[]),
        spec("intersect", Ctor, "std", "boolean intersection", &[]),
        spec("intersection", Ctor, "std", "boolean intersection", &[]),
        spec("difference", Ctor, "std", "boolean difference a-b", &[]),
        spec("subtract", Ctor, "std", "boolean difference a-b", &[]),
        spec("exclusion", Ctor, "std", "boolean symmetric difference", &[]),
        spec("xor", Ctor, "std", "boolean symmetric difference", &[]),
        // ---- std: braces ----
        spec("brace", Ctor, "std", "a curly brace", &[]),
        spec("bracelabel", Ctor, "std", "a brace with a label", &[]),
        spec("bracetext", Ctor, "std", "a brace's text label", &[]),
        // ---- std: verbs ----
        spec("draw", Verb, "std", "trace a stroke on", &[("id", Ident, R), ("dur", Num, O)]),
        spec("erase", Verb, "std", "reverse of draw", &[("id", Ident, R), ("dur", Num, O)]),
        spec("show", Verb, "std", "fade in", &[("id", Ident, R), ("dur", Num, O)]),
        spec("fade", Verb, "std", "fade out", &[("id", Ident, R), ("dur", Num, O)]),
        spec("flash", Verb, "std", "flash a colour, then restore", &[("id", Ident, R), ("color", Color, O), ("dur", Num, O), ("ease", Ease, O)]),
        spec("recolor", Verb, "std", "permanently change colour", &[("id", Ident, R), ("color", Color, R), ("dur", Num, O)]),
        spec("pulse", Verb, "std", "grow-and-settle attention pulse", &[("id", Ident, R), ("dur", Num, O)]),
        spec("shake", Verb, "std", "horizontal shake (error gesture)", &[("id", Ident, R), ("dur", Num, O)]),
        spec("spin", Verb, "std", "spin about the centre", &[("id", Ident, R), ("degrees", Num, O), ("dur", Num, O)]),
        spec("move", Verb, "std", "move to an absolute point", &[("id", Ident, R), ("to", Point, R), ("dur", Num, O), ("ease", Ease, O)]),
        spec("shift", Verb, "std", "move by a delta", &[("id", Ident, R), ("by", Point, R), ("dur", Num, O), ("ease", Ease, O)]),
        spec("grow", Verb, "std", "animate a line/arrow endpoint", &[("id", Ident, R), ("to", Point, R), ("dur", Num, O)]),
        spec("scale", Verb, "std", "animate scale to a factor", &[("id", Ident, R), ("factor", Num, R), ("dur", Num, O)]),
        spec("rotate", Verb, "std", "animate rotation", &[("id", Ident, R), ("degrees", Num, R), ("dur", Num, O)]),
        spec("say", Verb, "std", "crossfade text to new content", &[("id", Ident, R), ("text", Str, R), ("dur", Num, O)]),
        spec("type", Verb, "std", "typewriter reveal", &[("id", Ident, R)]),
        spec("to", Verb, "std", "animate any property to a value", &[("id", Ident, R), ("prop", Name, R), ("value", Num, R), ("dur", Num, O), ("ease", Ease, O)]),
        spec("set", Verb, "std", "alias of `to`", &[("id", Ident, R), ("prop", Name, R), ("value", Num, R), ("dur", Num, O), ("ease", Ease, O)]),
        spec("transform", Verb, "std", "apply a 2x2 matrix (ApplyMatrix)", &[("id", Ident, R), ("origin", Point, R), ("a", Num, R), ("b", Num, R), ("c", Num, R), ("d", Num, R), ("dur", Num, O), ("ease", Ease, O)]),
        spec("swap", MutVerb, "std", "swap two entities, or array slots i,j", &[("a", Ident, R), ("b", Ident, R), ("dur", Num, O)]),
        spec("cam", Verb, "std", "pan the camera to a point", &[("to", Point, R), ("dur", Num, O)]),
        spec("zoom", Verb, "std", "zoom the camera", &[("factor", Num, R), ("dur", Num, O)]),
        spec("karaoke", Verb, "std", "highlight caption words in sequence", &[]),
        spec("wordpop", Verb, "std", "pop caption words in one at a time", &[]),
        // ---- math ----
        spec("axes", Ctor, "math", "a coordinate frame", &[("id", Name, R), ("center", Point, R), ("halfw", Num, R), ("halfh", Num, R)]),
        spec("plot", Ctor, "math", "y = f(x) as a curve", &[("id", Name, R), ("center", Point, R), ("sx", Num, R), ("sy", Num, R), ("formula", Str, R)]),
        spec("vector", Ctor, "math", "an arrow from an origin", &[("id", Name, R), ("origin", Point, R), ("delta", Point, R)]),
        spec("numberline", Ctor, "math", "a labelled number line", &[]),
        spec("numberplane", Ctor, "math", "a gridded coordinate plane", &[]),
        spec("plane", Ctor, "math", "a coordinate plane", &[]),
        spec("complexplane", Ctor, "math", "the complex plane", &[]),
        spec("polarplane", Ctor, "math", "a polar grid", &[]),
        spec("matrix", Ctor, "math", "a bracketed matrix", &[("id", Name, R), ("data", Str, R), ("center", Point, R)]),
        spec("table", Ctor, "math", "a ruled table", &[("id", Name, R), ("data", Str, R), ("center", Point, R)]),
        spec("mathtable", Ctor, "math", "a table of math expressions", &[]),
        spec("integertable", Ctor, "math", "a table of integers", &[]),
        spec("decimaltable", Ctor, "math", "a table of decimals", &[]),
        spec("arc", Ctor, "math", "a circular arc", &[]),
        spec("sector", Ctor, "math", "a pie sector", &[]),
        spec("annulus", Ctor, "math", "a ring / annular sector", &[]),
        spec("pie", Ctor, "math", "a pie chart", &[]),
        spec("arrowfield", Ctor, "math", "a named vector field", &[]),
        spec("vectorfield", Ctor, "math", "a vector field", &[]),
        // ---- geo ----
        spec("point", Ctor, "geo", "a labelled point", &[("id", Name, R), ("at", Point, R), ("label", Str, O)]),
        spec("segment", Ctor, "geo", "segment between two points", &[("id", Name, R), ("a", Ident, R), ("b", Ident, R)]),
        spec("midpoint", Ctor, "geo", "midpoint of two points", &[("id", Name, R), ("a", Ident, R), ("b", Ident, R)]),
        spec("centroid", Ctor, "geo", "centroid of a triangle", &[]),
        spec("circumcenter", Ctor, "geo", "circumcentre", &[]),
        spec("incenter", Ctor, "geo", "incentre", &[]),
        spec("orthocenter", Ctor, "geo", "orthocentre", &[]),
        spec("foot", Ctor, "geo", "foot of a perpendicular", &[]),
        spec("meet", Ctor, "geo", "line-line intersection", &[]),
        spec("linecircle", Ctor, "geo", "line-circle intersection", &[]),
        spec("circlecircle", Ctor, "geo", "circle-circle intersection", &[]),
        spec("tangent", Ctor, "geo", "tangent touch points", &[]),
        spec("reflect", Ctor, "geo", "reflect a point over a line", &[]),
        spec("bisector", Ctor, "geo", "angle bisector", &[]),
        spec("rotpoint", Ctor, "geo", "rotate a point about another", &[]),
        spec("between", Ctor, "geo", "a point between two others", &[]),
        spec("anglepoint", Ctor, "geo", "a point at an angle", &[]),
        spec("circumcircle", Ctor, "geo", "circumscribed circle", &[]),
        spec("incircle", Ctor, "geo", "inscribed circle", &[]),
        spec("circle2", Ctor, "geo", "circle from centre + a point on it", &[]),
        spec("ellipse", Ctor, "geo", "an ellipse", &[]),
        spec("parabola", Ctor, "geo", "a parabola", &[]),
        spec("hyperbola", Ctor, "geo", "a hyperbola", &[]),
        spec("fullline", Ctor, "geo", "an infinite line through two points", &[]),
        spec("anglemark", Ctor, "geo", "an angle arc mark", &[]),
        spec("rightangle", Ctor, "geo", "a right-angle square mark", &[]),
        // ---- algo ----
        spec("graph", Ctor, "algo", "a node/edge graph (weights via a-b:w)", &[("id", Name, R), ("verts", Str, R), ("edges", Str, R), ("layout", Name, R), ("center", Point, R), ("scale", Num, R), ("radius", Num, O)]),
        spec("array", Ctor, "algo", "a row of value cells in slot boxes", &[("id", Name, R), ("vals", Str, R), ("center", Point, R), ("cw", Num, O), ("ch", Num, O)]),
        spec("list", Ctor, "algo", "a linked list (singly/doubly/circular)", &[("id", Name, R), ("vals", Str, R), ("center", Point, R), ("kind", Name, O), ("cw", Num, O), ("ch", Num, O)]),
        spec("stack", Ctor, "algo", "a stack (LIFO, grows up)", &[("id", Name, R), ("center", Point, R), ("cw", Num, O), ("ch", Num, O)]),
        spec("queue", Ctor, "algo", "a queue (FIFO, grows right)", &[("id", Name, R), ("center", Point, R), ("cw", Num, O), ("ch", Num, O)]),
        spec("hashmap", Ctor, "algo", "n buckets with separate chaining", &[("id", Name, R), ("n", Num, R), ("center", Point, R), ("ew", Num, O), ("ch", Num, O)]),
        spec("pointer", Ctor, "algo", "an index caret under an array slot", &[("id", Name, R), ("arr", Ident, R), ("slot", Num, R), ("label", Str, O)]),
        spec("caret", Ctor, "algo", "a labelled triangle marker", &[("id", Name, R), ("at", Point, R), ("label", Str, R), ("dir", Name, O)]),
        spec("compare", Verb, "algo", "flash the values in two array slots", &[("arr", Ident, R), ("i", Num, R), ("j", Num, R), ("color", Color, O)]),
        spec("pointat", Verb, "algo", "slide an index pointer to a slot", &[("id", Ident, R), ("arr", Ident, R), ("slot", Num, R), ("dur", Num, O)]),
        spec("push", MutVerb, "algo", "push onto a stack", &[("id", Ident, R), ("value", Str, R), ("dur", Num, O)]),
        spec("pop", MutVerb, "algo", "pop the top of a stack", &[("id", Ident, R), ("dur", Num, O)]),
        spec("enqueue", MutVerb, "algo", "enqueue at the back", &[("id", Ident, R), ("value", Str, R), ("dur", Num, O)]),
        spec("dequeue", MutVerb, "algo", "dequeue from the front", &[("id", Ident, R), ("dur", Num, O)]),
        spec("insert", MutVerb, "algo", "splice a node into a list", &[("id", Ident, R), ("after", Num, R), ("value", Str, R), ("dur", Num, O)]),
        spec("remove", MutVerb, "algo", "unlink a list node", &[("id", Ident, R), ("index", Num, R), ("dur", Num, O)]),
        spec("put", MutVerb, "algo", "hash a key into a bucket + chain", &[("id", Ident, R), ("key", Str, R), ("val", Str, R), ("dur", Num, O)]),
        spec("get", Verb, "algo", "scan a bucket's chain for a key", &[("id", Ident, R), ("key", Str, R), ("dur", Num, O)]),
        spec("bfs", MutVerb, "algo", "breadth-first traversal (queue)", &[("g", Ident, R), ("start", Ident, R)]),
        spec("dfs", MutVerb, "algo", "depth-first traversal (stack)", &[("g", Ident, R), ("start", Ident, R)]),
        spec("dijkstra", MutVerb, "algo", "single-source shortest paths", &[("g", Ident, R), ("start", Ident, R)]),
        // ---- brand ----
        spec("banner", Ctor, "brand", "the manic logo/banner", &[("id", Name, R), ("center", Point, R), ("scale", Num, O)]),
        spec("watermark", Ctor, "brand", "a screen-fixed watermark", &[("id", Name, R), ("at", Point, R), ("text", Str, O)]),
    ]
}

/// Palette colour names (the only colours the DSL accepts).
pub const COLORS: &[&str] = &["fg", "void", "cyan", "magenta", "lime", "dim", "panel"];

/// Easing names accepted where a `[ease]` argument is allowed.
pub const EASINGS: &[&str] = &[
    "linear", "smooth", "inout", "in", "out", "overshoot", "back", "bounce", "elastic", "spring",
];

/// Build-time control keywords (handled by the parser, not the registry).
pub const KEYWORDS: &[&str] = &["let", "for", "if", "else", "def", "in", "sum", "prod", "min", "max"];

/// Reserved variable names — canvas dims + constants. Never valid entity ids.
pub const RESERVED_VARS: &[&str] = &["w", "h", "cx", "cy", "pi", "e", "tau"];

/// `canvas(...)` preset names.
pub const CANVAS_PRESETS: &[&str] = &[
    "16:9", "1080p", "4k", "square", "portrait", "4:3", "widescreen", "720p", "fullhd", "hd",
    "2160p", "1:1", "9:16", "vertical", "story", "reel",
];

/// `template(...)` / `--template` names (primary + aliases).
pub const TEMPLATES: &[&str] = &[
    "plain", "terminal", "paper", "blueprint", "blank", "clean", "neon", "shell", "print", "light",
    "blue",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_duplicate_builtins() {
        let mut names: Vec<_> = catalog().iter().map(|s| s.name).collect();
        names.sort();
        let n = names.len();
        names.dedup();
        assert_eq!(names.len(), n, "duplicate builtin in catalog");
    }

    #[test]
    fn params_are_optional_suffix() {
        // once a param is optional, the rest must be too (no required-after-optional)
        for s in catalog() {
            let mut seen_opt = false;
            for p in &s.params {
                if p.optional {
                    seen_opt = true;
                } else {
                    assert!(!seen_opt, "{}: required param `{}` after an optional one", s.name, p.name);
                }
            }
        }
    }
}
