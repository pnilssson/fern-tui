# Agent guidelines

Before changing Rust, read `crates/AGENTS.md`.

## Approach
- Build only what the current task needs, shaped so the next steps fit without rework; no speculative features, options or abstractions.
- Fern has no users yet: keep no backward compatibility, and remove obsolete code and stale paths in the same change.
- Check current documentation or source before using a library, tool or API.
- Add dependencies, tools and GitHub Actions at their latest stable version; pin actions to a full commit SHA with the version in a comment.
- Stay small and fast: add a dependency only when it clearly beats a few lines of code.

## Working rules
- Never launch the TUI for visual checks unless asked; the user runs it in their own terminal.
- Run `just check` before committing or pushing and fix what it finds. Add an `#[allow]`, lint override, ignore entry or lower threshold only after the user agrees.
- Allow permissive and file-level copyleft licenses (BSD, ISC, Zlib, MPL-2.0) in cargo-deny without asking; ask before GPL, LGPL, AGPL or other strong copyleft.
- Leave changes uncommitted until the user asks. Work on `main`, without branches or pull requests unless asked.
- These files hold only rules an agent would otherwise get wrong and no check enforces. When a rule changes, replace the old one; add a new one only when the user asks.

## Code
- Prefer concise names, one word where it reads well.
- Use braces for every control-flow body, put early returns on their own lines, and separate logical steps with blank lines.
- Write no comments; names explain the code. Keep only directives tools need, such as lint attributes and `// SAFETY:`.
- Keep `README.md` to an overview and local development instructions.
- Every change works on Windows, macOS and Linux; never assume a path separator, home folder or shell.
- Use the terminal's ANSI palette so fern follows the theme of the terminal it runs in; syntax colors come from the ANSI theme in two-face; the only fixed colors are the dark diff tints (indexed 22, 28, 52, 88).

## Testing
- Test by risk: integrations with git, and parsing data we don't own. Test behaviour through public interfaces with real captured data.
- Add a test that reproduces each bug before fixing it.
- Skip tests for presentation, glue and trivial code.
- Use neutral sample data (`app`, `src/main.rs`), never the project's own name or real people's names.
