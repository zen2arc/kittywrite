use egui::{Color32, CornerRadius, Stroke, Visuals};

#[derive(Clone, Copy)]
pub struct CatTheme {
    // ui colors
    pub bg_void: Color32,
    pub bg_panel: Color32,
    pub bg_editor: Color32,
    pub bg_tab_idle: Color32,
    pub bg_tab_active: Color32,
    pub fg_main: Color32,
    pub fg_dim: Color32,
    pub fg_gutter: Color32,
    pub accent_paw: Color32,
    pub accent_eye: Color32,
    pub accent_fur: Color32,
    pub selection_bg: Color32,
    // syntax colors
    pub syntax_keyword: Color32,
    pub syntax_function: Color32,
    pub syntax_string: Color32,
    pub syntax_comment: Color32,
    pub syntax_number: Color32,
    pub syntax_type: Color32,
    pub syntax_variable: Color32,
    pub syntax_operator: Color32,
    pub syntax_punctuation: Color32,
}

#[derive(Clone)]
pub struct ThemeInfo {
    pub name: String,
    pub author: String,
    pub is_light: bool,
    pub theme: CatTheme,
}

fn h(hex: &str) -> Color32 {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Color32::from_rgb(r, g, b)
}

fn h_alpha(hex: &str, alpha: u8) -> Color32 {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Color32::from_rgba_premultiplied(r, g, b, alpha)
}

fn default_syntax() -> (
    Color32,
    Color32,
    Color32,
    Color32,
    Color32,
    Color32,
    Color32,
    Color32,
    Color32,
) {
    (
        h("#c678dd"), // keyword
        h("#61afef"), // function
        h("#98c379"), // string
        h("#5c6370"), // comment
        h("#d19a66"), // number
        h("#e5c07b"), // type
        h("#e06c75"), // variable
        h("#56b6c2"), // operator
        h("#abb2bf"), // punctuation
    )
}

fn kittywrite_original() -> CatTheme {
    let (kw, func, str_s, cmt, num, typ, var, op, punct) = default_syntax();
    CatTheme {
        bg_void: h("#0b0b12"),
        bg_panel: h("#11111c"),
        bg_editor: h("#0f0f18"),
        bg_tab_idle: h("#151522"),
        bg_tab_active: h("#1e1e32"),
        fg_main: h("#d4cfe2"),
        fg_dim: h("#69647d"),
        fg_gutter: h("#37344b"),
        accent_paw: h("#da8aa2"),
        accent_eye: h("#f8bc4b"),
        accent_fur: h("#ac9cd0"),
        selection_bg: Color32::from_rgba_premultiplied(85, 65, 135, 65),
        syntax_keyword: kw,
        syntax_function: func,
        syntax_string: str_s,
        syntax_comment: cmt,
        syntax_number: num,
        syntax_type: typ,
        syntax_variable: var,
        syntax_operator: op,
        syntax_punctuation: punct,
    }
}

fn catppuccin_mocha() -> CatTheme {
    CatTheme {
        bg_void: h("#11111b"),
        bg_panel: h("#1e1e2e"),
        bg_editor: h("#181825"),
        bg_tab_idle: h("#313244"),
        bg_tab_active: h("#45475a"),
        fg_main: h("#cdd6f4"),
        fg_dim: h("#7f849c"),
        fg_gutter: h("#45475a"),
        accent_paw: h("#f38ba8"),
        accent_eye: h("#f9e2af"),
        accent_fur: h("#a6e3a1"),
        selection_bg: Color32::from_rgba_premultiplied(137, 180, 250, 60),
        syntax_keyword: h("#cba6f7"),
        syntax_function: h("#89b4fa"),
        syntax_string: h("#a6e3a1"),
        syntax_comment: h("#6c7086"),
        syntax_number: h("#fab387"),
        syntax_type: h("#f9e2af"),
        syntax_variable: h("#f38ba8"),
        syntax_operator: h("#89dceb"),
        syntax_punctuation: h("#cdd6f4"),
    }
}

fn catppuccin_frappe() -> CatTheme {
    CatTheme {
        bg_void: h("#232634"),
        bg_panel: h("#303446"),
        bg_editor: h("#292c3c"),
        bg_tab_idle: h("#414559"),
        bg_tab_active: h("#51576d"),
        fg_main: h("#c6d0f5"),
        fg_dim: h("#838ba7"),
        fg_gutter: h("#51576d"),
        accent_paw: h("#e78284"),
        accent_eye: h("#e5c890"),
        accent_fur: h("#a6d189"),
        selection_bg: Color32::from_rgba_premultiplied(140, 170, 238, 60),
        syntax_keyword: h("#ca9ee6"),
        syntax_function: h("#8caaee"),
        syntax_string: h("#a6d189"),
        syntax_comment: h("#737994"),
        syntax_number: h("#ef9f76"),
        syntax_type: h("#e5c890"),
        syntax_variable: h("#e78284"),
        syntax_operator: h("#81d8b8"),
        syntax_punctuation: h("#c6d0f5"),
    }
}

fn catppuccin_macchiato() -> CatTheme {
    CatTheme {
        bg_void: h("#181924"),
        bg_panel: h("#24273a"),
        bg_editor: h("#1e2030"),
        bg_tab_idle: h("#363a4f"),
        bg_tab_active: h("#494d64"),
        fg_main: h("#cad3f5"),
        fg_dim: h("#8087a2"),
        fg_gutter: h("#494d64"),
        accent_paw: h("#ed8796"),
        accent_eye: h("#eed49f"),
        accent_fur: h("#a6da95"),
        selection_bg: Color32::from_rgba_premultiplied(138, 173, 244, 60),
        syntax_keyword: h("#c6a0f6"),
        syntax_function: h("#8aadf4"),
        syntax_string: h("#a6da95"),
        syntax_comment: h("#6c7086"),
        syntax_number: h("#f5a97f"),
        syntax_type: h("#eed49f"),
        syntax_variable: h("#ed8796"),
        syntax_operator: h("#8bd5ca"),
        syntax_punctuation: h("#cad3f5"),
    }
}

fn catppuccin_latte() -> CatTheme {
    CatTheme {
        bg_void: h("#dce0e8"),
        bg_panel: h("#eff1f5"),
        bg_editor: h("#e6e9ef"),
        bg_tab_idle: h("#ccd0da"),
        bg_tab_active: h("#bcc0cc"),
        fg_main: h("#4c4f69"),
        fg_dim: h("#8c8fa1"),
        fg_gutter: h("#bcc0cc"),
        accent_paw: h("#d20f39"),
        accent_eye: h("#df8e1d"),
        accent_fur: h("#40a02b"),
        selection_bg: Color32::from_rgba_premultiplied(30, 102, 245, 60),
        syntax_keyword: h("#8839ef"),
        syntax_function: h("#1e66f5"),
        syntax_string: h("#40a02b"),
        syntax_comment: h("#9ca0b0"),
        syntax_number: h("#fe640b"),
        syntax_type: h("#df8e1d"),
        syntax_variable: h("#d20f39"),
        syntax_operator: h("#179299"),
        syntax_punctuation: h("#4c4f69"),
    }
}

impl Default for CatTheme {
    fn default() -> Self {
        kittywrite_original()
    }
}

impl CatTheme {
    pub fn from_name(name: &str) -> Self {
        let themes = load_themes();
        for t in themes {
            if t.name.to_lowercase() == name.to_lowercase() {
                return t.theme;
            }
        }
        kittywrite_original()
    }

    pub fn list() -> Vec<String> {
        let themes = load_themes();
        themes.iter().map(|t| t.name.clone()).collect()
    }

    pub fn load_from_file(path: &std::path::Path) -> Option<ThemeInfo> {
        let content = std::fs::read_to_string(path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;

        let name = json.get("name")?.as_str()?.to_string();
        let author = json
            .get("author")
            .and_then(|a| a.as_str())
            .unwrap_or("unknown")
            .to_string();
        let is_light = json
            .get("is_light")
            .and_then(|l| l.as_bool())
            .unwrap_or(false);

        let ui = json.get("ui")?;
        let syntax = json.get("syntax");

        let get_color = |key: &str, default: &str| -> Color32 {
            ui.get(key)
                .and_then(|v| v.as_str())
                .map(|s| h(s))
                .unwrap_or_else(|| h(default))
        };

        let get_syntax_color = |key: &str, default: &str| -> Color32 {
            syntax
                .and_then(|s| s.get(key))
                .and_then(|v| v.as_str())
                .map(|s| h(s))
                .unwrap_or_else(|| h(default))
        };

        let theme = CatTheme {
            bg_void: get_color("bg_void", "#0b0b12"),
            bg_panel: get_color("bg_panel", "#11111c"),
            bg_editor: get_color("bg_editor", "#0f0f18"),
            bg_tab_idle: get_color("bg_tab_idle", "#151522"),
            bg_tab_active: get_color("bg_tab_active", "#1e1e32"),
            fg_main: get_color("fg_main", "#d4cfe2"),
            fg_dim: get_color("fg_dim", "#69647d"),
            fg_gutter: get_color("fg_gutter", "#37344b"),
            accent_paw: get_color("accent_paw", "#da8aa2"),
            accent_eye: get_color("accent_eye", "#f8bc4b"),
            accent_fur: get_color("accent_fur", "#ac9cd0"),
            selection_bg: ui
                .get("selection_bg")
                .and_then(|v| v.as_str())
                .map(|s| h_alpha(s, 60))
                .unwrap_or_else(|| Color32::from_rgba_premultiplied(85, 65, 135, 65)),
            syntax_keyword: get_syntax_color("keyword", "#c678dd"),
            syntax_function: get_syntax_color("function", "#61afef"),
            syntax_string: get_syntax_color("string", "#98c379"),
            syntax_comment: get_syntax_color("comment", "#5c6370"),
            syntax_number: get_syntax_color("number", "#d19a66"),
            syntax_type: get_syntax_color("type", "#e5c07b"),
            syntax_variable: get_syntax_color("variable", "#e06c75"),
            syntax_operator: get_syntax_color("operator", "#56b6c2"),
            syntax_punctuation: get_syntax_color("punctuation", "#abb2bf"),
        };

        Some(ThemeInfo {
            name,
            author,
            is_light,
            theme,
        })
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = (*ctx.global_style()).clone();
        let mut vis = Visuals::dark();
        if self.fg_main.r() > 100
            && self.fg_main.g() > 100
            && self.fg_main.b() > 100
            && self.bg_panel.r() > 200
        {
            vis = Visuals::light();
        }
        vis.override_text_color = Some(self.fg_main);
        vis.extreme_bg_color = self.bg_editor;
        vis.faint_bg_color = self.bg_panel;
        vis.code_bg_color = self.bg_void;
        vis.window_fill = self.bg_panel;
        vis.panel_fill = self.bg_panel;
        vis.widgets.noninteractive.bg_fill = self.bg_panel;
        vis.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.fg_dim);
        vis.widgets.noninteractive.corner_radius = CornerRadius::same(4);
        vis.widgets.inactive.bg_fill = self.bg_tab_idle;
        vis.widgets.inactive.fg_stroke = Stroke::new(1.0, self.fg_dim);
        vis.widgets.inactive.corner_radius = CornerRadius::same(4);
        vis.widgets.hovered.bg_fill = self.bg_tab_active;
        vis.widgets.hovered.fg_stroke = Stroke::new(1.0, self.fg_main);
        vis.widgets.hovered.corner_radius = CornerRadius::same(4);
        vis.widgets.active.bg_fill = self.accent_paw;
        vis.widgets.active.fg_stroke = Stroke::new(1.0, self.bg_void);
        vis.widgets.active.corner_radius = CornerRadius::same(4);
        vis.selection.bg_fill = self.selection_bg;
        vis.selection.stroke = Stroke::new(1.0, self.accent_eye);
        vis.window_corner_radius = CornerRadius::same(6);
        style.visuals = vis;
        ctx.set_global_style(style);
    }
}

fn load_themes() -> Vec<ThemeInfo> {
    // start with built-in themes
    let mut themes = vec![
        ThemeInfo {
            name: "kittywrite".to_string(),
            author: "kittywrite".to_string(),
            is_light: false,
            theme: kittywrite_original(),
        },
        ThemeInfo {
            name: "mocha".to_string(),
            author: "catppuccin".to_string(),
            is_light: false,
            theme: catppuccin_mocha(),
        },
        ThemeInfo {
            name: "frappe".to_string(),
            author: "catppuccin".to_string(),
            is_light: false,
            theme: catppuccin_frappe(),
        },
        ThemeInfo {
            name: "macchiato".to_string(),
            author: "catppuccin".to_string(),
            is_light: false,
            theme: catppuccin_macchiato(),
        },
        ThemeInfo {
            name: "latte".to_string(),
            author: "catppuccin".to_string(),
            is_light: true,
            theme: catppuccin_latte(),
        },
    ];

    // load custom themes from themes/ directory
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let themes_dir = dir.join("themes");
            if let Ok(entries) = std::fs::read_dir(&themes_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("json") {
                        if let Some(info) = CatTheme::load_from_file(&path) {
                            // don't override built-in themes
                            if !themes.iter().any(|t| t.name == info.name) {
                                themes.push(info);
                            }
                        }
                    }
                }
            }
        }
    }

    themes
}
