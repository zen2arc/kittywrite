# kittywrite theme & plugin guide

## table of contents

- [creating themes](#creating-themes)
- [creating plugins](#creating-plugins)
- [plugin api reference](#plugin-api-reference)

---

## creating themes

themes are json files placed in the `themes/` folder next to the kittywrite binary.
if the folder doesn't exist, it's created on first launch.

### quick start

1. create `themes/my-theme.json` next to kittywrite.exe
2. open kittywrite, go to plugins panel and click refresh (or restart)
3. select your theme in settings (`ctrl+,`)

### theme file format

```json
{
  "name": "my-theme",
  "author": "your-name",
  "is_light": false,
  "ui": {
    "bg_void": "#0d1117",
    "bg_panel": "#161b22",
    "bg_editor": "#0f0f18",
    "bg_tab_idle": "#21262d",
    "bg_tab_active": "#30363d",
    "fg_main": "#c9d1d9",
    "fg_dim": "#8b949e",
    "fg_gutter": "#30363d",
    "accent_paw": "#f85149",
    "accent_eye": "#e3b341",
    "accent_fur": "#3fb950",
    "selection_bg": "#388bfd"
  },
  "syntax": {
    "keyword": "#ff79c6",
    "function": "#8be9fd",
    "string": "#f1fa8c",
    "comment": "#6272a4",
    "number": "#bd93f9",
    "type": "#8be9fd",
    "variable": "#ffb86c",
    "operator": "#ff79c6",
    "punctuation": "#f8f8f2"
  }
}
```

### ui colors reference

| key | what it colors |
|-----|----------------|
| `bg_void` | deepest background behind everything |
| `bg_panel` | panels, menus, sidebar |
| `bg_editor` | main editor background |
| `bg_tab_idle` | inactive tab background |
| `bg_tab_active` | active tab background |
| `fg_main` | primary text color |
| `fg_dim` | secondary text, hints, placeholders |
| `fg_gutter` | line number area background |
| `accent_paw` | errors, delete markers, active widgets |
| `accent_eye` | highlights, search matches, tab titles |
| `accent_fur` | success, additions, links |
| `selection_bg` | text selection (use rgba for alpha) |

### syntax colors reference

| key | what it colors |
|-----|----------------|
| `keyword` | `fn`, `let`, `if`, `for`, etc. |
| `function` | function and method names |
| `string` | string literals |
| `comment` | comments |
| `number` | numeric literals |
| `type` | types, structs, classes |
| `variable` | variables, parameters |
| `operator` | `+`, `-`, `=`, `->`, etc. |
| `punctuation` | `(`, `)`, `{`, `}`, `;`, etc. |

### tips

- use `#rrggbb` format for colors
- `selection_bg` alpha is controlled by the editor (don't worry about it)
- look at existing themes in `src/theme.rs` for reference
- test by restarting kittywrite after editing the json

### installing themes via ui

1. get a `.json` theme file from someone
2. open plugins panel (`view > plugins`)
3. click "install theme"
4. select the `.json` file
5. restart or click refresh

---

## creating plugins

plugins are lua scripts placed in the `plugins/` folder next to the binary.
each plugin is a subfolder with an `init.lua` file inside.

### quick start

1. create folder structure: `plugins/my-plugin/init.lua`
2. add your lua code to `init.lua`
3. restart kittywrite
4. check plugins panel (`view > plugins`) to confirm it loaded

### plugin structure

```
plugins/
  word-count/
    init.lua
  my-cool-plugin/
    init.lua
    README.md     (optional)
    assets/       (optional)
```

### basic plugin

```lua
-- plugins/hello/init.lua

local kw = plugin({
    name = "hello",
    version = "1.0.0",
    description = "a simple hello plugin",
    author = "your-name",
})

-- run on startup
kw:on("startup", function()
    kittywrite.notify("hello plugin loaded!")
end)

-- register a command
kw:command("hello", "say hello", function()
    return "hello from plugin!"
end)
```

### hooks

hooks let you react to editor events.

| hook | when it fires | arguments |
|------|---------------|-----------|
| `startup` | editor starts | none |
| `shutdown` | editor closes | none |
| `open` | file is opened | path (string) |
| `save` | file is saved | path (string) |

```lua
kw:on("open", function(path)
    kittywrite.log("opened: " .. path)
end)

kw:on("save", function(path)
    kittywrite.log("saved: " .. path)
end)
```

### commands

commands appear in the plugin panel and can be triggered by the user.

```lua
kw:command("my-command", "what it does", function()
    -- do stuff
    return "done!"
end)
```

the return value is shown in the status bar.

### accessing files

plugins can read and write files using the built-in functions:

```lua
-- read a file
local content = read_file("/path/to/file.txt")

-- check if file exists
if file_exists("/path/to/config.json") then
    local data = read_file("/path/to/config.json")
end

-- write a file
write_file("/path/to/output.txt", "hello world")

-- list directory
local files = read_dir("/path/to/folder")
for _, f in ipairs(files) do
    kittywrite.log(f)
end
```

### example plugins

#### word counter

```lua
local kw = plugin({
    name = "word-count",
    version = "1.0.0",
    description = "counts words in current file",
    author = "kittywrite",
})

kw:on("open", function()
    local content = kittywrite.get_content()
    local count = 0
    for _ in content:gmatch("%S+") do
        count = count + 1
    end
    kittywrite.notify("words: " .. count)
end)
```

#### file info

```lua
local kw = plugin({
    name = "file-info",
    version = "1.0.0",
    description = "shows file info",
    author = "kittywrite",
})

kw:command("file-info", "show file info", function()
    local name = kittywrite.get_file_name()
    local lines = kittywrite.get_line_count()
    local theme = kittywrite.get_theme()
    local font = kittywrite.get_font_size()
    return string.format("%s | %d lines | %s | %.0fpx", name, lines, theme, font)
end)
```

#### theme switcher

```lua
local kw = plugin({
    name = "theme-switcher",
    version = "1.0.0",
    description = "cycles through themes",
    author = "kittywrite",
})

local themes = {"kittywrite", "mocha", "frappe", "macchiato", "latte"}
local idx = 1

kw:command("next-theme", "switch to next theme", function()
    idx = idx % #themes + 1
    kittywrite.set_theme(themes[idx])
    return "theme: " .. themes[idx]
end)
```

### installing plugins via ui

1. get a plugin folder (with `init.lua` inside)
2. open plugins panel (`view > plugins`)
3. click "install plugin"
4. select the plugin folder
5. restart or click refresh

### tips

- plugins run in lua 5.4
- the `kittywrite` table gives you editor access
- `kittywrite.log()` prints to the lua console
- `kittywrite.notify()` shows in the status bar
- commands return strings that show in the status bar
- hooks don't return values
- check the lua console for error messages if something breaks

---

## plugin api reference

### file operations

| function | signature | description |
|----------|-----------|-------------|
| `kittywrite.open_file` | `(path)` | open a file in a new tab |
| `kittywrite.save_file` | `()` | save current file |
| `kittywrite.new_file` | `()` | create new empty tab |

### content

| function | signature | description |
|----------|-----------|-------------|
| `kittywrite.get_content` | `() -> string` | get all text in editor |
| `kittywrite.set_content` | `(text)` | replace all text |
| `kittywrite.get_selection` | `() -> string` | get selected text |
| `kittywrite.set_selection` | `(text)` | replace selection |

### cursor

| function | signature | description |
|----------|-----------|-------------|
| `kittywrite.get_cursor_line` | `() -> number` | current line (1-indexed) |
| `kittywrite.get_cursor_col` | `() -> number` | current column (0-indexed) |
| `kittywrite.set_cursor` | `(line, col)` | move cursor to position |

### file info

| function | signature | description |
|----------|-----------|-------------|
| `kittywrite.get_file_path` | `() -> string` | full path of current file |
| `kittywrite.get_file_name` | `() -> string` | file name only |
| `kittywrite.get_line_count` | `() -> number` | total lines in file |
| `kittywrite.get_line` | `(n) -> string` | get content of line n |

### theme and font

| function | signature | description |
|----------|-----------|-------------|
| `kittywrite.get_theme` | `() -> string` | current theme name |
| `kittywrite.set_theme` | `(name)` | switch theme |
| `kittywrite.get_font_size` | `() -> number` | current font size |
| `kittywrite.set_font_size` | `(size)` | set font size (8-48) |

### ui

| function | signature | description |
|----------|-----------|-------------|
| `kittywrite.notify` | `(msg)` | show message in status bar |
| `kittywrite.log` | `(msg)` | print to lua console |

### file system

| function | signature | description |
|----------|-----------|-------------|
| `read_file` | `(path) -> string` | read file contents |
| `write_file` | `(path, content)` | write file contents |
| `file_exists` | `(path) -> bool` | check if file exists |
| `read_dir` | `(path) -> {paths}` | list directory contents |

---

## plugin hooks reference

| hook | fires when | callback args |
|------|------------|---------------|
| `startup` | editor starts | none |
| `shutdown` | editor closes | none |
| `open` | file opened | path (string) |
| `save` | file saved | path (string) |
