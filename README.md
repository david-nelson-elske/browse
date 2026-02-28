# browse

A fast terminal file browser with tree navigation and syntax-highlighted preview.

![browse](https://img.shields.io/badge/rust-stable-blue) ![License: MIT](https://img.shields.io/badge/license-MIT-green)

## Features

- IDE-style tree with expand/collapse (`▶`/`▼`)
- Syntax highlighting for 50+ languages (powered by syntect)
- Markdown rendering in the terminal
- Mouse support (click to select/expand, scroll to navigate preview)
- Vim-style keyboard navigation
- Lazy directory loading (fast on large trees)
- Single binary, no runtime dependencies
- ~2 MB, starts instantly

## Install

### From source

```bash
cargo install --path .
```

### Pre-built binaries

Check the [releases](https://github.com/davidjamesmoss/browse/releases) page.

## Usage

```bash
browse              # browse current directory
browse ~/projects   # browse a specific path
browse /etc         # browse anywhere
```

## Keyboard

| Key | Action |
|-----|--------|
| `j` / `k` / `↑` / `↓` | Navigate tree |
| `l` / `Enter` / `→` | Expand/collapse directory |
| `h` / `←` | Collapse directory or jump to parent |
| `g` / `G` | Jump to top/bottom |
| `.` | Toggle hidden files |
| `J` / `K` | Scroll preview line by line |
| `d` / `u` | Scroll preview half-page |
| `q` / `Ctrl-c` | Quit |

## Mouse

- **Click** a file to select it (preview updates)
- **Click** a directory to expand/collapse it
- **Scroll wheel** over the preview pane to scroll

## License

MIT
