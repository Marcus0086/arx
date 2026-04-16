use std::fs::File;
use std::io;

/// Read exactly `buf.len()` bytes from `file` at absolute `offset`,
/// without seeking or locking — safe for concurrent calls on the same `File`.
#[cfg(unix)]
pub fn read_exact_at(file: &File, buf: &mut [u8], mut offset: u64) -> io::Result<()> {
    use std::os::unix::fs::FileExt;
    let mut filled = 0;
    while filled < buf.len() {
        let n = file.read_at(&mut buf[filled..], offset)?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected EOF in read_exact_at",
            ));
        }
        filled += n;
        offset += n as u64;
    }
    Ok(())
}

#[cfg(windows)]
pub fn read_exact_at(file: &File, buf: &mut [u8], mut offset: u64) -> io::Result<()> {
    use std::os::windows::fs::FileExt;
    let mut filled = 0;
    while filled < buf.len() {
        let n = file.seek_read(&mut buf[filled..], offset)?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected EOF in read_exact_at",
            ));
        }
        filled += n;
        offset += n as u64;
    }
    Ok(())
}
