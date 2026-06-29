# Changelog

All notable changes to this project will be documented in this file.

## 0.0.5 - 2026-06-29

### Added

- Added focused diff mode for showing changed lines with configurable context and expandable hidden sections.
- Added Tree-sitter based code folding support.
- Added fold gutter indicators with configurable Unicode or ASCII symbols.
- Added mouse support for toggling folds from the gutter.
- Added public editor APIs for toggling folds, enabling/disabling folding, and configuring fold indicators.
- Added fold query files for C, C++, CSS, Go, HTML, JavaScript, JSON, Markdown, Python, Rust, shell, TOML, and YAML.
- Added a `fold_editor` example for trying code folding interactively.
- Added regression tests for diff focus behavior, fold detection, fold toggling, hidden fold navigation, mouse gutter behavior, and terminal input behavior.

### Changed

- Improved Unicode grapheme cursor movement and edit handling.
- Updated rendering and cursor movement to account for folded code rows.
- Updated diff focus click handling to account for the fold gutter width.
- Removed the old `editor_nl` source file after moving examples into workspace crates.

## 0.0.4 - 2026-05-13

### Added

- Added workspace example crates and GitHub CI workflow.
- Added support for hiding line numbers.
- Added support for configuring left code padding.

### Changed

- Moved examples into a Cargo workspace layout.
- Moved the core widget dependency from `ratatui` to `ratatui-core`.
- Made `crossterm` optional behind the `crossterm` feature.
- Relaxed dependency version constraints.
- Updated README documentation for workspace examples and feature flags.
- Moved `editor_nl` into a workspace example crate.

### Fixed

- Fixed workspace setup issues.
- Fixed right-aligned line number rendering.
- Adapted line-number and padding changes to the workspace structure.

## 0.0.3 - 2026-02-28

### Changed

- Updated Tree-sitter libraries to the 0.26 generation and related dependency versions.

## 0.0.2 - 2026-02-28

### Added

- Added custom highlight support during editor initialization.

### Changed

- Updated project dependencies.
- Updated README examples to show `Editor::new` returning a `Result`.

### Fixed

- Improved Python syntax highlighting.

## 0.0.1 - 2025-10-16

### Added

- Published the initial crates.io package.
- Added the editor widget with Tree-sitter syntax highlighting.
- Added built-in themes and Markdown highlighting.
- Added language support for multiple Tree-sitter grammars.
- Added embedded-language highlighting through Tree-sitter injections.
- Added editor actions for insertion, deletion, indentation, comments, duplicate line, delete line, undo, and redo.
- Added selection support, selection snapping, double-click and triple-click behavior.
- Added mouse click, drag, scroll, and auto-scroll handling.
- Added clipboard support for copy, cut, and paste.
- Added smart paste and indentation-aware editing behavior.
- Added marks support and APIs for marks, content slices, cursor access, offsets, selection, and focus.
- Added terminal cursor rendering support.
- Added LSP example and change notification callback support.

### Changed

- Split rendering into a separate `render.rs` module.
- Improved syntax highlighting performance with caching.
- Improved Unicode and grapheme handling.
- Improved cursor movement, line length calculation, and wide-line horizontal scrolling.
- Improved edit history and batch editing internals.
- Updated README documentation and examples.

### Fixed

- Fixed undo and redo state handling.
- Fixed delete behavior for empty selections and indentation cases.
- Fixed cursor up/down movement.
- Fixed scrolling at the end of text when horizontally scrolled.
- Fixed mouse click handling issues.
- Fixed tests after internal editor changes.
