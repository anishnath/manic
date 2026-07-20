//! Stable resolution for production-bundled and caller-provisioned assets.

use std::path::{Component, Path, PathBuf};

const SYSTEM_ASSET_ROOT: &str = "/usr/local/share/manic/assets";

fn candidate_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(root) = std::env::var_os("MANIC_ASSETS_DIR") {
        roots.push(PathBuf::from(root));
    }

    // This is the documented production install location. Keep it explicit:
    // CI and backend wrappers may execute a mounted/copy-staged binary from a
    // path such as /d/manic even though the shared catalog is installed here.
    roots.push(PathBuf::from(SYSTEM_ASSET_ROOT));

    if let Ok(executable) = std::env::current_exe() {
        if let Some(bin) = executable.parent() {
            roots.push(bin.join("assets"));
            roots.push(bin.join("../share/manic/assets"));
        }
    }
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
    roots.push(PathBuf::from("assets"));
    roots
}

/// Resolve an `asset:...` URI independently of the caller's working directory.
/// Ordinary paths pass through unchanged so a UI/backend can provision its own
/// files. Production installs the repository `assets/` tree under
/// `/usr/local/share/manic/assets`; development also finds the checkout tree.
pub(crate) fn resolve(path: &str) -> Result<PathBuf, String> {
    let Some(relative) = path.strip_prefix("asset:") else {
        return Ok(PathBuf::from(path));
    };
    let relative = Path::new(relative.trim_start_matches('/'));
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("bundled asset URI must be a relative path without `..`".into());
    }

    for root in candidate_roots() {
        let candidate = root.join(relative);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(format!(
        "bundled asset `{path}` is not installed; expected it under MANIC_ASSETS_DIR or /usr/local/share/manic/assets"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_asset_root_is_not_dependent_on_executable_location() {
        assert!(candidate_roots().contains(&PathBuf::from(SYSTEM_ASSET_ROOT)));
    }

    #[test]
    fn ordinary_paths_are_not_rewritten() {
        assert_eq!(
            resolve("uploads/model.obj").unwrap(),
            PathBuf::from("uploads/model.obj")
        );
    }
}
