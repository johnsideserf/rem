use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
struct MarksFile {
    marks: HashMap<String, String>,
}

fn marks_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("rem").join("marks.toml"))
}

pub fn load_marks() -> HashMap<char, PathBuf> {
    let Some(path) = marks_path() else {
        return HashMap::new();
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    let Ok(file) = toml::from_str::<MarksFile>(&content) else {
        return HashMap::new();
    };
    file.marks
        .into_iter()
        .filter_map(|(k, v)| {
            let c = k.chars().next()?;
            Some((c, PathBuf::from(v)))
        })
        .collect()
}

pub fn save_marks(marks: &HashMap<char, PathBuf>) {
    let Some(path) = marks_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = MarksFile {
        marks: marks
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string_lossy().into_owned()))
            .collect(),
    };
    if let Ok(content) = toml::to_string_pretty(&file) {
        let _ = std::fs::write(&path, content);
    }
}
