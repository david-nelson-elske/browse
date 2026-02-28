use std::collections::HashSet;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::preview::{PreviewContent, Previewer};
use crate::tree::{self, TreeNode, VisibleRow};

pub struct App {
    pub root_path: PathBuf,
    pub tree: Vec<TreeNode>,
    pub expanded: HashSet<PathBuf>,
    pub visible_rows: Vec<VisibleRow>,
    pub selected_index: usize,
    pub show_hidden: bool,
    pub preview_scroll: usize,
    pub preview_cache: (PreviewContent, usize),
    pub should_quit: bool,
    previewer: Previewer,
    last_preview_path: Option<PathBuf>,
}

impl App {
    pub fn new(root_path: PathBuf) -> Self {
        let tree = tree::build_tree(&root_path);
        let expanded = HashSet::new();
        let previewer = Previewer::new();

        let mut app = App {
            root_path,
            tree,
            expanded,
            visible_rows: Vec::new(),
            selected_index: 0,
            show_hidden: false,
            preview_scroll: 0,
            preview_cache: (PreviewContent::Empty, 0),
            should_quit: false,
            previewer,
            last_preview_path: None,
        };
        app.refresh();
        app
    }

    pub fn display_root(&self) -> String {
        let home = dirs::home_dir().unwrap_or_default();
        let root = self.root_path.to_string_lossy();
        let home_str = home.to_string_lossy();
        if root.as_ref() == home_str.as_ref() {
            "~".to_string()
        } else if let Ok(relative) = self.root_path.strip_prefix(&home) {
            format!("~/{}", relative.display())
        } else {
            root.to_string()
        }
    }

    /// Rebuild tree from disk and flatten, then update preview
    pub fn refresh(&mut self) {
        self.tree = tree::build_tree(&self.root_path);
        self.visible_rows =
            tree::flatten_tree(&mut self.tree, &self.expanded, self.show_hidden);

        // Clamp selected index
        if self.visible_rows.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.visible_rows.len() {
            self.selected_index = self.visible_rows.len() - 1;
        }

        self.update_preview();
    }

    fn update_preview(&mut self) {
        let current_path = self
            .visible_rows
            .get(self.selected_index)
            .map(|r| r.path.clone());

        if current_path != self.last_preview_path {
            self.last_preview_path = current_path.clone();
            if let Some(path) = current_path {
                self.preview_cache = self.previewer.preview(&path);
            } else {
                self.preview_cache = (PreviewContent::Empty, 0);
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_index < self.visible_rows.len().saturating_sub(1) {
            self.selected_index += 1;
            self.preview_scroll = 0;
            self.update_preview();
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.preview_scroll = 0;
            self.update_preview();
        }
    }

    pub fn jump_top(&mut self) {
        self.selected_index = 0;
        self.preview_scroll = 0;
        self.update_preview();
    }

    pub fn jump_bottom(&mut self) {
        self.selected_index = self.visible_rows.len().saturating_sub(1);
        self.preview_scroll = 0;
        self.update_preview();
    }

    pub fn toggle_expand(&mut self) {
        let row = match self.visible_rows.get(self.selected_index) {
            Some(r) => r.clone(),
            None => return,
        };

        if !row.is_directory {
            return;
        }

        if self.expanded.contains(&row.path) {
            self.expanded.remove(&row.path);
        } else {
            self.expanded.insert(row.path.clone());
        }
        self.preview_scroll = 0;
        self.refresh();
    }

    pub fn collapse_or_parent(&mut self) {
        let row = match self.visible_rows.get(self.selected_index) {
            Some(r) => r.clone(),
            None => return,
        };

        // If on an expanded dir, collapse it
        if row.is_expanded {
            self.expanded.remove(&row.path);
            self.preview_scroll = 0;
            self.refresh();
            return;
        }

        // Otherwise, move to parent
        let parent_idx = tree::find_parent_row(&self.visible_rows, self.selected_index);
        if parent_idx != self.selected_index {
            self.selected_index = parent_idx;
            self.preview_scroll = 0;
            self.update_preview();
        }
    }

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.selected_index = 0;
        self.preview_scroll = 0;
        self.refresh();
    }

    pub fn yank_path(&self) {
        let path = match self.visible_rows.get(self.selected_index) {
            Some(r) => r.path.to_string_lossy().to_string(),
            None => return,
        };

        // Try wl-copy (Wayland), then xclip (X11), then pbcopy (macOS)
        let clipboard_cmds: &[(&str, &[&str])] = &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("pbcopy", &[]),
        ];

        for (cmd, args) in clipboard_cmds {
            if let Ok(mut child) = Command::new(cmd)
                .args(*args)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(ref mut stdin) = child.stdin {
                    use std::io::Write;
                    let _ = stdin.write_all(path.as_bytes());
                }
                let _ = child.wait();
                return;
            }
        }
    }

    pub fn scroll_preview_down(&mut self, amount: usize) {
        self.preview_scroll += amount;
    }

    pub fn scroll_preview_up(&mut self, amount: usize) {
        self.preview_scroll = self.preview_scroll.saturating_sub(amount);
    }

    pub fn click_tree(&mut self, y: u16, area_height: u16) {
        let list_height = area_height.saturating_sub(1) as usize;
        let entries_height = list_height.saturating_sub(2); // minus header + separator

        // Calculate scroll offset (same as draw_tree)
        let mut scroll_offset: usize = 0;
        if self.visible_rows.len() > list_height {
            scroll_offset = self.selected_index.saturating_sub(list_height / 2);
            scroll_offset = scroll_offset.min(self.visible_rows.len().saturating_sub(list_height));
        }

        // y=0 is header, y=1 is separator, y>=2 is entries
        if y < 2 {
            return;
        }
        let entry_idx = (y as usize) - 2;
        if entry_idx >= entries_height {
            return;
        }

        let row_idx = scroll_offset + entry_idx;
        if row_idx >= self.visible_rows.len() {
            return;
        }

        self.selected_index = row_idx;
        self.preview_scroll = 0;
        self.update_preview();

        if self.visible_rows[row_idx].is_directory {
            self.toggle_expand();
        }
    }
}
