use std::fs;
use std::path::Path;

const MAX_PREVIEW_SIZE: u64 = 1_048_576; // 1MB
const MAX_LINES: usize = 500;

pub enum PreviewContent {
    Text(Vec<String>),
    Binary,
    TooLarge,
    Empty,
    Error(String),
}

pub fn load_preview(path: &Path) -> PreviewContent {
    if path.is_dir() {
        return PreviewContent::Error("DIRECTORY".to_string());
    }

    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return PreviewContent::Error(format!("{}", e)),
    };

    if meta.len() == 0 {
        return PreviewContent::Empty;
    }

    if meta.len() > MAX_PREVIEW_SIZE {
        return PreviewContent::TooLarge;
    }

    // Read raw bytes to detect binary
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => return PreviewContent::Error(format!("{}", e)),
    };

    // Check for binary content: NUL bytes in first 8KB
    let check_len = bytes.len().min(8192);
    if bytes[..check_len].contains(&0) {
        return PreviewContent::Binary;
    }

    // Parse as UTF-8 (lossy)
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text.lines().take(MAX_LINES).map(|l| l.to_string()).collect();
    PreviewContent::Text(lines)
}
