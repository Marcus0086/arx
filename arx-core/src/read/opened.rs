use crate::container::chunktab::{ChunkEntry, ENTRY_SIZE, read_table_from_slice};
use crate::container::manifest::Manifest;
use crate::container::superblock::{FLAG_ENCRYPTED, HEADER_LEN, Superblock};
use crate::container::tail::{TAIL_LEN, TAIL_MAGIC};
use crate::crypto::aead::{AeadKey, Region, derive_nonce};
use crate::error::Result;
use std::sync::Mutex;
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
    pub id: u64, // index into table
    pub codec: u8,
    pub file_off: u64,
    pub u_len: u64,
    pub c_len: u64,
    pub data_off: u64,
    pub pct_end: f32,
}

pub struct Opened {
    pub f: Arc<Mutex<File>>,
    pub sb: Superblock,
    pub manifest: Manifest,
    pub table: Vec<ChunkEntry>,
    pub aead: Option<(AeadKey, [u8; 32])>,
    pub file_end_for_data: u64,
}

impl Opened {
    pub fn open(path: &Path, aead_key: Option<[u8; 32]>, key_salt: [u8; 32]) -> Result<Self> {
        let mut f = File::open(path)?;
        let file_len = f.metadata()?.len();

        // superblock
        let sb = Superblock::read_from(&mut f)?;
        let enc_enabled = (sb.flags & FLAG_ENCRYPTED) != 0;

        // tail (optional)
        let mut file_end_for_data = file_len;
        if file_len >= TAIL_LEN {
            f.seek(SeekFrom::End(-(TAIL_LEN as i64)))?;
            let mut magic = [0u8; 8];
            if f.read_exact(&mut magic).is_ok() && magic == TAIL_MAGIC {
                file_end_for_data = file_len - TAIL_LEN;
            }
        }

        // manifest
        f.seek(SeekFrom::Start(HEADER_LEN))?;
        let mut mbytes = vec![0u8; sb.manifest_len as usize];
        f.read_exact(&mut mbytes)?;
        let manifest_bytes = if enc_enabled {
            let key = aead_key.ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "encrypted; --key/--key-salt required",
                )
            })?;
            let nonce = derive_nonce(&key_salt, Region::Manifest, 0);
            crate::crypto::aead::open_whole(&AeadKey(key), &nonce, b"manifest", &mbytes)
        } else {
            mbytes
        };
        let manifest: Manifest = ciborium::de::from_reader(&manifest_bytes[..])
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        // chunk table
        let table_ct_len = sb.data_off - sb.chunk_table_off;
        f.seek(SeekFrom::Start(sb.chunk_table_off))?;
        let mut tbytes = vec![0u8; table_ct_len as usize];
        f.read_exact(&mut tbytes)?;
        let raw_table = if enc_enabled {
            let key = aead_key.unwrap();
            let nonce = derive_nonce(&key_salt, Region::ChunkTable, 0);
            crate::crypto::aead::open_whole(&AeadKey(key), &nonce, b"chunktab", &tbytes)
        } else {
            tbytes
        };
        let expected_pt_len = sb.chunk_count as usize * ENTRY_SIZE;
        if raw_table.len() != expected_pt_len {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                format!(
                    "chunk table size mismatch: got {} expected {}",
                    raw_table.len(),
                    expected_pt_len
                ),
            )
            .into());
        }
        let table = read_table_from_slice(&mut &raw_table[..], sb.chunk_count)?;

        // bounds
        for (i, ce) in table.iter().enumerate() {
            if ce.data_off < sb.data_off
                || ce.data_off.saturating_add(ce.c_size) > file_end_for_data
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("chunk[{}] out of bounds", i),
                )
                .into());
            }
        }

        Ok(Self {
            f: Arc::new(Mutex::new(f)),
            sb,
            manifest,
            table,
            aead: if enc_enabled {
                Some((AeadKey(aead_key.unwrap()), key_salt))
            } else {
                None
            },
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
                    format!("no such file: {}", path),
                )
            })?;
        let mut acc = 0u64;
        let mut out = Vec::with_capacity(fe.chunk_refs.len());
        for (ord, cref) in fe.chunk_refs.iter().enumerate() {
            let ce = &self.table[cref.id as usize];
            let end = acc + ce.u_size as u64;
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

    // Readers implemented in read/stream.rs:
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
