use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub children: Option<Vec<TreeNode>>, // None = not yet loaded
}

#[derive(Clone)]
pub struct VisibleRow {
    #[allow(dead_code)]
    pub node_idx: Vec<usize>, // path of indices into the tree (for identification)
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub depth: usize,
    pub is_expanded: bool,
}

/// Read one level of a directory, returning TreeNodes with children = None
pub fn build_tree(dir_path: &Path) -> Vec<TreeNode> {
    let entries = match fs::read_dir(dir_path) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut nodes: Vec<TreeNode> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();

        let metadata = entry.metadata();
        let is_symlink = entry
            .file_type()
            .map(|ft| ft.is_symlink())
            .unwrap_or(false);

        let is_directory = if is_symlink {
            // Follow symlink to determine if it points to a directory
            fs::metadata(&path)
                .map(|m| m.is_dir())
                .unwrap_or(false)
        } else {
            metadata.map(|m| m.is_dir()).unwrap_or(false)
        };

        nodes.push(TreeNode {
            name,
            path,
            is_directory,
            is_symlink,
            children: None,
        });
    }

    // Sort: directories first, then case-insensitive alphabetical
    nodes.sort_by(|a, b| {
        if a.is_directory != b.is_directory {
            return if a.is_directory {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });

    nodes
}

/// Load children for a node in place
pub fn load_children(node: &mut TreeNode) {
    if !node.is_directory {
        return;
    }
    node.children = Some(build_tree(&node.path));
}

const MAX_TREE_DEPTH: usize = 50;

/// Recursively flatten expanded tree into visible rows
pub fn flatten_tree(
    nodes: &mut Vec<TreeNode>,
    expanded: &HashSet<PathBuf>,
    show_hidden: bool,
) -> Vec<VisibleRow> {
    let mut rows = Vec::new();
    flatten_recursive(nodes, expanded, show_hidden, 0, &mut Vec::new(), &mut rows);
    rows
}

fn flatten_recursive(
    nodes: &mut Vec<TreeNode>,
    expanded: &HashSet<PathBuf>,
    show_hidden: bool,
    depth: usize,
    idx_path: &mut Vec<usize>,
    rows: &mut Vec<VisibleRow>,
) {
    if depth > MAX_TREE_DEPTH {
        return;
    }

    for i in 0..nodes.len() {
        let node = &nodes[i];
        if !show_hidden && node.name.starts_with('.') {
            continue;
        }

        let is_expanded = node.is_directory && expanded.contains(&node.path);

        idx_path.push(i);
        rows.push(VisibleRow {
            node_idx: idx_path.clone(),
            name: node.name.clone(),
            path: node.path.clone(),
            is_directory: node.is_directory,
            is_symlink: node.is_symlink,
            depth,
            is_expanded,
        });

        if is_expanded {
            // Always reload children from disk to reflect filesystem changes
            load_children(&mut nodes[i]);
            if let Some(ref mut children) = nodes[i].children {
                flatten_recursive(children, expanded, show_hidden, depth + 1, idx_path, rows);
            }
        }

        idx_path.pop();
    }
}

/// Walk backward from index to find the nearest row with depth < current
pub fn find_parent_row(rows: &[VisibleRow], index: usize) -> usize {
    let current_depth = rows.get(index).map(|r| r.depth).unwrap_or(0);
    if current_depth == 0 {
        return index;
    }
    for i in (0..index).rev() {
        if rows[i].depth < current_depth {
            return i;
        }
    }
    index
}
