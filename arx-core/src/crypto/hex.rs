use crate::error::Result;
use hex;

pub fn parse_hex_array<const N: usize>(hex_str: &str) -> Result<[u8; N]> {
    let mut out = [0u8; N];
    let bytes = hex::decode(hex_str.trim())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("invalid hex: {e}")))?;
    if bytes.len() != N {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("expected {N} bytes ({} hex chars), got {}", N, bytes.len()),
        )
        .into());
    }
    out.copy_from_slice(&bytes);
    Ok(out)
}
