use std::path::Path;

use crate::app::ArchiveEntry;

/// Read all entries from a zip archive.
pub fn read_zip(path: &Path) -> Result<Vec<ArchiveEntry>, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("CANNOT OPEN: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("INVALID ARCHIVE: {}", e))?;

    let mut entries = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)
            .map_err(|e| format!("READ ERROR: {}", e))?;

        let name = entry.name().to_string();
        // Skip root-only entries and macOS metadata
        if name.is_empty() || name.starts_with("__MACOSX") {
            continue;
        }

        let is_dir = entry.is_dir();
        let size = entry.size();

        entries.push(ArchiveEntry {
            name: name.rsplit('/').find(|s| !s.is_empty())
                .unwrap_or(&name).to_string(),
            full_path: name,
            is_dir,
            size,
        });
    }

    Ok(entries)
}

/// Check if a file extension indicates a supported archive format.
pub fn is_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_lowercase().as_str(), "zip"))
        .unwrap_or(false)
}
