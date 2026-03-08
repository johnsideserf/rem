/// Weyland-Yutani corporate logo and branding assets.
///
/// Dotmatrix-style WY combined mark. The `@` characters form the solid
/// geometry; `.` characters are the negative-space gaps. Rendered with
/// dual-color styling: `@` in text_hot, `.` in border_dim for authentic
/// dot-matrix display effect.

use crate::throbber::PaletteVariant;

/// Combined W-Y mark — 11 lines (1 border + 9 content + 1 border), 80 chars wide.
/// Symmetric dotmatrix art derived from reference file.
const LOGO: &[&str] = &[
    "@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@",
    "@@@@@...........@@@@...........@@@@..........@@@@...........@@@@...........@@@@@",
    "@@@@@@@...........@@@@.......@@@@..............@@@@.......@@@@...........@@@@@@@",
    "@@@@@@@@@...........@@@@...@@@@..................@@@@...@@@@...........@@@@@@@@@",
    "@@@@@@@@@@@...........@@@@@@........................@@@@@@...........@@@@@@@@@@@",
    "@@@@@@@@@@@@@...........@@.............@@.............@@...........@@@@@@@@@@@@@",
    "@@@@@@@@@@@@@@.......................@@..@@.......................@@@@@@@@@@@@@@",
    "@@@@@@@@@@@@@@@@@..................@@......@@..................@@@@@@@@@@@@@@@@@",
    "@@@@@@@@@@@@@@@@@@..............@@@..........@@@..............@@@@@@@@@@@@@@@@@@",
    "@@@@@@@@@@@@@@@@@@@@@..........@@@@..........@@@@..........@@@@@@@@@@@@@@@@@@@@@",
    "@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@",
];

/// Get the logo lines. Same art for all palettes; color differentiation
/// happens at render time via the palette.
pub fn logo_for(_variant: PaletteVariant) -> &'static [&'static str] {
    LOGO
}

/// Corporate name — letter-spaced.
pub const CORP_NAME: &str = "W E Y L A N D \u{2500} Y U T A N I";

/// Corporate tagline.
pub const CORP_TAG: &str = "BUILDING BETTER WORLDS";

/// Compact header badge — corporate name.
pub const HEADER_BADGE: &str = "\u{25c6} WEYLAND-YUTANI CORP";
