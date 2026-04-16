use crate::container::chunktab::{ChunkEntry, read_table_from_slice};
use crate::container::manifest::Manifest;
use crate::container::superblock::{FLAG_ENCRYPTED, Superblock};
use crate::container::tail::{TAIL_LEN, TAIL_MAGIC};
use crate::crypto::aead::{AeadKey, Region, derive_nonce};
use crate::error::Result;
use crate::util::buf::read_exact_at;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub path: String,
    pub u_size: u64,
    pub chunks: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct ChunkView {
    pub ordinal: u64,
    pub id: u64,
    pub codec: u8,
    pub file_off: u64,
    pub u_len: u64,
    pub c_len: u64,
    pub data_off: u64,
    pub pct_end: f32,
}

pub struct Opened {
    /// Thread-safe file handle for lock-free positional reads.
    pub f: Arc<File>,
    pub sb: Superblock,
    pub manifest: Manifest,
    pub table: Vec<ChunkEntry>,
    pub aead: Option<(AeadKey, [u8; 32])>,
    pub file_end_for_data: u64,
}

impl Opened {
    /// `key_salt` is accepted for backward API compatibility but the archive's own
    /// stored `kdf_salt` (from the superblock) is used for nonce derivation.
    pub fn open(path: &Path, aead_key: Option<[u8; 32]>, _key_salt: [u8; 32]) -> Result<Self> {
        let mut f = File::open(path)?;
        let file_len = f.metadata()?.len();

        let sb = Superblock::read_from(&mut f)?;
        let header_len = sb.header_len();
        let enc_enabled = (sb.flags & FLAG_ENCRYPTED) != 0;

        // Detect optional tail
        let mut file_end_for_data = file_len;
        if file_len >= TAIL_LEN {
            f.seek(SeekFrom::End(-(TAIL_LEN as i64)))?;
            let mut magic = [0u8; 8];
            if f.read_exact(&mut magic).is_ok() && magic == TAIL_MAGIC {
                file_end_for_data = file_len - TAIL_LEN;
            }
        }

        // Resolve key
        let resolved_key: Option<AeadKey> = if enc_enabled {
            if let Some(raw) = aead_key {
                Some(AeadKey(raw))
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "encrypted; --key/--key-salt required",
                )
                .into());
            }
        } else {
            None
        };
        // Use the archive's stored kdf_salt for nonce derivation
        let salt = sb.kdf_salt;

        // Manifest
        f.seek(SeekFrom::Start(header_len))?;
        let mut mbytes = vec![0u8; sb.manifest_len as usize];
        f.read_exact(&mut mbytes)?;
        let manifest_bytes = if let Some(ref key) = resolved_key {
            let nonce = derive_nonce(&salt, Region::Manifest, 0);
            crate::crypto::aead::open_whole(key, &nonce, b"manifest", &mbytes)?
        } else {
            mbytes
        };
        let manifest: Manifest = ciborium::de::from_reader(&manifest_bytes[..])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // Chunk table
        let table_ct_len = sb.data_off - sb.chunk_table_off;
        f.seek(SeekFrom::Start(sb.chunk_table_off))?;
        let mut tbytes = vec![0u8; table_ct_len as usize];
        f.read_exact(&mut tbytes)?;
        let raw_table = if let Some(ref key) = resolved_key {
            let nonce = derive_nonce(&salt, Region::ChunkTable, 0);
            crate::crypto::aead::open_whole(key, &nonce, b"chunktab", &tbytes)?
        } else {
            tbytes
        };
        let table = read_table_from_slice(&raw_table, sb.chunk_count)?;

        // Bounds check
        for (i, ce) in table.iter().enumerate() {
            if ce.data_off < sb.data_off
                || ce.data_off.saturating_add(ce.c_size) > file_end_for_data
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("chunk[{i}] out of bounds"),
                )
                .into());
            }
        }

        let aead = resolved_key.map(|k| (k, salt));

        Ok(Self {
            f: Arc::new(f),
            sb,
            manifest,
            table,
            aead,
            file_end_for_data,
        })
    }

    pub fn list_entries(&self) -> impl Iterator<Item = FileEntry> + '_ {
        self.manifest.files.iter().map(|fe| FileEntry {
            path: fe.path.clone(),
            u_size: fe.u_size,
            chunks: fe.chunk_refs.iter().map(|r| r.id as u32).collect(),
        })
    }

    pub fn chunk_map_for(&self, path: &str) -> Result<Vec<ChunkView>> {
        let fe = self
            .manifest
            .files
            .iter()
            .find(|x| x.path == path)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("no such file: {path}"),
                )
            })?;
        let mut acc = 0u64;
        let mut out = Vec::with_capacity(fe.chunk_refs.len());
        for (ord, cref) in fe.chunk_refs.iter().enumerate() {
            let ce = &self.table[cref.id as usize];
            let end = acc + ce.u_size;
            let pct_end = (end as f64 / fe.u_size.max(1) as f64) as f32;
            out.push(ChunkView {
                ordinal: ord as u64,
                id: cref.id,
                codec: ce.codec,
                file_off: acc,
                u_len: ce.u_size,
                c_len: ce.c_size,
                data_off: ce.data_off,
                pct_end,
            });
            acc = end;
        }
        Ok(out)
    }

    /// Read raw (possibly encrypted) chunk bytes at the given offset using a
    /// lock-free positional read — safe for concurrent callers on the same file.
    pub fn read_chunk_bytes(&self, data_off: u64, c_size: u64) -> std::io::Result<Vec<u8>> {
        let mut buf = vec![0u8; c_size as usize];
        read_exact_at(&self.f, &mut buf, data_off)?;
        Ok(buf)
    }

    pub fn open_reader(&self, path: &str) -> Result<crate::read::stream::FileReader<'_>> {
        crate::read::stream::FileReader::new(self, path)
    }

    pub fn open_range(
        &self,
        path: &str,
        start: u64,
        len: u64,
    ) -> Result<crate::read::stream::RangeReader<'_>> {
        crate::read::stream::RangeReader::new(self, path, start, len)
    }
}
