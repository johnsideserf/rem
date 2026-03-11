use std::path::Path;

#[derive(Clone, PartialEq)]
pub enum DiffKind {
    Same,
    Added,
    Removed,
}

#[derive(Clone)]
pub struct DiffLine {
    pub text: String,
    pub kind: DiffKind,
}

pub struct DiffView {
    pub left_path: String,
    pub right_path: String,
    pub left_lines: Vec<DiffLine>,
    pub right_lines: Vec<DiffLine>,
    pub scroll: usize,
    pub max_lines: usize,
}

impl DiffView {
    pub fn from_files(path1: &Path, path2: &Path) -> Result<Self, String> {
        let left_text = std::fs::read_to_string(path1)
            .map_err(|e| format!("CANNOT READ {}: {}", path1.display(), e))?;
        let right_text = std::fs::read_to_string(path2)
            .map_err(|e| format!("CANNOT READ {}: {}", path2.display(), e))?;

        let left_raw: Vec<&str> = left_text.lines().collect();
        let right_raw: Vec<&str> = right_text.lines().collect();

        // Simple LCS-based diff
        let (left_lines, right_lines) = compute_diff(&left_raw, &right_raw);
        let max_lines = left_lines.len().max(right_lines.len());

        Ok(Self {
            left_path: path1.file_name().map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path1.to_string_lossy().into_owned()),
            right_path: path2.file_name().map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path2.to_string_lossy().into_owned()),
            left_lines,
            right_lines,
            scroll: 0,
            max_lines,
        })
    }
}

fn compute_diff(left: &[&str], right: &[&str]) -> (Vec<DiffLine>, Vec<DiffLine>) {
    let m = left.len();
    let n = right.len();

    // LCS table
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if left[i - 1] == right[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to build diff
    let mut left_stack = Vec::new();
    let mut right_stack = Vec::new();
    let mut i = m;
    let mut j = n;

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && left[i - 1] == right[j - 1] {
            left_stack.push(DiffLine { text: left[i - 1].to_string(), kind: DiffKind::Same });
            right_stack.push(DiffLine { text: right[j - 1].to_string(), kind: DiffKind::Same });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            left_stack.push(DiffLine { text: String::new(), kind: DiffKind::Same });
            right_stack.push(DiffLine { text: right[j - 1].to_string(), kind: DiffKind::Added });
            j -= 1;
        } else if i > 0 {
            left_stack.push(DiffLine { text: left[i - 1].to_string(), kind: DiffKind::Removed });
            right_stack.push(DiffLine { text: String::new(), kind: DiffKind::Same });
            i -= 1;
        }
    }

    left_stack.reverse();
    right_stack.reverse();

    (left_stack, right_stack)
}
