use std::fs;
use std::path::Path;

const MAX_PREVIEW_SIZE: u64 = 1_048_576; // 1MB
const MAX_LINES: usize = 500;

pub enum PreviewContent {
    Text(Vec<String>),
    Binary,
    HexDump { lines: Vec<String>, size: u64 },
    TooLarge,
    Empty,
    Error(String),
    Image {
        width: u32,
        height: u32,
        format: String,
        braille: Vec<String>,
    },
}

const MAX_IMAGE_SIZE: u64 = 5_242_880; // 5MB

/// Check if a file extension is a supported image type.
pub fn is_image(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp")
    )
}

/// Load an image and convert to braille art.
pub fn load_image_preview(path: &Path, preview_width: usize, preview_height: usize) -> PreviewContent {
    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => return PreviewContent::Error(format!("{}", e)),
    };
    if meta.len() > MAX_IMAGE_SIZE {
        return PreviewContent::Error("IMAGE EXCEEDS 5MB THRESHOLD".to_string());
    }

    let img = match image::open(path) {
        Ok(i) => i,
        Err(e) => return PreviewContent::Error(format!("IMAGE DECODE FAILURE: {}", e)),
    };

    let orig_w = img.width();
    let orig_h = img.height();
    let format = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_uppercase())
        .unwrap_or_else(|| "IMG".to_string());

    // Each braille char represents 2x4 pixels
    let target_w = (preview_width * 2).max(4);
    let target_h = (preview_height * 4).max(8);

    let resized = img.resize(target_w as u32, target_h as u32, image::imageops::FilterType::Nearest);
    let gray = resized.to_luma8();
    let gw = gray.width() as usize;
    let gh = gray.height() as usize;

    // Convert to braille
    let mut braille_lines: Vec<String> = Vec::new();
    let mut y = 0;
    while y + 3 < gh {
        let mut line = String::new();
        let mut x = 0;
        while x + 1 < gw {
            // 2x4 block → braille character
            let threshold = 128u8;
            let mut code: u32 = 0x2800;
            // Braille dot mapping:
            // (0,0)=0x01 (1,0)=0x08
            // (0,1)=0x02 (1,1)=0x10
            // (0,2)=0x04 (1,2)=0x20
            // (0,3)=0x40 (1,3)=0x80
            let dots = [
                (0, 0, 0x01), (0, 1, 0x02), (0, 2, 0x04), (0, 3, 0x40),
                (1, 0, 0x08), (1, 1, 0x10), (1, 2, 0x20), (1, 3, 0x80),
            ];
            for &(dx, dy, bit) in &dots {
                let px = x + dx;
                let py = y + dy;
                if px < gw && py < gh && gray.get_pixel(px as u32, py as u32).0[0] < threshold {
                    code |= bit;
                }
            }
            if let Some(ch) = char::from_u32(code) {
                line.push(ch);
            }
            x += 2;
        }
        braille_lines.push(line);
        y += 4;
    }

    PreviewContent::Image {
        width: orig_w,
        height: orig_h,
        format,
        braille: braille_lines,
    }
}

pub fn load_preview(path: &Path) -> PreviewContent {
    if path.is_dir() {
        return PreviewContent::Error("DIRECTORY".to_string());
    }

    // Image preview (#40)
    if is_image(path) {
        return load_image_preview(path, 40, 20);
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
        // Generate hex dump of first 256 bytes
        let hex_bytes = &bytes[..bytes.len().min(256)];
        let mut hex_lines = Vec::new();
        for (offset, chunk) in hex_bytes.chunks(16).enumerate() {
            let addr = format!("{:08X}", offset * 16);
            let hex_part: String = chunk.iter()
                .enumerate()
                .map(|(i, b)| {
                    if i == 8 { format!("  {:02X}", b) } else { format!(" {:02X}", b) }
                })
                .collect();
            // Pad hex part to fixed width (49 chars: 16 bytes * 3 chars + 1 extra space at pos 8)
            let hex_padded = format!("{:<49}", hex_part);
            let ascii: String = chunk.iter()
                .map(|&b| if b >= 0x20 && b < 0x7F { b as char } else { '.' })
                .collect();
            hex_lines.push(format!("{}  {}  |{}|", addr, hex_padded, ascii));
        }
        return PreviewContent::HexDump { lines: hex_lines, size: meta.len() };
    }

    // Parse as UTF-8 (lossy)
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text.lines().take(MAX_LINES).map(|l| l.to_string()).collect();
    PreviewContent::Text(lines)
}
