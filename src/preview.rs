use std::fs;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

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
            let rendered = termimad::text(&truncated);
            return (PreviewContent::Text(rendered.to_string()), total_lines);
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
