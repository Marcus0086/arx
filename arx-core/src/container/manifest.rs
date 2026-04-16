use serde::{Deserialize, Serialize};

/// Reference to a chunk in the chunk table.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChunkRef {
    pub id: u64,     // index into ChunkTable
    pub u_size: u64, // uncompressed size of this chunk
}

/// A regular file stored in the archive.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub mode: u32,
    pub mtime: i64,
    pub u_size: u64,
    pub chunk_refs: Vec<ChunkRef>,
}

/// A directory stored in the archive.
#[derive(Serialize, Deserialize, Debug)]
pub struct DirEntry {
    pub path: String,
    pub mode: u32,
    pub mtime: i64,
}

/// A symbolic link stored in the archive.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SymlinkEntry {
    pub path: String,
    /// The path the symlink points to (restored verbatim).
    pub target: String,
    pub mode: u32,
    pub mtime: i64,
}

/// Archive-level metadata.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Meta {
    pub created: i64,
    pub tool: String,
    /// Human-readable archive label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Owner / creator identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Free-form notes embedded at creation time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// The root manifest — CBOR-serialized and stored in the manifest region.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Manifest {
    pub files: Vec<FileEntry>,
    pub dirs: Vec<DirEntry>,
    pub meta: Meta,
    /// Symbolic links (empty for archives created before v4).
    #[serde(default)]
    pub symlinks: Vec<SymlinkEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(m: &Manifest) -> Manifest {
        let mut buf = Vec::new();
        ciborium::ser::into_writer(m, &mut buf).unwrap();
        ciborium::de::from_reader(buf.as_slice()).unwrap()
    }

    #[test]
    fn test_roundtrip_with_all_fields() {
        let m = Manifest {
            files: vec![FileEntry {
                path: "hello.txt".into(),
                mode: 0o644,
                mtime: 1_700_000_000,
                u_size: 12,
                chunk_refs: vec![ChunkRef { id: 0, u_size: 12 }],
            }],
            dirs: vec![DirEntry {
                path: "subdir".into(),
                mode: 0o755,
                mtime: 0,
            }],
            meta: Meta {
                created: 1_700_000_000,
                tool: "arx 0.1.0".into(),
                label: Some("test archive".into()),
                owner: Some("alice".into()),
                notes: Some("integration test".into()),
            },
            symlinks: vec![SymlinkEntry {
                path: "link".into(),
                target: "hello.txt".into(),
                mode: 0o777,
                mtime: 0,
            }],
        };
        let back = roundtrip(&m);
        assert_eq!(back.files[0].path, "hello.txt");
        assert_eq!(back.meta.label.as_deref(), Some("test archive"));
        assert_eq!(back.meta.owner.as_deref(), Some("alice"));
        assert_eq!(back.symlinks[0].target, "hello.txt");
    }

    #[test]
    fn test_old_manifest_compat() {
        // Simulate deserializing a pre-symlink manifest (no symlinks field)
        let m = Manifest {
            files: vec![],
            dirs: vec![],
            meta: Meta::default(),
            symlinks: vec![],
        };
        let back = roundtrip(&m);
        assert!(
            back.symlinks.is_empty(),
            "missing symlinks field should default to empty"
        );
    }
}
