//! The **grid kit**: a first-class 2-D cell grid — the one primitive underneath
//! tilemaps, spatial pathfinding (space, not `graph`'s topology), cellular
//! automata and Wave Function Collapse. A *computational* kit that extends the
//! algo lineage (`array`/`graph`/`matrix`/`table`); this file + one line in
//! `default_registry`, zero core changes.
//!
//! Reuse, don't duplicate:
//! - cell addressing = `matrix`/`table` (`{id}.r{i}c{j}`, `{id}.lines`),
//! - pathfinding colours mirror the algo kit's `bfs`/`dijkstra`
//!   (discovered cyan → current magenta → done lime, live frontier/visited),
//! - CA/WFC pre-simulate at build time and replay with the physics/optics `run`.

use std::collections::VecDeque;

use macroquad::prelude::{Color, Vec2};

use crate::easing::Easing;
use crate::lang::diag::Error;
use crate::lang::lower::{Args, Registry};
use crate::primitives::{Align, Entity, FontKind, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TextEvent, TrackSpec, Value};

/// Readability cap (matching the ML/tensor kits): an oversized grid should fail
/// clearly at `manic check` rather than render an unreadable wall of cells.
pub const MAX_SIDE: usize = 40;

const C_DISCOVERED: Color = style::CYAN;
const C_CURRENT: Color = style::MAGENTA;
const C_DONE: Color = style::LIME;
const C_PATH: Color = style::GOLD;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellKind {
    #[default]
    Open,
    Wall,
    Start,
    Goal,
}

impl CellKind {
    fn parse(s: &str) -> Option<CellKind> {
        Some(match s {
            "open" => CellKind::Open,
            "wall" => CellKind::Wall,
            "start" => CellKind::Start,
            "goal" => CellKind::Goal,
            _ => return None,
        })
    }
    /// The fill colour that shows a cell's kind. `Open` is the empty panel; a
    /// `Wall` is a solid block; start/goal are fixed bright anchors.
    fn fill(self) -> Color {
        match self {
            CellKind::Open => style::PANEL,
            CellKind::Wall => style::DIM,
            CellKind::Start => style::LIME,
            CellKind::Goal => style::GOLD,
        }
    }
    /// A cell is "alive" for a cellular automaton when it is filled (a `Wall`).
    fn alive(self) -> bool {
        matches!(self, CellKind::Wall)
    }
    fn from_char(c: char) -> Option<CellKind> {
        Some(match c {
            '#' => CellKind::Wall,
            '.' => CellKind::Open,
            '@' => CellKind::Start,
            '*' => CellKind::Goal,
            _ => return None,
        })
    }
}

/// Build-time state for one grid. Visible cells are ordinary 2-D entities; this
/// only backs layout, mutation, pathfinding and the pre-simulated replay frames.
#[derive(Debug, Clone, Default)]
pub struct GridData {
    pub cols: usize,
    pub rows: usize,
    pub cellsize: f32,
    pub origin: Vec2, // top-left corner (x0, y0)
    pub diag: bool,   // 8-connectivity when true, else 4
    pub kinds: Vec<CellKind>, // row-major, len rows*cols
    /// Pre-simulated CA generations / WFC settling steps (each a full kind grid),
    /// replayed by `run`. The first entry is the state after the first `evolve`.
    pub frames: Vec<Vec<CellKind>>,
}

impl GridData {
    fn idx(&self, r: usize, c: usize) -> usize {
        r * self.cols + c
    }
    fn center(&self, r: usize, c: usize) -> Vec2 {
        Vec2::new(
            self.origin.x + (c as f32 + 0.5) * self.cellsize,
            self.origin.y + (r as f32 + 0.5) * self.cellsize,
        )
    }
    fn in_bounds(&self, r: isize, c: isize) -> bool {
        r >= 0 && c >= 0 && (r as usize) < self.rows && (c as usize) < self.cols
    }
    /// 4- or 8-connected open neighbours of (r, c).
    fn neighbors(&self, r: usize, c: usize) -> Vec<(usize, usize)> {
        let steps: &[(isize, isize)] = if self.diag {
            &[
                (-1, 0),
                (1, 0),
                (0, -1),
                (0, 1),
                (-1, -1),
                (-1, 1),
                (1, -1),
                (1, 1),
            ]
        } else {
            &[(-1, 0), (1, 0), (0, -1), (0, 1)]
        };
        let mut out = Vec::new();
        for &(dr, dc) in steps {
            let (nr, nc) = (r as isize + dr, c as isize + dc);
            if self.in_bounds(nr, nc) {
                let (nr, nc) = (nr as usize, nc as usize);
                if self.kinds[self.idx(nr, nc)] != CellKind::Wall {
                    out.push((nr, nc));
                }
            }
        }
        out
    }
}

fn cell_id(id: &str, r: usize, c: usize) -> String {
    format!("{id}.r{r}c{c}")
}

/// Apply a cell's kind to its visible entity (fill + outline).
fn style_cell(scene: &mut Scene, id: &str, r: usize, c: usize, kind: CellKind) {
    if let Some(e) = scene.get_mut(&cell_id(id, r, c)) {
        e.color = kind.fill();
        e.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 1.5,
            outline_color: Some(style::DIM),
        };
    }
}

// ============================ Tier 1: construction ============================

/// `grid(id, (cx,cy), cols, rows, [cellsize])` — an empty grid, OR
/// `grid(id, "spec", (cx,cy), cols, rows, [cellsize])` — seeded from a compact
/// ASCII layout (rows split by `;`): `# . . . ; . . # . ; @ . . *`.
fn c_grid(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    // Overload by the type of arg 1: a quoted spec seeds the grid, otherwise
    // arg 1 is the centre of an empty grid.
    let seeded = a.text(1).is_ok();
    let (spec, center, cols, rows, cellsize) = if seeded {
        (
            Some(a.text(1)?),
            a.pair(2)?,
            a.num(3)? as usize,
            a.num(4)? as usize,
            a.opt_num(5)?.unwrap_or(48.0),
        )
    } else {
        (
            None,
            a.pair(1)?,
            a.num(2)? as usize,
            a.num(3)? as usize,
            a.opt_num(4)?.unwrap_or(48.0),
        )
    };
    if cols == 0 || rows == 0 {
        return Err(Error::new("grid needs cols > 0 and rows > 0", a.span_of(0)));
    }
    if cols > MAX_SIDE || rows > MAX_SIDE {
        return Err(Error::new(
            format!("grid is {cols}×{rows}; the readable limit is {MAX_SIDE}×{MAX_SIDE} — split it into smaller grids"),
            a.span_of(0),
        ));
    }

    // Parse the optional ASCII spec into a kind grid (default all Open).
    let mut kinds = vec![CellKind::Open; rows * cols];
    if let Some(spec) = &spec {
        for (r, line) in spec.split(';').enumerate() {
            let chars: Vec<char> = line.split_whitespace().flat_map(|t| t.chars()).collect();
            for (c, &ch) in chars.iter().enumerate() {
                if r < rows && c < cols {
                    if let Some(k) = CellKind::from_char(ch) {
                        kinds[r * cols + c] = k;
                    }
                }
            }
        }
    }

    let totalw = cols as f32 * cellsize;
    let totalh = rows as f32 * cellsize;
    let origin = Vec2::new(center.x - totalw / 2.0, center.y - totalh / 2.0);

    let mut data = GridData {
        cols,
        rows,
        cellsize,
        origin,
        diag: false,
        kinds,
        frames: Vec::new(),
    };

    // cells
    let cells_tag = format!("{id}.cells");
    for r in 0..rows {
        for c in 0..cols {
            let kind = data.kinds[data.idx(r, c)];
            let mut e = Entity::new(
                cell_id(&id, r, c),
                Shape::Rect {
                    w: cellsize,
                    h: cellsize,
                },
                data.center(r, c),
                kind.fill(),
            );
            e.stroke = StrokeStyle {
                fill: true,
                outline: true,
                width: 1.5,
                outline_color: Some(style::DIM),
            };
            e.z = 1;
            e.tags = vec![
                cells_tag.clone(),
                format!("{id}.row{r}"),
                format!("{id}.col{c}"),
            ];
            s.add(e);
        }
    }
    // grid lines (outer included), reusing table's h{k}/v{k} convention
    let lines_tag = format!("{id}.lines");
    let line = |s: &mut Scene, lid: String, from: Vec2, to: Vec2| {
        let mut e = Entity::new(lid, Shape::Line { to }, from, style::DIM);
        e.stroke.width = 1.5;
        e.glow = 0.0;
        e.z = 2;
        e.tags = vec![lines_tag.clone()];
        s.add(e);
    };
    for k in 0..=rows {
        let y = origin.y + k as f32 * cellsize;
        line(
            s,
            format!("{id}.h{k}"),
            Vec2::new(origin.x, y),
            Vec2::new(origin.x + totalw, y),
        );
    }
    for k in 0..=cols {
        let x = origin.x + k as f32 * cellsize;
        line(
            s,
            format!("{id}.v{k}"),
            Vec2::new(x, origin.y),
            Vec2::new(x, origin.y + totalh),
        );
    }

    data.frames.clear();
    s.grids.insert(id, data);
    Ok(())
}

/// `neighbors(id, "4"|"8")` — connectivity mode (4-directional default, or
/// 8-directional allowing diagonals).
fn c_neighbors(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let mode = a.text(1)?;
    let diag = match mode.trim() {
        "4" => false,
        "8" => true,
        _ => {
            return Err(Error::new(
                "neighbors mode must be \"4\" or \"8\"",
                a.span_of(1),
            ))
        }
    };
    let g = s
        .grids
        .get_mut(&id)
        .ok_or_else(|| Error::new(format!("`{id}` is not a grid"), a.span_of(0)))?;
    g.diag = diag;
    Ok(())
}

// ============================= Tier 2: mutation ==============================

/// `setcell(id, r, c, "kind")` — recolour + retag one cell (build-time state, so
/// a following `evolve`/`collapse`/pathfinder sees it). `kind` ∈ wall/open/start/goal.
fn c_setcell(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let r = a.num(1)? as usize;
    let c = a.num(2)? as usize;
    let kind = CellKind::parse(&a.ident(3)?)
        .ok_or_else(|| Error::new("kind must be one of wall/open/start/goal", a.span_of(3)))?;
    let (rows, cols) = {
        let g = s
            .grids
            .get(&id)
            .ok_or_else(|| Error::new(format!("`{id}` is not a grid"), a.span_of(0)))?;
        (g.rows, g.cols)
    };
    if r >= rows || c >= cols {
        return Err(Error::new(
            format!("cell ({r},{c}) is outside the {rows}×{cols} grid `{id}`"),
            a.span_of(1),
        ));
    }
    if let Some(g) = s.grids.get_mut(&id) {
        let i = g.idx(r, c);
        g.kinds[i] = kind;
    }
    style_cell(s, &id, r, c, kind);
    Ok(())
}

/// `walls(id, "r,c r,c …")` — batch-set several cells to `wall` in one call.
fn c_walls(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let spec = a.text(1)?;
    let (rows, cols) = {
        let g = s
            .grids
            .get(&id)
            .ok_or_else(|| Error::new(format!("`{id}` is not a grid"), a.span_of(0)))?;
        (g.rows, g.cols)
    };
    // each token is "r,c" (commas are also whitespace to the lexer, so accept both)
    let nums: Vec<usize> = spec
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse::<usize>().ok())
        .collect();
    if nums.is_empty() || nums.len() % 2 != 0 {
        return Err(Error::new(
            "walls expects pairs `r,c r,c …`",
            a.span_of(1),
        ));
    }
    for pair in nums.chunks(2) {
        let (r, c) = (pair[0], pair[1]);
        if r >= rows || c >= cols {
            return Err(Error::new(
                format!("wall cell ({r},{c}) is outside the {rows}×{cols} grid `{id}`"),
                a.span_of(1),
            ));
        }
        if let Some(g) = s.grids.get_mut(&id) {
            let i = g.idx(r, c);
            g.kinds[i] = CellKind::Wall;
        }
        style_cell(s, &id, r, c, CellKind::Wall);
    }
    Ok(())
}

// =========================== Tier 3: pathfinding =============================

/// Read a `(col, row)` point argument as a bounds-checked `(row, col)` cell.
fn cell_arg(g: &GridData, a: &Args, i: usize) -> Result<(usize, usize), Error> {
    let p = a.pair(i)?;
    let (c, r) = (p.x.round() as isize, p.y.round() as isize);
    if !g.in_bounds(r, c) {
        return Err(Error::new(
            format!("cell ({}, {}) is outside the {}×{} grid", p.x, p.y, g.cols, g.rows),
            a.span_of(i),
        ));
    }
    Ok((r as usize, c as usize))
}

/// Shared readout: two counters below the grid — `frontier: N` and `visited: M`.
fn readouts(s: &mut Scene, id: &str, g: &GridData) -> (String, String) {
    let fid = format!("{id}.frontier");
    let vid = format!("{id}.visited");
    let x = g.origin.x;
    let ybase = g.origin.y + g.rows as f32 * g.cellsize;
    for (rid, dy, color, init) in [
        (&fid, 30.0, C_DISCOVERED, "frontier: 0"),
        (&vid, 62.0, C_DONE, "visited: 0"),
    ] {
        if !s.contains(rid) {
            let mut t = Entity::new(
                rid.clone(),
                Shape::Text {
                    content: init.to_string(),
                    size: 22.0,
                },
                Vec2::new(x, ybase + dy),
                color,
            );
            t.font = FontKind::MonoBold;
            t.align = Align::Left;
            t.z = 8;
            s.add(t);
        }
    }
    (fid, vid)
}

fn recolor(tracks: &mut Vec<TrackSpec>, id: &str, c: Color, at: f32, dur: f32) {
    tracks.push(TrackSpec {
        id: id.into(),
        prop: Prop::Color,
        target: TargetValue::Abs(Value::C(c)),
        start: at,
        dur,
        easing: Easing::OutQuad,
    });
}

/// `gridbfs(id, start, goal)` — unweighted BFS over open cells with the algo
/// kit's colour grammar (discovered cyan → current magenta → done lime) and a
/// live frontier/visited readout. Highlights the shortest route on arrival.
fn v_gridbfs(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let g = s
        .grids
        .get(&id)
        .ok_or_else(|| Error::new(format!("`{id}` is not a grid"), a.span_of(0)))?
        .clone();
    let start = cell_arg(&g, a, 1)?;
    let goal = cell_arg(&g, a, 2)?;
    let (fid, vid) = readouts(s, &id, &g);

    let mut tracks: Vec<TrackSpec> = Vec::new();
    let mut events: Vec<TextEvent> = Vec::new();
    let mut visited_n = 0usize;
    let mut frontier_n: usize;

    // BFS, tracking parents to reconstruct the path.
    let n = g.rows * g.cols;
    let mut parent: Vec<Option<usize>> = vec![None; n];
    let mut seen = vec![false; n];
    let mut q: VecDeque<(usize, usize)> = VecDeque::new();
    let si = g.idx(start.0, start.1);
    seen[si] = true;
    q.push_back(start);
    frontier_n = 1;

    let mut t = 0.4;
    let stepd = 0.14;
    recolor(&mut tracks, &cell_id(&id, start.0, start.1), C_DISCOVERED, 0.0, 0.2);
    events.push(TextEvent::text(fid.clone(), "frontier: 1".into(), 0.15));
    let mut found = false;

    while let Some((r, c)) = q.pop_front() {
        frontier_n = frontier_n.saturating_sub(1);
        // current pop → magenta pulse, then settle to done
        recolor(&mut tracks, &cell_id(&id, r, c), C_CURRENT, t, 0.15);
        recolor(&mut tracks, &cell_id(&id, r, c), C_DONE, t + 0.16, 0.2);
        visited_n += 1;
        events.push(TextEvent::text(vid.clone(), format!("visited: {visited_n}"), t + 0.05));
        if (r, c) == goal {
            found = true;
            t += stepd;
            break;
        }
        let mut sub = t + 0.04;
        for (nr, nc) in g.neighbors(r, c) {
            let ni = g.idx(nr, nc);
            if seen[ni] {
                continue;
            }
            seen[ni] = true;
            parent[ni] = Some(g.idx(r, c));
            q.push_back((nr, nc));
            frontier_n += 1;
            recolor(&mut tracks, &cell_id(&id, nr, nc), C_DISCOVERED, sub, 0.18);
            events.push(TextEvent::text(fid.clone(), format!("frontier: {frontier_n}"), sub + 0.02));
            sub += 0.03;
        }
        t = sub.max(t + stepd) + 0.02;
    }

    // reconstruct + flash the shortest path gold
    if found {
        let mut path = Vec::new();
        let mut cur = g.idx(goal.0, goal.1);
        loop {
            path.push(cur);
            match parent[cur] {
                Some(p) => cur = p,
                None => break,
            }
        }
        path.reverse();
        let mut pt = t + 0.2;
        for &ci in &path {
            let (r, c) = (ci / g.cols, ci % g.cols);
            recolor(&mut tracks, &cell_id(&id, r, c), C_PATH, pt, 0.16);
            pt += 0.06;
        }
        t = pt;
    }

    Ok(Clip {
        dur: t + 0.3,
        tracks,
        events,
    })
}

/// `gridastar(id, start, goal, ["manhattan"|"euclidean"|"diagonal"])` — A* with
/// the given heuristic; explored cells shade in, and the settled route is traced
/// in gold as `{id}.path` (a polyline you can `draw`).
fn v_gridastar(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let g = s
        .grids
        .get(&id)
        .ok_or_else(|| Error::new(format!("`{id}` is not a grid"), a.span_of(0)))?
        .clone();
    let start = cell_arg(&g, a, 1)?;
    let goal = cell_arg(&g, a, 2)?;
    let heuristic = if a.len() > 3 {
        a.ident(3)?
    } else {
        "manhattan".to_string()
    };
    let h = |r: usize, c: usize| -> f32 {
        let (dr, dc) = (
            (r as f32 - goal.0 as f32).abs(),
            (c as f32 - goal.1 as f32).abs(),
        );
        match heuristic.as_str() {
            "euclidean" => (dr * dr + dc * dc).sqrt(),
            "diagonal" => dr.max(dc) + (std::f32::consts::SQRT_2 - 1.0) * dr.min(dc),
            _ => dr + dc, // manhattan (default)
        }
    };
    if !matches!(heuristic.as_str(), "manhattan" | "euclidean" | "diagonal") {
        return Err(Error::new(
            "heuristic must be manhattan, euclidean or diagonal",
            a.span_of(3),
        ));
    }
    let (fid, vid) = readouts(s, &id, &g);

    let n = g.rows * g.cols;
    let mut gscore = vec![f32::INFINITY; n];
    let mut parent: Vec<Option<usize>> = vec![None; n];
    let mut closed = vec![false; n];
    // simple open set as a vector we scan for the min f (grids are capped small)
    let mut open: Vec<usize> = Vec::new();
    let si = g.idx(start.0, start.1);
    gscore[si] = 0.0;
    open.push(si);

    let mut tracks: Vec<TrackSpec> = Vec::new();
    let mut events: Vec<TextEvent> = Vec::new();
    let mut visited_n = 0usize;
    let mut t = 0.4;
    let stepd = 0.14;
    recolor(&mut tracks, &cell_id(&id, start.0, start.1), C_DISCOVERED, 0.0, 0.2);
    let mut found = false;

    while !open.is_empty() {
        // pop lowest f = g + h
        let (best_pos, &best) = open
            .iter()
            .enumerate()
            .min_by(|(_, &x), (_, &y)| {
                let fx = gscore[x] + h(x / g.cols, x % g.cols);
                let fy = gscore[y] + h(y / g.cols, y % g.cols);
                fx.partial_cmp(&fy).unwrap()
            })
            .unwrap();
        open.swap_remove(best_pos);
        if closed[best] {
            continue;
        }
        closed[best] = true;
        let (r, c) = (best / g.cols, best % g.cols);
        recolor(&mut tracks, &cell_id(&id, r, c), C_CURRENT, t, 0.14);
        recolor(&mut tracks, &cell_id(&id, r, c), C_DISCOVERED, t + 0.15, 0.2);
        visited_n += 1;
        events.push(TextEvent::text(vid.clone(), format!("visited: {visited_n}"), t + 0.05));
        if (r, c) == goal {
            found = true;
            t += stepd;
            break;
        }
        let mut sub = t + 0.04;
        for (nr, nc) in g.neighbors(r, c) {
            let ni = g.idx(nr, nc);
            if closed[ni] {
                continue;
            }
            let step_cost = if nr != r && nc != c {
                std::f32::consts::SQRT_2
            } else {
                1.0
            };
            let tentative = gscore[best] + step_cost;
            if tentative < gscore[ni] {
                gscore[ni] = tentative;
                parent[ni] = Some(best);
                open.push(ni);
                recolor(&mut tracks, &cell_id(&id, nr, nc), C_DISCOVERED, sub, 0.18);
                sub += 0.02;
            }
        }
        events.push(TextEvent::text(fid.clone(), format!("frontier: {}", open.len()), sub));
        t = sub.max(t + stepd) + 0.02;
    }

    // build {id}.path as a gold polyline through the route (author draws it)
    if found {
        let mut path_cells = Vec::new();
        let mut cur = g.idx(goal.0, goal.1);
        loop {
            path_cells.push(cur);
            match parent[cur] {
                Some(p) => cur = p,
                None => break,
            }
        }
        path_cells.reverse();
        let pts: Vec<Vec2> = path_cells
            .iter()
            .map(|&ci| g.center(ci / g.cols, ci % g.cols))
            .collect();
        let path_id = format!("{id}.path");
        if pts.len() >= 2 {
            let mut e = Entity::new(path_id.clone(), Shape::Polyline { pts }, Vec2::ZERO, C_PATH);
            e.stroke = StrokeStyle {
                fill: false,
                outline: true,
                width: 5.0,
                outline_color: Some(C_PATH),
            };
            e.trace = 0.0; // start undrawn so `draw(g.path, …)` animates it
            e.z = 6;
            e.tags = vec![path_id.clone()];
            s.add(e);
        }
    }

    Ok(Clip {
        dur: t + 0.3,
        tracks,
        events,
    })
}

// ===================== Tier 4: generation (pre-sim + replay) =================

/// A tiny deterministic PRNG (LCG) so a seeded `collapse` is reproducible, the
/// way the stats kit seeds its sampling.
struct Lcg(u64);
impl Lcg {
    fn unit(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 40) as f32) / ((1u32 << 24) as f32)
    }
}

/// Parse a Golly-style birth/survival rulestring (or `"life"` = `B3/S23`) into
/// per-neighbour-count birth/survive tables (index 0..=8).
fn parse_rule(rule: &str) -> Option<([bool; 9], [bool; 9])> {
    let rule = rule.trim();
    let rule = if rule.eq_ignore_ascii_case("life") {
        "B3/S23"
    } else {
        rule
    };
    let mut birth = [false; 9];
    let mut survive = [false; 9];
    for part in rule.split('/') {
        let (tag, digits) = part.split_at(1);
        let table = match tag {
            "B" | "b" => &mut birth,
            "S" | "s" => &mut survive,
            _ => return None,
        };
        for ch in digits.chars() {
            let d = ch.to_digit(10)? as usize;
            if d <= 8 {
                table[d] = true;
            }
        }
    }
    Some((birth, survive))
}

/// The state a `evolve`/`collapse` builds on: the latest pre-simulated frame, or
/// the base cell kinds if none yet.
fn latest(g: &GridData) -> Vec<CellKind> {
    g.frames.last().cloned().unwrap_or_else(|| g.kinds.clone())
}

/// `evolve(id, "life"|"B3/S23")` — pre-simulate ONE cellular-automaton generation
/// (alive = a filled `wall` cell, 8-neighbourhood) and append it as a replay
/// frame. Call it N times, then `run(id, N, dur)` to watch them play out.
/// (Named `evolve`, not `step`: `step` is the core timeline-stage block.)
fn c_evolve(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let rule = a.text(1)?;
    let (birth, survive) =
        parse_rule(&rule).ok_or_else(|| Error::new("rule must be \"life\" or a Golly rulestring like B3/S23", a.span_of(1)))?;
    let g = s
        .grids
        .get_mut(&id)
        .ok_or_else(|| Error::new(format!("`{id}` is not a grid"), a.span_of(0)))?;
    let (rows, cols) = (g.rows, g.cols);
    let cur = latest(g);
    let alive = |r: isize, c: isize| -> bool {
        r >= 0 && c >= 0 && (r as usize) < rows && (c as usize) < cols && cur[r as usize * cols + c as usize].alive()
    };
    let mut next = vec![CellKind::Open; rows * cols];
    for r in 0..rows {
        for c in 0..cols {
            let mut n = 0;
            for dr in -1..=1isize {
                for dc in -1..=1isize {
                    if (dr, dc) != (0, 0) && alive(r as isize + dr, c as isize + dc) {
                        n += 1;
                    }
                }
            }
            let now = cur[r * cols + c].alive();
            let next_alive = if now { survive[n] } else { birth[n] };
            next[r * cols + c] = if next_alive {
                CellKind::Wall
            } else {
                CellKind::Open
            };
        }
    }
    g.frames.push(next);
    Ok(())
}

/// `collapse(id, "tileset", [seed])` — pre-simulate a Wave-Function-Collapse-style
/// settling at build time: cells are decided row by row, each biased by its
/// already-decided neighbours (constraint propagation), recording one frame per
/// row so `run` can replay the grid resolving. Seeded ⇒ deterministic.
fn c_collapse(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let tileset = a.text(1)?;
    let seed = a.opt_num(2)?.unwrap_or(1.0) as u64;
    let (base, k) = match tileset.trim() {
        "islands" => (0.42f32, 0.15f32),
        _ => (0.30f32, 0.12f32), // "maze" / default: thinner, more connected walls
    };
    let g = s
        .grids
        .get_mut(&id)
        .ok_or_else(|| Error::new(format!("`{id}` is not a grid"), a.span_of(0)))?;
    let (rows, cols) = (g.rows, g.cols);
    let mut rng = Lcg(seed.wrapping_mul(2654435761).wrapping_add(1));
    let mut state = vec![CellKind::Open; rows * cols];
    g.frames.clear();
    for r in 0..rows {
        for c in 0..cols {
            // count already-decided wall neighbours (row above + left)
            let mut wn = 0;
            for (dr, dc) in [(-1isize, 0isize), (0, -1), (-1, -1), (-1, 1)] {
                let (nr, nc) = (r as isize + dr, c as isize + dc);
                if nr >= 0 && nc >= 0 && (nc as usize) < cols {
                    if state[nr as usize * cols + nc as usize] == CellKind::Wall {
                        wn += 1;
                    }
                }
            }
            let p = (base + k * wn as f32).clamp(0.05, 0.85);
            state[r * cols + c] = if rng.unit() < p {
                CellKind::Wall
            } else {
                CellKind::Open
            };
        }
        g.frames.push(state.clone());
    }
    Ok(())
}

/// `run(id, [gens], [dur])` — replay the pre-simulated CA generations / WFC
/// settling frames over `dur` seconds, recolouring the cells frame by frame.
/// Shared with the physics/optics `run`; dispatched here when `id` is a grid.
pub fn replay(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let g = s
        .grids
        .get(&id)
        .ok_or_else(|| Error::new(format!("`{id}` is not a grid"), a.span_of(0)))?;
    if g.frames.is_empty() {
        return Err(Error::new(
            format!("`{id}` has no pre-simulated frames — call `step(...)` or `collapse(...)` first"),
            a.span_of(0),
        ));
    }
    let avail = g.frames.len();
    let gens = a.opt_num(1)?.map(|n| (n as usize).min(avail)).unwrap_or(avail).max(1);
    let dur = a.opt_num(2)?.unwrap_or(4.0).max(0.2);
    // Hold the base grid for one slot, then step through each generation, so the
    // starting state is visible before it changes.
    let frame_dur = dur / (gens + 1) as f32;
    // states[0] = the base grid, then each replayed generation
    let mut prev = g.kinds.clone();
    let mut tracks: Vec<TrackSpec> = Vec::new();
    for step in 0..gens {
        let cur = &g.frames[step];
        let at = (step + 1) as f32 * frame_dur;
        for r in 0..g.rows {
            for c in 0..g.cols {
                let i = g.idx(r, c);
                if cur[i] != prev[i] {
                    recolor(&mut tracks, &cell_id(&id, r, c), cur[i].fill(), at, frame_dur * 0.6);
                }
            }
        }
        prev = cur.clone();
    }
    Ok(Clip {
        dur: dur + 0.2,
        tracks,
        events: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::CellKind;
    use crate::movie::Movie;

    fn movie(src: &str) -> Movie {
        crate::parse(src).unwrap_or_else(|e| panic!("parse failed: {e:?}"))
    }

    #[test]
    fn grid_builds_cells_and_lines() {
        let m = movie("canvas(\"16:9\");\ngrid(g, (640,360), 5, 4, 60);\n");
        let gd = &m.base().grids["g"];
        assert_eq!((gd.cols, gd.rows), (5, 4));
        assert!(m.base().get("g.r0c0").is_some());
        assert!(m.base().get("g.r3c4").is_some());
        assert!(m.base().get("g.h0").is_some() && m.base().get("g.v5").is_some());
    }

    #[test]
    fn seed_spec_marks_kinds() {
        let m = movie("canvas(\"16:9\");\ngrid(g, \"@ . # ; . . *\", (640,360), 3, 2, 60);\n");
        let k = &m.base().grids["g"].kinds;
        assert_eq!(k[0], CellKind::Start); // (0,0) = @
        assert_eq!(k[2], CellKind::Wall); //  (0,2) = #
        assert_eq!(k[5], CellKind::Goal); //  (1,2) = *
    }

    #[test]
    fn oversized_grid_is_rejected() {
        assert!(crate::parse("canvas(\"16:9\");\ngrid(g, (640,360), 41, 4);\n").is_err());
    }

    #[test]
    fn astar_traces_a_path() {
        let m = movie(
            "canvas(\"16:9\");\ngrid(g, \"@ . . # . ; . . . # . ; . # . . . ; . # . . *\", (640,360), 5, 4, 60);\ngridastar(g, (0,0), (4,3), manhattan);\n",
        );
        assert!(
            m.base().get("g.path").is_some(),
            "A* should build the g.path polyline"
        );
    }

    #[test]
    fn life_blinker_oscillates() {
        // a horizontal blinker on the middle row of a 3×3 flips to vertical
        let m = movie(
            "canvas(\"16:9\");\ngrid(l, (640,360), 3, 3, 60);\nsetcell(l,1,0,wall); setcell(l,1,1,wall); setcell(l,1,2,wall);\nevolve(l, \"life\");\n",
        );
        let f = &m.base().grids["l"].frames[0];
        assert_eq!(f[1], CellKind::Wall); // (0,1)
        assert_eq!(f[4], CellKind::Wall); // (1,1)
        assert_eq!(f[7], CellKind::Wall); // (2,1)
        assert_eq!(f[3], CellKind::Open); // (1,0)
        assert_eq!(f[5], CellKind::Open); // (1,2)
    }

    #[test]
    fn collapse_is_deterministic() {
        let src = "canvas(\"16:9\");\ngrid(g, (640,360), 8, 8, 40);\ncollapse(g, \"maze\", 42);\n";
        assert_eq!(
            movie(src).base().grids["g"].frames,
            movie(src).base().grids["g"].frames
        );
    }

    #[test]
    fn neighbors_sets_connectivity() {
        let m = movie("canvas(\"16:9\");\ngrid(g, (640,360), 4, 4);\nneighbors(g, \"8\");\n");
        assert!(m.base().grids["g"].diag);
    }
}

pub fn register(r: &mut Registry) {
    r.ctor("grid", c_grid);
    r.ctor("neighbors", c_neighbors);
    r.ctor("setcell", c_setcell);
    r.ctor("walls", c_walls);
    r.mut_verb("gridbfs", v_gridbfs);
    r.mut_verb("gridastar", v_gridastar);
    r.ctor("evolve", c_evolve);
    r.ctor("collapse", c_collapse);
    // `run` (replay) is registered by the physics kit and dispatches to
    // grid::replay when the target is a grid — so it is NOT re-registered here.
}
