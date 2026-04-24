# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

- `cargo build` - build the debug binary.
- `cargo build --release` - build the release binary used for distribution.
- `cargo run` - run the TUI locally.
- `cargo test` - run all unit and integration tests.
- `cargo test manual_repo` - run the integration test target/filter for manual repository indexing.
- `cargo test <test_name>` - run a single test by name, for example `cargo test changing_entry_refreshes_loaded_content`.
- `cargo clippy` - run Rust lints.
- `cargo fmt` - format the Rust codebase.

## Architecture

This is a Rust 2021 terminal UI application for browsing a local SCUT survival manual repository. The binary entry point in `src/main.rs` only owns terminal lifecycle concerns: enabling raw mode, entering the alternate screen, installing a panic hook that restores the terminal, and calling the library app runner.

The core event loop and state machine live in `src/app.rs`. `App` switches between `PathPrompt`, `Home`, and `Manual` modes, handles keyboard events, and owns `ManualState`. `ManualState` tracks the current manual repository, selected section and entry, focus across the three panes, loaded Markdown document, scroll position, viewport size, and the dual-column content toggle.

Manual repository discovery is isolated in `src/manual.rs`. A valid manual root must contain a `docs/` directory. Top-level Markdown files become the synthetic `首页 / 顶层` section, each non-empty subdirectory becomes a section, hidden paths are ignored, nested Markdown files are collected with `walkdir`, and entry titles come from Markdown frontmatter, then the first H1, then the filename.

Markdown parsing and rendering helpers are in `src/content.rs`. `DocumentContent` is the internal plain-text block representation used by tests and wrapping logic, while `RichContentRenderCache` uses `tui-markdown` to produce styled `ratatui::text::Text` for the live UI. `LoadedDocument` in `src/app.rs` caches rich rendering by width so content is regenerated when the viewport changes.

Terminal drawing is concentrated in `src/ui.rs`. It renders the path prompt, home screen, and manual browser. The manual browser uses a fixed three-pane conceptual layout: sections, entries, and content. Very narrow terminals show a resize message; very wide terminals cap section and entry widths and give remaining space to content. UI tests cover banner selection and related layout helpers, while state and parser tests live beside their modules.

Text entry behavior is encapsulated in `src/input.rs`. `TextInput` handles Unicode-aware cursor movement, editing, and command history; app code delegates raw key events to it for home commands and path input.

Integration fixtures under `tests/fixtures/manual_repo` model the expected manual repository shape. Keep fixture changes synchronized with assertions in `tests/manual_repo.rs` and module-level tests in `src/manual.rs` / `src/app.rs`.
