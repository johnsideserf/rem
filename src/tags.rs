use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
struct TagsFile {
    #[serde(default)]
    tags: HashMap<String, Vec<String>>,
}

pub struct TagStore {
    pub tags: HashMap<PathBuf, Vec<String>>,
}

impl TagStore {
    pub fn new() -> Self {
        Self { tags: HashMap::new() }
    }

    pub fn get(&self, path: &Path) -> Option<&Vec<String>> {
        self.tags.get(path)
    }

    pub fn add_tag(&mut self, path: PathBuf, tag: String) {
        let entry = self.tags.entry(path).or_insert_with(Vec::new);
        if !entry.contains(&tag) {
            entry.push(tag);
        }
    }

    pub fn remove_tag(&mut self, path: &Path, tag: &str) {
        if let Some(tags) = self.tags.get_mut(path) {
            tags.retain(|t| t != tag);
            if tags.is_empty() {
                self.tags.remove(path);
            }
        }
    }

    #[allow(dead_code)]
    pub fn has_tags(&self, path: &Path) -> bool {
        self.tags.get(path).map_or(false, |t| !t.is_empty())
    }
}

fn tags_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("rem").join("tags.toml"))
}

pub fn load_tags() -> TagStore {
    let Some(path) = tags_path() else { return TagStore::new() };
    let Ok(content) = std::fs::read_to_string(&path) else { return TagStore::new() };
    let Ok(file) = toml::from_str::<TagsFile>(&content) else { return TagStore::new() };
    let tags = file.tags.into_iter()
        .map(|(k, v)| (PathBuf::from(k), v))
        .collect();
    TagStore { tags }
}

pub fn save_tags(store: &TagStore) {
    let Some(path) = tags_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = TagsFile {
        tags: store.tags.iter()
            .map(|(k, v)| (k.to_string_lossy().into_owned(), v.clone()))
            .collect(),
    };
    if let Ok(content) = toml::to_string_pretty(&file) {
        let _ = std::fs::write(&path, content);
    }
}
