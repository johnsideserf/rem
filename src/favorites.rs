use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
struct FavoritesFile {
    #[serde(default)]
    directories: Vec<String>,
}

pub fn favorites_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("rem").join("favorites.toml"))
}

pub fn load_favorites() -> Vec<PathBuf> {
    let Some(path) = favorites_path() else { return Vec::new() };
    let Ok(content) = std::fs::read_to_string(&path) else { return Vec::new() };
    let Ok(file) = toml::from_str::<FavoritesFile>(&content) else { return Vec::new() };
    file.directories.into_iter().map(PathBuf::from).collect()
}

pub fn save_favorites(favs: &[PathBuf]) {
    let Some(path) = favorites_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = FavoritesFile {
        directories: favs.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
    };
    if let Ok(content) = toml::to_string_pretty(&file) {
        let _ = std::fs::write(&path, content);
    }
}
