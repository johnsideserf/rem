/// Throbber/spinner system with per-palette character sets.
///
/// Three throbber types at different tick rates:
/// - Data Stream (every 1 tick / 100ms) — I/O operations
/// - Processing  (every 2 ticks / 200ms) — compute-bound work
/// - Heartbeat   (every 3 ticks / 300ms) — persistent system status

/// Which kind of throbber animation to use.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThrobberKind {
    DataStream,
    Processing,
    Heartbeat,
}

impl ThrobberKind {
    fn tick_divisor(self) -> u32 {
        match self {
            Self::DataStream => 1,
            Self::Processing => 2,
            Self::Heartbeat => 3,
        }
    }
}

/// Which palette variant drives the frame set.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteVariant {
    Green,
    Amber,
    Cyan,
}

// ---------------------------------------------------------------------------
// Frame arrays — per palette, per kind
// ---------------------------------------------------------------------------

// Data Stream: I/O operations (directory scan, file copy)
const DS_GREEN: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const DS_AMBER: &[&str] = &["⠁", "⠈", "⠐", "⠠", "⢀", "⡀", "⠄", "⠂"]; // sparse — colony signal
const DS_CYAN:  &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"]; // dense — corporate

// Processing: compute-bound work (recursive size calc, search indexing)
const PR_GREEN: &[&str] = &["░", "▒", "▓", "█", "▓", "▒", "░"];
const PR_AMBER: &[&str] = &["╸", "╺", "╸", "╺", " ", "╸", " ", "╺", "╸"]; // gaps — colony degradation
const PR_CYAN:  &[&str] = &["◰", "◳", "◲", "◱"]; // clean — corporate precision

// Heartbeat: persistent system status in header bar
const HB_GREEN: &[&str] = &["·", "∙", "•", "●", "•", "∙", "·"];
const HB_AMBER: &[&str] = &["⡀", "⡀", "⣀", "⣠", "⣤", "⣶", "⣿", "⣶", "⣤", "⣠", "⣀", "⡀", " ", " ", "⡀"]; // gaps — colony dropout
const HB_CYAN:  &[&str] = &["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▂", "▁"]; // smooth — corporate

fn frames_for(kind: ThrobberKind, variant: PaletteVariant) -> &'static [&'static str] {
    match (kind, variant) {
        (ThrobberKind::DataStream,  PaletteVariant::Green) => DS_GREEN,
        (ThrobberKind::DataStream,  PaletteVariant::Amber) => DS_AMBER,
        (ThrobberKind::DataStream,  PaletteVariant::Cyan)  => DS_CYAN,
        (ThrobberKind::Processing,  PaletteVariant::Green) => PR_GREEN,
        (ThrobberKind::Processing,  PaletteVariant::Amber) => PR_AMBER,
        (ThrobberKind::Processing,  PaletteVariant::Cyan)  => PR_CYAN,
        (ThrobberKind::Heartbeat,   PaletteVariant::Green) => HB_GREEN,
        (ThrobberKind::Heartbeat,   PaletteVariant::Amber) => HB_AMBER,
        (ThrobberKind::Heartbeat,   PaletteVariant::Cyan)  => HB_CYAN,
    }
}

// ---------------------------------------------------------------------------
// Throbber struct
// ---------------------------------------------------------------------------

pub struct Throbber {
    frames: &'static [&'static str],
    current: usize,
    tick_divisor: u32,
    tick_count: u32,
}

impl Throbber {
    /// Create a new throbber for the given kind and palette variant.
    pub fn new(kind: ThrobberKind, variant: PaletteVariant) -> Self {
        Self {
            frames: frames_for(kind, variant),
            current: 0,
            tick_divisor: kind.tick_divisor(),
            tick_count: 0,
        }
    }

    /// Create a throbber with custom frame set from a symbol set.
    pub fn from_frames(frames: &'static [&'static str], kind: ThrobberKind) -> Self {
        Self {
            frames,
            current: 0,
            tick_divisor: kind.tick_divisor(),
            tick_count: 0,
        }
    }

    /// Advance one tick. The frame only changes when tick_count reaches tick_divisor.
    pub fn tick(&mut self) {
        self.tick_count += 1;
        if self.tick_count >= self.tick_divisor {
            self.tick_count = 0;
            self.current = (self.current + 1) % self.frames.len();
        }
    }

    /// Current frame character to render.
    pub fn frame(&self) -> &str {
        self.frames[self.current]
    }
}
