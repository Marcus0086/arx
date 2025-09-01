#![forbid(unsafe_code)]

pub mod error;

use std::path::Path;

pub fn pack(inputs: &[&Path], out: &Path) -> Result<(), error::ArxError> {
    println!("Packing {:?} into {:?}", inputs, out);
    Ok(())
}

pub fn list(archive: &Path) -> Result<(), error::ArxError> {
    println!("Listing {:?}", archive);
    Ok(())
}

pub fn extract(archive: &Path, dest: &Path) -> Result<(), error::ArxError> {
    println!("Extracting {:?} to {:?}", archive, dest);
    Ok(())
}
