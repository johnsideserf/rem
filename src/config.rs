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
    #[serde(default)]
    ticker: TickerConfig,
    #[serde(default)]
    screensaver: ScreensaverConfig,
    #[serde(default)]
    comms: CommsConfig,
}

#[derive(Deserialize, Default)]
struct CommsConfig {
    channel: Option<String>,
    #[serde(default)]
    feeds: Vec<CommsFeedEntry>,
    messages: Option<Vec<String>>,
    refresh_interval: Option<u32>,
    display_time: Option<u8>,
}

#[derive(Deserialize)]
struct CommsFeedEntry {
    name: String,
    url: String,
}

#[derive(Deserialize, Default)]
struct ScreensaverConfig {
    enabled: Option<bool>,
    timeout: Option<u64>,
    distress_timeout: Option<u64>,
}

#[derive(Deserialize, Default)]
struct TickerConfig {
    enabled: Option<bool>,
    messages: Option<Vec<String>>,
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
    glitch_enabled: Option<bool>,
}

pub struct Config {
    pub palette: Palette,
    pub symbols: SymbolSet,
    pub show_hidden: bool,
    pub default_panel: RightPanel,
    pub boot_sequence: bool,
    pub sort_mode: SortMode,
    pub reduce_motion: bool,
    pub glitch_enabled: bool,
    pub mouse_enabled: bool,
    pub warnings: Vec<String>,
    pub ticker_enabled: bool,
    pub ticker_messages: Vec<String>,
    pub screensaver_enabled: bool,
    pub screensaver_timeout: u64,
    pub distress_timeout: u64,
    pub comms_channel: crate::comms::Channel,
    pub comms_feeds: Vec<crate::comms::FeedConfig>,
    pub comms_custom_messages: Vec<String>,
    pub comms_refresh_interval: u32,
    pub comms_display_time: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            palette: Palette::phosphor_green(),
            symbols: SymbolSet::for_variant(SymbolVariant::Standard),
            show_hidden: false,
            default_panel: RightPanel::Info,
            boot_sequence: true,
            sort_mode: SortMode::default(),
            reduce_motion: false,
            glitch_enabled: true,
            mouse_enabled: true,
            warnings: Vec::new(),
            ticker_enabled: true,
            ticker_messages: Vec::new(), // empty = use app defaults
            screensaver_enabled: true,
            screensaver_timeout: 45,
            distress_timeout: 300,
            comms_channel: crate::comms::Channel::All,
            comms_feeds: Vec::new(),
            comms_custom_messages: Vec::new(),
            comms_refresh_interval: 30,
            comms_display_time: 8,
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

/// Save the glitch_enabled setting to config.toml, preserving other settings.
pub fn save_glitch(enabled: bool) {
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
        t.insert("glitch_enabled".to_string(), toml::Value::Boolean(enabled));
    }

    let _ = std::fs::write(&path, doc.to_string());
}

/// Save the comms channel to config.toml, preserving other settings.
pub fn save_comms_channel(channel: crate::comms::Channel) {
    let Some(path) = config_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut doc: toml::Table = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    let comms = doc.entry("comms")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if let toml::Value::Table(t) = comms {
        t.insert("channel".to_string(), toml::Value::String(channel.config_name().to_string()));
    }

    let _ = std::fs::write(&path, doc.to_string());
}

pub fn save_comms_display_time(secs: u8) {
    let Some(path) = config_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut doc: toml::Table = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    let comms = doc.entry("comms")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));
    if let toml::Value::Table(t) = comms {
        t.insert("display_time".to_string(), toml::Value::Integer(secs as i64));
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
                            "green" => Palette::phosphor_green(),
                            "amber" => Palette::amber(),
                            "cyan" => Palette::degraded_cyan(),
                            other => {
                                cfg.warnings.push(format!("unknown palette '{}', using green", other));
                                Palette::phosphor_green()
                            }
                        };
                    }
                    if let Some(v) = file.behavior.show_hidden {
                        cfg.show_hidden = v;
                    }
                    if let Some(p) = &file.behavior.default_panel {
                        cfg.default_panel = match p.as_str() {
                            "info" => RightPanel::Info,
                            "preview" => RightPanel::Preview,
                            "hidden" => RightPanel::Hidden,
                            other => {
                                cfg.warnings.push(format!("unknown panel '{}', using info", other));
                                RightPanel::Info
                            }
                        };
                    }
                    if let Some(v) = file.behavior.boot_sequence {
                        cfg.boot_sequence = v;
                    }
                    if let Some(v) = file.behavior.reduce_motion {
                        cfg.reduce_motion = v;
                    }
                    if let Some(v) = file.behavior.glitch_enabled {
                        cfg.glitch_enabled = v;
                    }
                    if let Some(s) = &file.behavior.sort_mode {
                        cfg.sort_mode = match s.as_str() {
                            "name_asc" => SortMode::NameAsc,
                            "name_desc" => SortMode::NameDesc,
                            "size_desc" => SortMode::SizeDesc,
                            "size_asc" => SortMode::SizeAsc,
                            "date_newest" => SortMode::DateNewest,
                            "date_oldest" => SortMode::DateOldest,
                            other => {
                                cfg.warnings.push(format!("unknown sort_mode '{}', using name_asc", other));
                                SortMode::NameAsc
                            }
                        };
                    }
                    // Ticker config (#88)
                    if let Some(v) = file.ticker.enabled {
                        cfg.ticker_enabled = v;
                    }
                    if let Some(msgs) = file.ticker.messages {
                        if !msgs.is_empty() {
                            cfg.ticker_messages = msgs;
                        }
                    }
                    // Screensaver config (#107)
                    if let Some(v) = file.screensaver.enabled {
                        cfg.screensaver_enabled = v;
                    }
                    if let Some(v) = file.screensaver.timeout {
                        cfg.screensaver_timeout = v;
                    }
                    if let Some(v) = file.screensaver.distress_timeout {
                        cfg.distress_timeout = v;
                    }
                    // Comms config (#105)
                    if let Some(ch) = &file.comms.channel {
                        cfg.comms_channel = crate::comms::Channel::from_config(ch);
                    }
                    if !file.comms.feeds.is_empty() {
                        cfg.comms_feeds = file.comms.feeds.iter().map(|f| {
                            crate::comms::FeedConfig { name: f.name.clone(), url: f.url.clone() }
                        }).collect();
                    }
                    if let Some(msgs) = file.comms.messages {
                        if !msgs.is_empty() {
                            cfg.comms_custom_messages = msgs;
                        }
                    }
                    if let Some(v) = file.comms.refresh_interval {
                        cfg.comms_refresh_interval = v;
                    }
                    if let Some(v) = file.comms.display_time {
                        cfg.comms_display_time = v.max(1);
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
                "--no-mouse" => cfg.mouse_enabled = false,
                _ => {}
            }
        }

        cfg
    }
}
