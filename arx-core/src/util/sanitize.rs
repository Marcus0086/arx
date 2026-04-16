use crate::error::{ArxError, Result};
use std::path::{Path, PathBuf};

/// Join `root` with `rel`, rejecting any path that would escape the root
/// (absolute paths, `..` components, backslash traversal).
pub fn safe_join(root: &Path, rel: &str) -> Result<PathBuf> {
    let p = Path::new(rel);
    if p.is_absolute() || rel.contains("../") || rel.contains("..\\") || rel == ".." {
        return Err(ArxError::Format(format!("unsafe path: {rel}")));
    }
    Ok(root.join(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_normal_path_allowed() {
        let root = Path::new("/tmp/out");
        let result = safe_join(root, "foo/bar.txt").unwrap();
        assert_eq!(result, Path::new("/tmp/out/foo/bar.txt"));
    }

    #[test]
    fn test_absolute_path_rejected() {
        let root = Path::new("/tmp/out");
        assert!(safe_join(root, "/etc/passwd").is_err());
    }

    #[test]
    fn test_dotdot_rejected() {
        let root = Path::new("/tmp/out");
        assert!(safe_join(root, "../secret").is_err());
        assert!(safe_join(root, "foo/../../../etc/passwd").is_err());
        assert!(safe_join(root, "..").is_err());
    }
}
