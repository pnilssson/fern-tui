# Rust

- Use `thiserror` in library crates and `anyhow` only in binaries. Return errors instead of panicking.
- Keep `fern-git` free of terminal and UI code; it runs git through the CLI and parses its output.
- Run git with `GIT_OPTIONAL_LOCKS=0` so reading status never touches the index and re-triggers the watcher.
- Keep the UI thread free of blocking work; git and file reads run on worker threads and report back as messages.
- Declare dependencies once in the root `[workspace.dependencies]`; every crate inherits `[workspace.lints]`.
- Build paths with `Path` and `PathBuf`, and keep platform-specific code behind `#[cfg]` in one place per concern, with the same interface everywhere.
- Put unit tests inline in a `#[cfg(test)] mod tests` at the bottom of the file and integration tests in `tests/`, with shared helpers in `tests/common/mod.rs`.
