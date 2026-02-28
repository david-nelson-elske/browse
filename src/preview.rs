use std::fs;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

// ANSI escape helpers
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const ITALIC: &str = "\x1b[3m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const GREEN: &str = "\x1b[32m";
const MAGENTA: &str = "\x1b[35m";
const BLUE: &str = "\x1b[34m";

const MAX_PREVIEW_BYTES: u64 = 512 * 1024; // 512 KB
const MAX_PREVIEW_LINES: usize = 1000;

pub enum PreviewContent {
    Text(String),
    Directory(String),
    Binary(String),
    Empty,
    Error(String),
}

pub struct Previewer {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Previewer {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn preview(&self, file_path: &Path) -> (PreviewContent, usize) {
        let metadata = match fs::metadata(file_path) {
            Ok(m) => m,
            Err(e) => return (PreviewContent::Error(format!("Error: {}", e)), 1),
        };

        if metadata.is_dir() {
            return self.preview_directory(file_path);
        }

        if is_likely_binary(file_path) {
            let ext = file_path
                .extension()
                .map(|e| e.to_string_lossy().to_uppercase())
                .unwrap_or_else(|| "unknown".to_string());
            let size = format_size(metadata.len());
            return (
                PreviewContent::Binary(format!("Binary file ({})\nSize: {}", ext, size)),
                2,
            );
        }

        if metadata.len() == 0 {
            return (PreviewContent::Empty, 1);
        }

        if metadata.len() > MAX_PREVIEW_BYTES {
            let size = format_size(metadata.len());
            return (
                PreviewContent::Text(format!("File too large to preview ({})", size)),
                1,
            );
        }

        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => return (PreviewContent::Error(format!("Error: {}", e)), 1),
        };

        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        let truncated: String = if lines.len() > MAX_PREVIEW_LINES {
            lines[..MAX_PREVIEW_LINES].join("\n")
        } else {
            content.clone()
        };

        // Check if it's markdown
        let ext = file_path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        if ext == "md" || ext == "mdx" {
            let rendered = self.render_markdown(&truncated);
            return (PreviewContent::Text(rendered), total_lines);
        }

        // Try syntax highlighting
        let syntax = self
            .syntax_set
            .find_syntax_for_file(file_path)
            .ok()
            .flatten()
            .or_else(|| self.syntax_set.find_syntax_by_extension(&ext));

        if let Some(syntax) = syntax {
            let theme = &self.theme_set.themes["base16-ocean.dark"];
            let mut highlighter = HighlightLines::new(syntax, theme);
            let mut highlighted = String::new();

            for line in LinesWithEndings::from(&truncated) {
                if let Ok(ranges) = highlighter.highlight_line(line, &self.syntax_set) {
                    let escaped = as_24_bit_terminal_escaped(&ranges, false);
                    highlighted.push_str(&escaped);
                }
            }
            highlighted.push_str("\x1b[0m"); // reset
            return (PreviewContent::Text(highlighted), total_lines);
        }

        (PreviewContent::Text(truncated), total_lines)
    }

    /// Render markdown with syntax-highlighted code blocks
    fn render_markdown(&self, input: &str) -> String {
        let mut output = String::new();
        let mut lines = input.lines().peekable();
        let theme = &self.theme_set.themes["base16-ocean.dark"];

        while let Some(line) = lines.next() {
            // Fenced code block
            if line.starts_with("```") {
                let lang = line.trim_start_matches('`').trim();
                let mut code_lines: Vec<String> = Vec::new();

                // Collect lines until closing fence
                while let Some(code_line) = lines.next() {
                    if code_line.starts_with("```") {
                        break;
                    }
                    code_lines.push(code_line.to_string());
                }

                let code = code_lines.join("\n");

                // Try syntax highlighting
                let syntax = if !lang.is_empty() {
                    self.syntax_set
                        .find_syntax_by_token(lang)
                        .or_else(|| self.syntax_set.find_syntax_by_extension(lang))
                } else {
                    None
                };

                if let Some(syntax) = syntax {
                    let mut highlighter = HighlightLines::new(syntax, theme);
                    let code_with_newlines = code + "\n";
                    for code_line in LinesWithEndings::from(&code_with_newlines) {
                        if let Ok(ranges) =
                            highlighter.highlight_line(code_line, &self.syntax_set)
                        {
                            let escaped = as_24_bit_terminal_escaped(&ranges, false);
                            output.push_str(&format!("  {}", escaped));
                        }
                    }
                    output.push_str(RESET);
                } else {
                    // No syntax found — render as dimmed code
                    for code_line in &code_lines {
                        output.push_str(&format!("  {DIM}{}{RESET}\n", code_line));
                    }
                }
                output.push('\n');
                continue;
            }

            // Headings
            if line.starts_with("# ") {
                output.push_str(&format!(
                    "{BOLD}{CYAN}{}{RESET}\n",
                    &line[2..]
                ));
                continue;
            }
            if line.starts_with("## ") {
                output.push_str(&format!(
                    "{BOLD}{GREEN}{}{RESET}\n",
                    &line[3..]
                ));
                continue;
            }
            if line.starts_with("### ") {
                output.push_str(&format!(
                    "{BOLD}{YELLOW}{}{RESET}\n",
                    &line[4..]
                ));
                continue;
            }
            if line.starts_with("#### ") || line.starts_with("##### ") || line.starts_with("###### ") {
                let text = line.trim_start_matches('#').trim_start();
                output.push_str(&format!("{BOLD}{MAGENTA}{text}{RESET}\n"));
                continue;
            }

            // Horizontal rule
            if (line.starts_with("---") || line.starts_with("***") || line.starts_with("___"))
                && line.chars().all(|c| c == '-' || c == '*' || c == '_' || c == ' ')
                && line.len() >= 3
            {
                output.push_str(&format!("{DIM}────────────────────────────────{RESET}\n"));
                continue;
            }

            // Bullet lists (with optional leading whitespace for nesting)
            if let Some((indent_level, text)) = strip_bullet_prefix(line) {
                let indent = "  ".repeat(indent_level + 1);
                let rendered = render_inline(text);
                output.push_str(&format!("{indent}{BLUE}•{RESET} {rendered}\n"));
                continue;
            }

            // Numbered lists (with optional leading whitespace)
            if let Some((indent_level, text)) = strip_numbered_prefix(line) {
                let indent = "  ".repeat(indent_level + 1);
                let rendered = render_inline(text);
                output.push_str(&format!("{indent}{rendered}\n"));
                continue;
            }

            // Blockquotes
            if line.starts_with("> ") {
                let text = &line[2..];
                let rendered = render_inline(text);
                output.push_str(&format!("  {DIM}│{RESET} {rendered}\n"));
                continue;
            }

            // Regular text with inline formatting
            let rendered = render_inline(line);
            output.push_str(&rendered);
            output.push('\n');
        }

        output
    }

    fn preview_directory(&self, dir_path: &Path) -> (PreviewContent, usize) {
        match fs::read_dir(dir_path) {
            Ok(entries) => {
                let entries: Vec<_> = entries.flatten().collect();
                let total = entries.len();
                let dirs = entries
                    .iter()
                    .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                    .count();
                let files = total - dirs;
                (
                    PreviewContent::Directory(format!(
                        "Directory: {} items ({} dirs, {} files)",
                        total, dirs, files
                    )),
                    1,
                )
            }
            Err(e) => (PreviewContent::Error(format!("Error: {}", e)), 1),
        }
    }

    #[allow(dead_code)]
    pub fn file_info(&self, file_path: &Path) -> String {
        let metadata = match fs::metadata(file_path) {
            Ok(m) => m,
            Err(_) => return String::new(),
        };

        let mut parts = Vec::new();
        if let Some(ext) = file_path.extension() {
            parts.push(ext.to_string_lossy().to_uppercase().to_string());
        }
        parts.push(format_size(metadata.len()));
        if metadata.file_type().is_symlink() {
            parts.push("symlink".to_string());
        }
        parts.join(" | ")
    }
}

/// Render inline markdown formatting: **bold**, *italic*, `code`, [links](url)
fn render_inline(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Inline code: `text`
        if chars[i] == '`' {
            if let Some(end) = find_closing(&chars, i + 1, '`') {
                let code_text: String = chars[i + 1..end].iter().collect();
                result.push_str(&format!("{DIM}{CYAN}{code_text}{RESET}"));
                i = end + 1;
                continue;
            }
        }

        // Bold: **text**
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_double_closing(&chars, i + 2, '*') {
                let inner: String = chars[i + 2..end].iter().collect();
                result.push_str(&format!("{BOLD}{inner}{RESET}"));
                i = end + 2;
                continue;
            }
        }

        // Italic: *text*
        if chars[i] == '*' && (i + 1 < len && chars[i + 1] != '*') {
            if let Some(end) = find_closing(&chars, i + 1, '*') {
                let inner: String = chars[i + 1..end].iter().collect();
                result.push_str(&format!("{ITALIC}{inner}{RESET}"));
                i = end + 1;
                continue;
            }
        }

        // Link: [text](url) — show text in blue
        if chars[i] == '[' {
            if let Some(bracket_end) = find_closing(&chars, i + 1, ']') {
                if bracket_end + 1 < len && chars[bracket_end + 1] == '(' {
                    if let Some(paren_end) = find_closing(&chars, bracket_end + 2, ')') {
                        let link_text: String = chars[i + 1..bracket_end].iter().collect();
                        result.push_str(&format!("{BLUE}{link_text}{RESET}"));
                        i = paren_end + 1;
                        continue;
                    }
                }
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

fn find_closing(chars: &[char], start: usize, target: char) -> Option<usize> {
    for i in start..chars.len() {
        if chars[i] == target {
            return Some(i);
        }
    }
    None
}

fn find_double_closing(chars: &[char], start: usize, target: char) -> Option<usize> {
    for i in start..chars.len().saturating_sub(1) {
        if chars[i] == target && chars[i + 1] == target {
            return Some(i);
        }
    }
    None
}

/// Strip bullet prefix (- or *) with optional leading whitespace, returns (indent_level, text)
fn strip_bullet_prefix(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let leading_spaces = line.len() - trimmed.len();
    let indent_level = leading_spaces / 2;

    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        Some((indent_level, &trimmed[2..]))
    } else {
        None
    }
}

/// Strip numbered prefix with optional leading whitespace, returns (indent_level, text)
fn strip_numbered_prefix(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let leading_spaces = line.len() - trimmed.len();
    let indent_level = leading_spaces / 2;

    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i > 0 && i < bytes.len() && (bytes[i] == b'.' || bytes[i] == b')') {
        let rest = &trimmed[i + 1..];
        if rest.starts_with(' ') {
            return Some((indent_level, rest.trim_start()));
        }
    }
    None
}

fn is_likely_binary(file_path: &Path) -> bool {
    const BINARY_EXTS: &[&str] = &[
        "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "avif", "mp3", "mp4", "wav", "avi",
        "mkv", "mov", "flac", "ogg", "zip", "gz", "tar", "bz2", "xz", "7z", "rar", "pdf", "doc",
        "docx", "xls", "xlsx", "ppt", "pptx", "exe", "dll", "so", "dylib", "bin", "o", "a",
        "woff", "woff2", "ttf", "otf", "eot", "sqlite", "db",
    ];
    file_path
        .extension()
        .map(|e| {
            let ext = e.to_string_lossy().to_lowercase();
            BINARY_EXTS.contains(&ext.as_str())
        })
        .unwrap_or(false)
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
