//! Shared "did you mean" suggestions for animation/reference errors: given an id
//! the author typed that doesn't exist, find the nearest real entity id or tag by
//! edit distance. Used by lowering (the parse-time verb check) and by
//! `Movie::validate` (the `manic check` whole-file pass).

use crate::scene::Scene;

/// Every name an animation could legitimately target: entity ids + their tags
/// (2D and 3D), sorted and de-duplicated — the candidate pool for suggestions.
pub fn candidate_names(scene: &Scene) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut set: BTreeSet<String> = BTreeSet::new();
    for e in &scene.entities {
        set.insert(e.id.clone());
        set.extend(e.tags.iter().cloned());
    }
    for e in &scene.entities_3d {
        set.insert(e.id.clone());
        set.extend(e.tags.iter().cloned());
    }
    set.into_iter().collect()
}

/// Nearest candidate to `target` by Levenshtein distance, but only when it's a
/// *plausible* fix — within `max(2, len/3)` edits — so we suggest `gost`→`ghost`
/// or `hedaer`→`header`, never an unrelated id.
pub fn nearest_name(target: &str, candidates: &[String]) -> Option<String> {
    let budget = (target.chars().count() / 3).max(2);
    candidates
        .iter()
        .map(|c| (levenshtein(target, c), c))
        .filter(|(d, _)| *d <= budget)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c.clone())
}

/// Classic Levenshtein edit distance over chars (two-row DP).
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::levenshtein;
    #[test]
    fn edit_distance_basics() {
        assert_eq!(levenshtein("header", "header"), 0);
        assert_eq!(levenshtein("hedaer", "header"), 2); // transposition = 2 edits
        assert_eq!(levenshtein("gost", "ghost"), 1);
    }
}
