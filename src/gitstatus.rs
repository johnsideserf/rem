use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitFileStatus {
    Added,
    Modified,
    Untracked,
    Staged,
    Conflict,
}

pub fn parse_git_status(dir: &Path) -> HashMap<String, GitFileStatus> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain", "-uall"])
        .current_dir(dir)
        .output();
    let Ok(output) = output else { return HashMap::new() };
    if !output.status.success() { return HashMap::new(); }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut statuses = HashMap::new();

    for line in stdout.lines() {
        if line.len() < 4 { continue; }
        let index = line.as_bytes()[0];
        let worktree = line.as_bytes()[1];
        let filename = line[3..].trim().to_string();
        // Strip path to just filename
        let name = Path::new(&filename)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or(filename.clone());

        let status = match (index, worktree) {
            (b'U', _) | (_, b'U') | (b'A', b'A') | (b'D', b'D') => GitFileStatus::Conflict,
            (b'A', _) | (b'C', _) => GitFileStatus::Added,
            (b'M', b' ') | (b'R', _) | (b'D', _) => GitFileStatus::Staged,
            (_, b'M') | (b' ', b'D') => GitFileStatus::Modified,
            (b'?', b'?') => GitFileStatus::Untracked,
            _ => continue,
        };
        statuses.insert(name, status);
    }
    statuses
}

pub fn git_stage(dir: &Path, file: &str) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .args(["add", file])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("GIT ADD FAILED: {}", e))?;
    if output.status.success() { Ok(()) }
    else { Err(format!("GIT ADD: {}", String::from_utf8_lossy(&output.stderr))) }
}

pub fn git_unstage(dir: &Path, file: &str) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .args(["reset", "HEAD", file])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("GIT RESET FAILED: {}", e))?;
    if output.status.success() { Ok(()) }
    else { Err(format!("GIT RESET: {}", String::from_utf8_lossy(&output.stderr))) }
}

pub fn git_commit(dir: &Path, msg: &str) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("GIT COMMIT FAILED: {}", e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!("GIT COMMIT: {}", String::from_utf8_lossy(&output.stderr)))
    }
}
