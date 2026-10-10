//! Writing our own files (settings, Personal Bests, caches).

use std::io::Write;
use std::path::Path;

/// Write via a temporary file and a rename, so a crash never leaves half a file.
pub fn write_atomically(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    let mut file = std::fs::File::create(&tmp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, path)
}
