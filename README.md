# kittywrite

![](https://img.shields.io/badge/run%20tests-passing-green?style=flat&labelColor=black) ![](https://img.shields.io/badge/made%20with...-kittywrite%20and%20zed.-white?style=flat&labelColor=black)

lightweight, cat-themed code editor. built with rust, egui, syntect and lua.

```
=^.^=
```

## features

- syntax highlighting for 40+ languages via syntect (pure rust, no C deps apart from lua)
- tab bar -- open as many files as you want
- line number gutter with git diff indicators (green bars for changed lines)
- catppuccin themes (mocha, frappe, macchiato, latte) plus original kittywrite theme
- quick open (`ctrl+p`) -- fuzzy search recently opened files
- settings panel (`ctrl+,`) -- change theme, font, editor options
- lua-powered config -- edit `init.lua` next to the binary
- lua console at runtime (`tools > lua console`) for live tweaks
- native file dialogs (open / save / save-as)
- adjustable font size from the view menu
- cross-platform: windows, mac, linux -- one codebase

## building

you need:
- rust 1.70+ (`rustup update stable`)
- a C compiler (gcc / clang / msvc) -- needed to compile the vendored lua 5.4 source

```sh
# debug build -- compiles fast, runs slower
cargo build

# release build -- takes a few minutes, gives you a fast stripped binary
cargo build --release
```

the binary lands at `target/release/kittywrite` (linux/mac) or `target\release\kittywrite.exe` (windows).

copy `init.lua` from the repo root to sit next to the binary, or the editor falls back to its built-in defaults.

### linux notes

the file dialog crate (`rfd`) talks to `xdg-desktop-portal` over d-bus.
gnome, kde, xfce and most desktop environments ship this out of the box.

on a headless or very minimal system without d-bus, open/save dialogs will silently do nothing.
workaround: open the terminal alongside kittywrite and pipe filenames in, or add a cli arg handler to `main.rs` (`std::env::args().nth(1)`).

### windows notes

the `windows_subsystem = "windows"` attribute suppresses the console window
in release builds. in debug builds you still get it, which is handy for
seeing lua error output.

### mac notes

standard `cargo build --release` works. if you want a `.app` bundle,
look at `cargo-bundle` -- it wraps the binary for you.

## folders next to binary

kittywrite creates these folders next to the binary on first launch:

```
kittywrite.exe (or kittywrite on linux/mac)
  init.lua       user config
  plugins/       plugin folders (each has init.lua inside)
  themes/        custom theme json files
  recent.txt     recent files list (auto-generated)
  projects.txt   recent projects list (auto-generated)
```

## config (`init.lua`)

place this file next to the kittywrite binary. it's run on every startup.

| key                   | type    | default     | notes                                    |
|-----------------------|---------|-------------|------------------------------------------|
| `font_size`           | number  | 14          | pixels, clamped to 8..48                 |
| `tab_width`           | number  | 4           | spaces per tab stop                      |
| `auto_indent`         | boolean | true        | match prev line indent on enter          |
| `auto_pair`           | boolean | true        | automatically insert brackets/quotes     |
| `line_height`         | number  | 1.0         | line height multiplier (1.0 = normal)    |
| `word_wrap`           | boolean | false       | wrap at window edge                      |
| `show_line_numbers`   | boolean | true        | left gutter                              |
| `theme`               | string  | "kittywrite"| kittywrite, mocha, frappe, macchiato, latte |

example:

```lua
kittywrite.font_size = 16
kittywrite.word_wrap = true
kittywrite.show_line_numbers = false
kittywrite.theme = "mocha"
```

## keyboard shortcuts

| shortcut         | action          |
|------------------|-----------------|
| `ctrl+n`         | new tab         |
| `ctrl+o`         | open file       |
| `ctrl+s`         | save            |
| `ctrl+shift+s`   | save as         |
| `ctrl+w`         | close tab       |
| `ctrl+p`         | quick open (recent files) |
| `ctrl+,`         | settings        |
| `ctrl+z`         | undo            |
| `ctrl+y`         | redo            |
| `ctrl+x/c/v`     | cut / copy / paste |
| `ctrl+a`         | select all      |

undo/redo/cut/copy/paste/select-all are handled natively by the text widget.

## supported languages (syntax highlighting)

rust, python, javascript, typescript, jsx/tsx, c, c++, c#, go, java, kotlin,
swift, ruby, php, lua, bash/zsh/fish, powershell, html, css/scss, json, yaml,
toml, markdown, sql, xml, r, dart, elixir, haskell, ocaml, clojure, scala,
nim, zig, v, terraform/hcl, protobuf, graphql, viml, makefile, dockerfile

anything else falls back to plain text (no crash, just no colors).

## architecture

```
src/
  main.rs         entry point, window setup
  app.rs          main egui update loop, all ui panels, action dispatch
  editor.rs       EditorTab struct, language detection from file extension
  filetree.rs     file explorer sidebar with git status
  highlighter.rs  syntect -> egui LayoutJob conversion
  theme.rs        cat color palette, egui style setup, json theme loading
  lua_engine.rs   mlua vm, config script loading, runtime exec
  plugin.rs       plugin system, hooks, commands, editor api
init.lua          user config (place next to binary)
plugins/          user plugins (folder with init.lua in each subfolder)
themes/           user themes (json files)
```

## themes & plugins

see [GUIDE.md](GUIDE.md) for:
- creating custom themes (json format)
- creating plugins (lua scripts)
- full plugin api reference
- theme color reference
