# fern

A small, fast terminal browser for folders and git changes, inspired by [yazi](https://github.com/sxyazi/yazi) and [lazygit](https://github.com/jesseduffield/lazygit).

Browse any folder with a syntax highlighted preview next to the list. Inside a git repo a second pane lists the changed files, and the preview shows each one as the full file with the changes marked, or as a plain diff. The view follows the files on disk, so it updates as they change.

## Install

macOS and Linux:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/pnilssson/fern-tui/releases/latest/download/fern-installer.sh | sh
```

Windows:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/pnilssson/fern-tui/releases/latest/download/fern-installer.ps1 | iex"
```

Update with `fern-update`. fern checks GitHub for a newer release once a day and shows a hint in the footer.

## Usage

Run `fern` to browse the current folder, or `fern PATH` to start somewhere else. Press `?` to see every key.

- `1` Files: move with the arrow keys. Left goes up a folder, right goes into a folder or opens a file in the preview.
- `2` Changes: the changed files of the current repo. Enter reveals a file in Files, and `f` switches between the full file and only the diff.

Copying paths and selected text needs `pbcopy` on macOS, `clip` on Windows, or `wl-copy`, `xclip` or `xsel` on Linux.

## Configuration

| Variable | Effect |
| --- | --- |
| `FERN_ROUNDED=1` | Rounds the selection. Needs a font with Powerline symbols, such as a Nerd Font. |
| `FERN_NO_UPDATE_CHECK=1` | Turns off the daily update check. |

## Development

Install the toolchain from `rust-toolchain.toml`, then:

```sh
just run [PATH]
just check
```

`just check` needs `just`, `cargo-deny`, `cargo-shear`, `cargo-nextest`, `typos` and `zizmor`.

Every push to `main` that passes CI is released with the next patch version.

## License

Licensed under the [MIT license](LICENSE-MIT).
