use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Read file contents at a specific offset and length.
pub fn read_file_range(
    path: &Path,
    offset: u64,
    length: u32,
    max_file_size: u64,
) -> anyhow::Result<Vec<u8>> {
    let meta = fs::metadata(path)?;
    if meta.len() > max_file_size {
        anyhow::bail!("File exceeds maximum size limit");
    }

    let mut file = fs::File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; length as usize];
    let bytes_read = file.read(&mut buf)?;
    buf.truncate(bytes_read);
    Ok(buf)
}
