# fern

A small, fast terminal browser for a git repository. Move through folders like yazi, and jump to the changed files like lazygit.

- `1` Files: the repo tree. Arrow keys move in, out, up and down.
- `2` Changes: the changed files, shown as the full file with changes marked. `f` switches to only the diff.
- Right on a file reads it in the preview. Up and down scroll, Left goes back.

Built with Rust and ratatui. Code is syntax highlighted with the terminal's own colors, so it fits whatever theme it runs in.

Set `FERN_ROUNDED=1` to round the selection. It needs a font with the Powerline symbols, such as a Nerd Font.

## Development

Install the toolchain from `rust-toolchain.toml`, then:

```sh
just run [PATH]
just check
```

`just check` needs `just`, `cargo-deny`, `cargo-shear`, `cargo-nextest`, `typos` and `zizmor`.

## License

Licensed under the [MIT license](LICENSE-MIT).
