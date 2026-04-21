use std::path::{Component, Path, PathBuf};

/// Reject any path component that isn't a plain filename segment.
/// Returns None if the path contains `..`, absolute roots, or is empty.
/// Used to prevent path traversal in file upload handlers.
pub fn sanitize_path(raw: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for comp in Path::new(raw).components() {
        match comp {
            Component::Normal(c) => out.push(c),
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}
