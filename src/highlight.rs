use std::path::Path;

use ratatui::style::Style;
use ratatui::text::Span;

use crate::palette::Palette;

/// Language detection by extension.
#[derive(Clone, Copy, PartialEq)]
enum Lang {
    Rust,
    Python,
    JavaScript,
    Go,
    C,
    Shell,
    Toml,
    Yaml,
    Json,
    Css,
    Html,
    Sql,
    Markdown,
    Unknown,
}

fn detect_lang(path: &Path) -> Lang {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());
    match ext.as_deref() {
        Some("rs") => Lang::Rust,
        Some("py") => Lang::Python,
        Some("js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx") => Lang::JavaScript,
        Some("go") => Lang::Go,
        Some("c" | "cpp" | "cc" | "cxx" | "h" | "hpp") => Lang::C,
        Some("sh" | "bash" | "zsh" | "fish") => Lang::Shell,
        Some("toml") => Lang::Toml,
        Some("yaml" | "yml") => Lang::Yaml,
        Some("json") => Lang::Json,
        Some("css" | "scss" | "sass" | "less") => Lang::Css,
        Some("html" | "htm" | "xml") => Lang::Html,
        Some("sql") => Lang::Sql,
        Some("md" | "mdx") => Lang::Markdown,
        _ => Lang::Unknown,
    }
}

fn keywords_for(lang: Lang) -> &'static [&'static str] {
    match lang {
        Lang::Rust => &[
            "fn", "let", "mut", "pub", "use", "mod", "struct", "enum", "impl", "trait",
            "where", "for", "while", "loop", "if", "else", "match", "return", "self",
            "Self", "crate", "super", "const", "static", "type", "as", "in", "ref",
            "move", "async", "await", "unsafe", "extern", "dyn", "macro_rules",
        ],
        Lang::Python => &[
            "def", "class", "import", "from", "if", "elif", "else", "for", "while",
            "return", "yield", "with", "as", "try", "except", "finally", "raise",
            "lambda", "pass", "break", "continue", "and", "or", "not", "in", "is",
            "None", "True", "False", "self", "async", "await",
        ],
        Lang::JavaScript => &[
            "function", "const", "let", "var", "if", "else", "for", "while", "do",
            "return", "class", "extends", "new", "this", "import", "export", "from",
            "default", "switch", "case", "break", "continue", "try", "catch", "finally",
            "throw", "async", "await", "yield", "typeof", "instanceof", "of", "in",
            "null", "undefined", "true", "false",
        ],
        Lang::Go => &[
            "func", "package", "import", "type", "struct", "interface", "map",
            "chan", "go", "defer", "if", "else", "for", "range", "switch", "case",
            "default", "return", "break", "continue", "select", "var", "const",
            "nil", "true", "false",
        ],
        Lang::C => &[
            "int", "char", "float", "double", "void", "long", "short", "unsigned",
            "signed", "struct", "union", "enum", "typedef", "if", "else", "for",
            "while", "do", "switch", "case", "default", "return", "break", "continue",
            "sizeof", "static", "extern", "const", "volatile", "register", "auto",
            "#include", "#define", "#ifdef", "#ifndef", "#endif", "#pragma",
            "NULL", "true", "false",
        ],
        Lang::Shell => &[
            "if", "then", "else", "elif", "fi", "for", "while", "do", "done",
            "case", "esac", "function", "return", "exit", "echo", "read",
            "export", "local", "set", "unset", "shift", "source",
        ],
        Lang::Sql => &[
            "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET",
            "DELETE", "CREATE", "TABLE", "DROP", "ALTER", "INDEX", "JOIN", "LEFT",
            "RIGHT", "INNER", "OUTER", "ON", "AND", "OR", "NOT", "NULL", "AS",
            "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "OFFSET", "UNION", "DISTINCT",
            "select", "from", "where", "insert", "into", "values", "update", "set",
            "delete", "create", "table", "drop", "alter", "join", "left", "right",
            "inner", "outer", "on", "and", "or", "not", "null", "as", "order", "by",
            "group", "having", "limit", "offset", "union", "distinct",
        ],
        _ => &[],
    }
}

fn comment_prefix(lang: Lang) -> &'static [&'static str] {
    match lang {
        Lang::Rust | Lang::JavaScript | Lang::Go | Lang::C | Lang::Css => &["//"],
        Lang::Python | Lang::Shell | Lang::Toml | Lang::Yaml => &["#"],
        Lang::Sql => &["--"],
        Lang::Html => &["<!--"],
        _ => &[],
    }
}

/// Highlight a single line of source code, producing ratatui Spans.
pub fn highlight_line<'a>(line: &str, path: &Path, pal: &Palette) -> Vec<Span<'a>> {
    let lang = detect_lang(path);

    // Colors
    let keyword_color = pal.text_hot;
    let string_color = pal.text_mid;
    let comment_color = pal.border_hot;
    let number_color = pal.text_mid;
    let default_color = pal.text_dim;
    let bg = pal.bg;

    // Check for full-line comment
    let trimmed = line.trim_start();
    for prefix in comment_prefix(lang) {
        if trimmed.starts_with(prefix) {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(comment_color).bg(bg),
            )];
        }
    }

    // Markdown: highlight headings and emphasis
    if lang == Lang::Markdown {
        if trimmed.starts_with('#') {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(keyword_color).bg(bg),
            )];
        }
        if trimmed.starts_with("```") || trimmed.starts_with("---") || trimmed.starts_with("***") {
            return vec![Span::styled(
                line.to_string(),
                Style::default().fg(comment_color).bg(bg),
            )];
        }
    }

    // JSON/YAML/TOML: highlight keys
    if matches!(lang, Lang::Json | Lang::Yaml | Lang::Toml) {
        // Simple key detection: text before `:` or `=`
        if let Some(sep_pos) = line.find(|c| c == ':' || c == '=') {
            let key_part = &line[..sep_pos];
            let rest = &line[sep_pos..];
            // Check if the key looks like a string key (has quotes or is just a word)
            if !key_part.trim().is_empty() {
                return vec![
                    Span::styled(key_part.to_string(), Style::default().fg(keyword_color).bg(bg)),
                    Span::styled(rest.to_string(), Style::default().fg(string_color).bg(bg)),
                ];
            }
        }
    }

    // Token-based highlighting for code languages
    let keywords = keywords_for(lang);
    if keywords.is_empty() && lang == Lang::Unknown {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(default_color).bg(bg),
        )];
    }

    let mut spans: Vec<Span<'a>> = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        // String literals
        if ch == '"' || ch == '\'' || ch == '`' {
            let quote = ch;
            let start = i;
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' {
                    i += 1; // skip escaped char
                }
                i += 1;
            }
            if i < len {
                i += 1; // closing quote
            }
            let s: String = chars[start..i].iter().collect();
            spans.push(Span::styled(s, Style::default().fg(string_color).bg(bg)));
            continue;
        }

        // Inline comment
        if ch == '/' && i + 1 < len && chars[i + 1] == '/' {
            let s: String = chars[i..].iter().collect();
            spans.push(Span::styled(s, Style::default().fg(comment_color).bg(bg)));
            break;
        }
        if ch == '#' && matches!(lang, Lang::Python | Lang::Shell | Lang::Toml | Lang::Yaml) {
            let s: String = chars[i..].iter().collect();
            spans.push(Span::styled(s, Style::default().fg(comment_color).bg(bg)));
            break;
        }
        if ch == '-' && i + 1 < len && chars[i + 1] == '-' && lang == Lang::Sql {
            let s: String = chars[i..].iter().collect();
            spans.push(Span::styled(s, Style::default().fg(comment_color).bg(bg)));
            break;
        }

        // Numbers
        if ch.is_ascii_digit() && (i == 0 || !chars[i - 1].is_alphanumeric()) {
            let start = i;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == 'x'
                || chars[i] == 'b' || chars[i] == 'o' || chars[i] == '_'
                || (chars[i].is_ascii_hexdigit() && start + 1 < len))
            {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            spans.push(Span::styled(s, Style::default().fg(number_color).bg(bg)));
            continue;
        }

        // Word (potential keyword)
        if ch.is_alphanumeric() || ch == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if keywords.contains(&word.as_str()) {
                spans.push(Span::styled(word, Style::default().fg(keyword_color).bg(bg)));
            } else {
                spans.push(Span::styled(word, Style::default().fg(default_color).bg(bg)));
            }
            continue;
        }

        // Preprocessor directives (C)
        if ch == '#' && lang == Lang::C {
            let start = i;
            while i < len && !chars[i].is_whitespace() {
                i += 1;
            }
            let directive: String = chars[start..i].iter().collect();
            if keywords.contains(&directive.as_str()) {
                spans.push(Span::styled(directive, Style::default().fg(keyword_color).bg(bg)));
            } else {
                spans.push(Span::styled(directive, Style::default().fg(default_color).bg(bg)));
            }
            continue;
        }

        // Punctuation and whitespace
        spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(default_color).bg(bg),
        ));
        i += 1;
    }

    if spans.is_empty() {
        spans.push(Span::styled(
            line.to_string(),
            Style::default().fg(default_color).bg(bg),
        ));
    }

    spans
}
