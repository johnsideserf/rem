/// Swappable glyph/symbol sets for the entire UI.
///
/// Each set defines all visual characters used across the interface:
/// indicators, progress bars, separators, file icons, and throbber frames.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SymbolVariant {
    Standard,
    Ascii,
    Block,
    Minimal,
    Pipeline,
    Braille,
    Scanline,
}

impl SymbolVariant {
    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => "STANDARD",
            Self::Ascii => "ASCII",
            Self::Block => "BLOCK",
            Self::Minimal => "MINIMAL",
            Self::Pipeline => "PIPELINE",
            Self::Braille => "BRAILLE",
            Self::Scanline => "SCANLINE",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Self::Standard => "Nerd Font glyphs",
            Self::Ascii => "Pure ASCII compat",
            Self::Block => "Heavy geometric",
            Self::Minimal => "Clean, sparse",
            Self::Pipeline => "Industrial pipes",
            Self::Braille => "Dot patterns",
            Self::Scanline => "CRT interlaced",
        }
    }

    pub fn config_name(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Ascii => "ascii",
            Self::Block => "block",
            Self::Minimal => "minimal",
            Self::Pipeline => "pipeline",
            Self::Braille => "braille",
            Self::Scanline => "scanline",
        }
    }

    pub fn from_config(s: &str) -> Self {
        match s {
            "ascii" => Self::Ascii,
            "block" => Self::Block,
            "minimal" => Self::Minimal,
            "pipeline" => Self::Pipeline,
            "braille" => Self::Braille,
            "scanline" => Self::Scanline,
            _ => Self::Standard,
        }
    }

    pub const ALL: &'static [SymbolVariant] = &[
        Self::Standard,
        Self::Ascii,
        Self::Block,
        Self::Minimal,
        Self::Pipeline,
        Self::Braille,
        Self::Scanline,
    ];
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct SymbolSet {
    pub variant: SymbolVariant,
    // Indicators
    pub cursor: &'static str,
    pub mark: &'static str,
    pub cut: &'static str,
    pub copy: &'static str,
    pub checkmark: &'static str,
    pub warning: &'static str,
    // Progress
    pub bar_fill: &'static str,
    pub bar_empty: &'static str,
    // Separators & structure
    pub separator: &'static str,
    pub scroll_thumb: &'static str,
    pub scroll_track: &'static str,
    pub text_cursor: &'static str,
    pub ellipsis: &'static str,
    pub em_dash: &'static str,
    pub arrow_right: &'static str,
    pub sort_asc: &'static str,
    pub sort_desc: &'static str,
    // Network
    pub tx_indicator: &'static str,
    pub rx_indicator: &'static str,
    // Telemetry border
    pub rule_left: &'static str,
    pub rule_fill: &'static str,
    pub rule_right: &'static str,
    // File icons (generic fallbacks for non-nerd-font sets)
    pub use_nerd_fonts: bool,
    pub dir_icon: &'static str,
    pub file_icon: &'static str,
    // Git
    pub git_dirty: &'static str,
    // Throbber / heartbeat frames
    pub throbber_frames: &'static [&'static str],
    pub heartbeat_frames: &'static [&'static str],
}

impl SymbolSet {
    pub fn for_variant(v: SymbolVariant) -> Self {
        match v {
            SymbolVariant::Standard => standard(),
            SymbolVariant::Ascii => ascii(),
            SymbolVariant::Block => block(),
            SymbolVariant::Minimal => minimal(),
            SymbolVariant::Pipeline => pipeline(),
            SymbolVariant::Braille => braille(),
            SymbolVariant::Scanline => scanline(),
        }
    }
}

pub fn standard() -> SymbolSet {
    SymbolSet {
        variant: SymbolVariant::Standard,
        cursor: "\u{25b6}",      // ▶
        mark: "\u{25c6}",        // ◆
        cut: "\u{2702}",         // ✂
        copy: "\u{2295}",        // ⊕
        checkmark: "\u{2713}",   // ✓
        warning: "\u{26a0}",     // ⚠
        bar_fill: "\u{2588}",    // █
        bar_empty: "\u{2591}",   // ░
        separator: "\u{00b7}",   // ·
        scroll_thumb: "\u{2588}", // █
        scroll_track: "\u{2502}", // │
        text_cursor: "\u{258b}", // ▋
        ellipsis: "\u{2026}",    // …
        em_dash: "\u{2014}",     // —
        arrow_right: "\u{2192}", // →
        sort_asc: "\u{2191}",    // ↑
        sort_desc: "\u{2193}",   // ↓
        tx_indicator: "\u{25b2}", // ▲
        rx_indicator: "\u{25bc}", // ▼
        rule_left: "\u{2576}\u{2500}",
        rule_fill: "\u{2500}",   // ─
        rule_right: "\u{2574}",
        use_nerd_fonts: true,
        dir_icon: "\u{f07b}",    //  (fallback)
        file_icon: "\u{f15b}",   //  (fallback)
        git_dirty: "\u{25c6}",   // ◆
        throbber_frames: &["\u{2801}", "\u{2809}", "\u{2819}", "\u{2818}", "\u{2838}", "\u{2834}", "\u{2826}", "\u{2827}", "\u{2807}", "\u{280f}"],
        heartbeat_frames: &["\u{00b7}", "\u{2219}", "\u{2022}", "\u{25cf}", "\u{2022}", "\u{2219}", "\u{00b7}"],
    }
}

pub fn ascii() -> SymbolSet {
    SymbolSet {
        variant: SymbolVariant::Ascii,
        cursor: ">",
        mark: "*",
        cut: "x",
        copy: "+",
        checkmark: "ok",
        warning: "!!",
        bar_fill: "#",
        bar_empty: "-",
        separator: "|",
        scroll_thumb: "#",
        scroll_track: "|",
        text_cursor: "_",
        ellipsis: "~",
        em_dash: "-",
        arrow_right: "->",
        sort_asc: "^",
        sort_desc: "v",
        tx_indicator: "TX",
        rx_indicator: "RX",
        rule_left: "+-",
        rule_fill: "-",
        rule_right: "-+",
        use_nerd_fonts: false,
        dir_icon: "[D]",
        file_icon: "[F]",
        git_dirty: "*",
        throbber_frames: &["|", "/", "-", "\\"],
        heartbeat_frames: &[".", "o", "O", "o", ".", " "],
    }
}

pub fn block() -> SymbolSet {
    SymbolSet {
        variant: SymbolVariant::Block,
        cursor: "\u{2588}\u{25b6}",  // █▶
        mark: "\u{2588}",            // █
        cut: "\u{2592}",             // ▒
        copy: "\u{2593}",            // ▓
        checkmark: "\u{2588}",       // █
        warning: "\u{2591}",         // ░
        bar_fill: "\u{2588}",        // █
        bar_empty: "\u{2592}",       // ▒
        separator: "\u{2588}",       // █
        scroll_thumb: "\u{2588}",    // █
        scroll_track: "\u{2591}",    // ░
        text_cursor: "\u{2588}",     // █
        ellipsis: "\u{2026}",        // …
        em_dash: "\u{2014}",         // —
        arrow_right: "\u{25b6}",     // ▶
        sort_asc: "\u{25b2}",        // ▲
        sort_desc: "\u{25bc}",       // ▼
        tx_indicator: "\u{25b2}",    // ▲
        rx_indicator: "\u{25bc}",    // ▼
        rule_left: "\u{2588}",
        rule_fill: "\u{2580}",       // ▀
        rule_right: "\u{2588}",
        use_nerd_fonts: false,
        dir_icon: "\u{25a0}",        // ■
        file_icon: "\u{25a1}",       // □
        git_dirty: "\u{2588}",       // █
        throbber_frames: &["\u{2591}", "\u{2592}", "\u{2593}", "\u{2588}", "\u{2593}", "\u{2592}"],
        heartbeat_frames: &["\u{2581}", "\u{2582}", "\u{2583}", "\u{2584}", "\u{2585}", "\u{2586}", "\u{2587}", "\u{2588}", "\u{2587}", "\u{2586}", "\u{2585}", "\u{2584}", "\u{2583}", "\u{2582}", "\u{2581}"],
    }
}

pub fn minimal() -> SymbolSet {
    SymbolSet {
        variant: SymbolVariant::Minimal,
        cursor: "\u{203a}",          // ›
        mark: "\u{2022}",            // •
        cut: "~",
        copy: "=",
        checkmark: "\u{2022}",       // •
        warning: "!",
        bar_fill: "\u{25cf}",        // ●
        bar_empty: "\u{00b7}",       // ·
        separator: " ",
        scroll_thumb: "\u{25cf}",    // ●
        scroll_track: "\u{00b7}",    // ·
        text_cursor: "\u{258f}",     // ▏
        ellipsis: "\u{2026}",        // …
        em_dash: "\u{2013}",         // –
        arrow_right: "\u{203a}",     // ›
        sort_asc: "\u{2191}",        // ↑
        sort_desc: "\u{2193}",       // ↓
        tx_indicator: "\u{2191}",    // ↑
        rx_indicator: "\u{2193}",    // ↓
        rule_left: "\u{00b7}",
        rule_fill: "\u{00b7}",       // ·
        rule_right: "\u{00b7}",
        use_nerd_fonts: false,
        dir_icon: "/",
        file_icon: " ",
        git_dirty: "\u{2022}",       // •
        throbber_frames: &["\u{00b7}", " ", "\u{2022}", " ", "\u{25cf}", " ", "\u{2022}", " ", "\u{00b7}"],
        heartbeat_frames: &["\u{00b7}", "\u{2022}", "\u{25cf}", "\u{2022}", "\u{00b7}", " "],
    }
}

pub fn pipeline() -> SymbolSet {
    SymbolSet {
        variant: SymbolVariant::Pipeline,
        cursor: "\u{25b7}",          // ▷
        mark: "\u{25c8}",            // ◈
        cut: "\u{2298}",             // ⊘
        copy: "\u{229e}",            // ⊞
        checkmark: "\u{25c6}",       // ◆
        warning: "\u{25c7}",         // ◇
        bar_fill: "\u{2588}",        // █
        bar_empty: "\u{2591}",       // ░
        separator: "\u{256b}",       // ╫
        scroll_thumb: "\u{2551}",    // ║
        scroll_track: "\u{2502}",    // │
        text_cursor: "\u{258b}",     // ▋
        ellipsis: "\u{2026}",        // …
        em_dash: "\u{2550}",         // ═
        arrow_right: "\u{25b7}",     // ▷
        sort_asc: "\u{25b3}",        // △
        sort_desc: "\u{25bd}",       // ▽
        tx_indicator: "\u{25b3}",    // △
        rx_indicator: "\u{25bd}",    // ▽
        rule_left: "\u{2554}\u{2550}",
        rule_fill: "\u{2550}",       // ═
        rule_right: "\u{2557}",
        use_nerd_fonts: false,
        dir_icon: "\u{2560}",        // ╠
        file_icon: "\u{255f}",       // ╟
        git_dirty: "\u{25c8}",       // ◈
        throbber_frames: &["\u{2574}", "\u{2578}", "\u{2576}", "\u{257a}"],
        heartbeat_frames: &["\u{25c6}", "\u{25c7}", "\u{25c6}", "\u{25c7}", "\u{25c6}", " "],
    }
}

pub fn braille() -> SymbolSet {
    SymbolSet {
        variant: SymbolVariant::Braille,
        cursor: "\u{28ff}",          // ⣿
        mark: "\u{2836}",            // ⠶
        cut: "\u{282d}",             // ⠭
        copy: "\u{283f}",            // ⠿
        checkmark: "\u{28ff}",       // ⣿
        warning: "\u{2821}",         // ⠡
        bar_fill: "\u{28ff}",        // ⣿
        bar_empty: "\u{2840}",       // ⡀
        separator: "\u{2810}",       // ⠐
        scroll_thumb: "\u{28ff}",    // ⣿
        scroll_track: "\u{2847}",    // ⡇
        text_cursor: "\u{28ff}",     // ⣿
        ellipsis: "\u{2026}",        // …
        em_dash: "\u{2500}",         // ─
        arrow_right: "\u{2836}",     // ⠶
        sort_asc: "\u{281b}",        // ⠛
        sort_desc: "\u{28e4}",       // ⣤
        tx_indicator: "\u{281b}",    // ⠛
        rx_indicator: "\u{28e4}",    // ⣤
        rule_left: "\u{28c0}\u{28c0}",
        rule_fill: "\u{28c0}",       // ⣀
        rule_right: "\u{28c0}",
        use_nerd_fonts: false,
        dir_icon: "\u{2847}",        // ⡇
        file_icon: "\u{2802}",       // ⠂
        git_dirty: "\u{2836}",       // ⠶
        throbber_frames: &["\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}", "\u{283c}", "\u{2834}", "\u{2826}", "\u{2827}", "\u{2807}", "\u{280f}"],
        heartbeat_frames: &["\u{283f}", "\u{2837}", "\u{2836}", "\u{2826}", "\u{2836}", "\u{2837}"],
    }
}

pub fn scanline() -> SymbolSet {
    SymbolSet {
        variant: SymbolVariant::Scanline,
        cursor: "\u{25b8}",          // ▸
        mark: "\u{25c9}",            // ◉
        cut: "\u{2326}",             // ⌦
        copy: "\u{2325}",            // ⌥
        checkmark: "\u{25c9}",       // ◉
        warning: "\u{25ce}",         // ◎
        bar_fill: "\u{25ae}",        // ▮
        bar_empty: "\u{25af}",       // ▯
        separator: "\u{22ee}",       // ⋮
        scroll_thumb: "\u{25ae}",    // ▮
        scroll_track: "\u{25af}",    // ▯
        text_cursor: "\u{258b}",     // ▋
        ellipsis: "\u{2026}",        // …
        em_dash: "\u{2500}",         // ─
        arrow_right: "\u{25b8}",     // ▸
        sort_asc: "\u{25b4}",        // ▴
        sort_desc: "\u{25be}",       // ▾
        tx_indicator: "\u{25b4}",    // ▴
        rx_indicator: "\u{25be}",    // ▾
        rule_left: "\u{254c}\u{254c}",
        rule_fill: "\u{254c}",       // ╌
        rule_right: "\u{254c}",
        use_nerd_fonts: false,
        dir_icon: "\u{25aa}",        // ▪
        file_icon: "\u{25ab}",       // ▫
        git_dirty: "\u{25c9}",       // ◉
        throbber_frames: &["\u{25ae}", "\u{25af}", "\u{25ae}", "\u{25af}"],
        heartbeat_frames: &["\u{25c9}", "\u{25ce}", "\u{25c9}", "\u{25ce}", "\u{25c9}", " "],
    }
}
