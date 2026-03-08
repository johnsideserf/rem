use std::path::PathBuf;

use serde::Deserialize;

use crate::app::{RightPanel, SortMode};
use crate::palette::Palette;
use crate::symbols::{SymbolSet, SymbolVariant};
use crate::throbber::PaletteVariant;

#[derive(Deserialize, Default)]
struct ConfigFile {
    #[serde(default)]
    appearance: AppearanceConfig,
    #[serde(default)]
    behavior: BehaviorConfig,
}

#[derive(Deserialize, Default)]
struct AppearanceConfig {
    palette: Option<String>,
    symbols: Option<String>,
}

#[derive(Deserialize, Default)]
struct BehaviorConfig {
    show_hidden: Option<bool>,
    default_panel: Option<String>,
    boot_sequence: Option<bool>,
    sort_mode: Option<String>,
    reduce_motion: Option<bool>,
}

pub struct Config {
    pub palette: Palette,
    pub symbols: SymbolSet,
    pub show_hidden: bool,
    pub default_panel: RightPanel,
    pub boot_sequence: bool,
    pub sort_mode: SortMode,
    pub reduce_motion: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            palette: Palette::phosphor_green(),
            symbols: SymbolSet::for_variant(SymbolVariant::Standard),
            show_hidden: true,
            default_panel: RightPanel::Info,
            boot_sequence: true,
            sort_mode: SortMode::default(),
            reduce_motion: false,
        }
    }
}

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("rem").join("config.toml"))
}

/// Save the current theme variant to config.toml, preserving other settings.
pub fn save_theme(variant: PaletteVariant) {
    let Some(path) = config_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Read existing config or start fresh
    let mut doc: toml::Table = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    // Update appearance.palette
    let appearance = doc.entry("appearance")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if let toml::Value::Table(t) = appearance {
        let name = match variant {
            PaletteVariant::Green => "green",
            PaletteVariant::Amber => "amber",
            PaletteVariant::Cyan => "cyan",
        };
        t.insert("palette".to_string(), toml::Value::String(name.to_string()));
    }

    let _ = std::fs::write(&path, doc.to_string());
}

/// Save the current symbol set to config.toml, preserving other settings.
pub fn save_symbols(variant: SymbolVariant) {
    let Some(path) = config_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut doc: toml::Table = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    let appearance = doc.entry("appearance")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if let toml::Value::Table(t) = appearance {
        t.insert("symbols".to_string(), toml::Value::String(variant.config_name().to_string()));
    }

    let _ = std::fs::write(&path, doc.to_string());
}

/// Save the current sort mode to config.toml, preserving other settings.
pub fn save_sort_mode(mode: SortMode) {
    let Some(path) = config_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut doc: toml::Table = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    let behavior = doc.entry("behavior")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if let toml::Value::Table(t) = behavior {
        let name = match mode {
            SortMode::NameAsc => "name_asc",
            SortMode::NameDesc => "name_desc",
            SortMode::SizeDesc => "size_desc",
            SortMode::SizeAsc => "size_asc",
            SortMode::DateNewest => "date_newest",
            SortMode::DateOldest => "date_oldest",
        };
        t.insert("sort_mode".to_string(), toml::Value::String(name.to_string()));
    }

    let _ = std::fs::write(&path, doc.to_string());
}

impl Config {
    /// Load config from file, then apply CLI overrides.
    pub fn load(args: &[String]) -> Self {
        let mut cfg = Self::default();

        // Load from file
        if let Some(path) = config_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(file) = toml::from_str::<ConfigFile>(&content) {
                    if let Some(s) = &file.appearance.symbols {
                        cfg.symbols = SymbolSet::for_variant(SymbolVariant::from_config(s));
                    }
                    if let Some(p) = &file.appearance.palette {
                        cfg.palette = match p.as_str() {
                            "amber" => Palette::amber(),
                            "cyan" => Palette::degraded_cyan(),
                            _ => Palette::phosphor_green(),
                        };
                    }
                    if let Some(v) = file.behavior.show_hidden {
                        cfg.show_hidden = v;
                    }
                    if let Some(p) = &file.behavior.default_panel {
                        cfg.default_panel = match p.as_str() {
                            "preview" => RightPanel::Preview,
                            "hidden" => RightPanel::Hidden,
                            _ => RightPanel::Info,
                        };
                    }
                    if let Some(v) = file.behavior.boot_sequence {
                        cfg.boot_sequence = v;
                    }
                    if let Some(v) = file.behavior.reduce_motion {
                        cfg.reduce_motion = v;
                    }
                    if let Some(s) = &file.behavior.sort_mode {
                        cfg.sort_mode = match s.as_str() {
                            "name_desc" => SortMode::NameDesc,
                            "size_desc" => SortMode::SizeDesc,
                            "size_asc" => SortMode::SizeAsc,
                            "date_newest" => SortMode::DateNewest,
                            "date_oldest" => SortMode::DateOldest,
                            _ => SortMode::NameAsc,
                        };
                    }
                }
            }
        }

        // CLI flags override config
        for arg in args {
            match arg.as_str() {
                "--amber" => cfg.palette = Palette::amber(),
                "--cyan" => cfg.palette = Palette::degraded_cyan(),
                "--green" => cfg.palette = Palette::phosphor_green(),
                "--no-boot" => cfg.boot_sequence = false,
                _ => {}
            }
        }

        cfg
    }
}
