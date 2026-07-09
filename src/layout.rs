//! Position helpers: compute `Vec2`s, feed them to the scene builder.
//! Pure functions — no engine state involved. Generic geometry, reusable by
//! any kit (an algo `tree`, a math lattice `grid`, a unit-circle `ring`).

use macroquad::prelude::Vec2;

/// `n` positions evenly spaced along the horizontal line `y`, spanning
/// `[x0, x1]` inclusive.
pub fn row(n: usize, y: f32, x0: f32, x1: f32) -> Vec<Vec2> {
    if n == 1 {
        return vec![Vec2::new((x0 + x1) / 2.0, y)];
    }
    (0..n)
        .map(|i| Vec2::new(x0 + (x1 - x0) * i as f32 / (n - 1) as f32, y))
        .collect()
}

/// `cols × rows` cell centres filling the rectangle `min..max`, row-major.
pub fn grid(cols: usize, rows: usize, min: Vec2, max: Vec2) -> Vec<Vec2> {
    let cw = (max.x - min.x) / cols as f32;
    let ch = (max.y - min.y) / rows as f32;
    (0..rows)
        .flat_map(|r| {
            (0..cols).map(move |c| {
                Vec2::new(min.x + cw * (c as f32 + 0.5), min.y + ch * (r as f32 + 0.5))
            })
        })
        .collect()
}

/// `n` positions on a circle, clockwise from 12 o'clock. The natural layout
/// for consistent-hash rings, circular buffers, and unit-circle diagrams.
pub fn ring(n: usize, center: Vec2, r: f32) -> Vec<Vec2> {
    (0..n)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / n as f32 - std::f32::consts::FRAC_PI_2;
            center + Vec2::new(a.cos(), a.sin()) * r
        })
        .collect()
}

/// Tree layout from parent links (`parents[i] = None` for roots).
/// Leaves get consecutive horizontal slots `dx` apart; internal nodes centre
/// over their children; depth `d` sits at `top.y + d * dy`. The whole forest
/// is centred on `top.x`.
pub fn tree(parents: &[Option<usize>], top: Vec2, dx: f32, dy: f32) -> Vec<Vec2> {
    let n = parents.len();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut roots = Vec::new();
    for (i, p) in parents.iter().enumerate() {
        match p {
            Some(p) => children[*p].push(i),
            None => roots.push(i),
        }
    }

    let mut x = vec![0.0f32; n];
    let mut depth = vec![0usize; n];
    let mut next_slot = 0.0f32;

    fn place(
        i: usize,
        d: usize,
        children: &[Vec<usize>],
        x: &mut [f32],
        depth: &mut [usize],
        next_slot: &mut f32,
    ) {
        depth[i] = d;
        if children[i].is_empty() {
            x[i] = *next_slot;
            *next_slot += 1.0;
            return;
        }
        for &c in &children[i] {
            place(c, d + 1, children, x, depth, next_slot);
        }
        let sum: f32 = children[i].iter().map(|&c| x[c]).sum();
        x[i] = sum / children[i].len() as f32;
    }

    for &r in &roots {
        place(r, 0, &children, &mut x, &mut depth, &mut next_slot);
    }

    let mid = (next_slot - 1.0).max(0.0) / 2.0;
    (0..n)
        .map(|i| Vec2::new(top.x + (x[i] - mid) * dx, top.y + depth[i] as f32 * dy))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_spans_inclusive() {
        let p = row(3, 100.0, 0.0, 200.0);
        assert_eq!(p[0].x, 0.0);
        assert_eq!(p[1].x, 100.0);
        assert_eq!(p[2].x, 200.0);
    }

    #[test]
    fn tree_centres_parent_over_children() {
        // 2 <- {0, 1}
        let p = tree(&[Some(2), Some(2), None], Vec2::new(0.0, 0.0), 100.0, 80.0);
        assert_eq!(p[2].x, (p[0].x + p[1].x) / 2.0);
        assert_eq!(p[2].y, 0.0);
        assert_eq!(p[0].y, 80.0);
    }
}
