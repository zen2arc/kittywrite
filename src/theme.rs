use egui::{Color32, CornerRadius, Stroke, Visuals};

#[derive(Clone, Copy)]
pub struct CatTheme {
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
}

fn h(hex: &str) -> Color32 {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    Color32::from_rgb(r, g, b)
}

fn kittywrite_original() -> CatTheme {
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
    }
}

// catppuccin mocha - exact values from palette v1.8.0
fn catppuccin_mocha() -> CatTheme {
    CatTheme {
        bg_void: h("#11111b"),                                             // crust
        bg_panel: h("#1e1e2e"),                                            // base
        bg_editor: h("#181825"),                                           // mantle
        bg_tab_idle: h("#313244"),                                         // surface0
        bg_tab_active: h("#45475a"),                                       // surface1
        fg_main: h("#cdd6f4"),                                             // text
        fg_dim: h("#7f849c"),                                              // overlay1
        fg_gutter: h("#45475a"),                                           // surface1
        accent_paw: h("#f38ba8"),                                          // red
        accent_eye: h("#f9e2af"),                                          // yellow
        accent_fur: h("#a6e3a1"),                                          // green
        selection_bg: Color32::from_rgba_premultiplied(137, 180, 250, 60), // blue a=0.24
    }
}

// catppuccin frappe - exact values from palette v1.8.0
fn catppuccin_frappe() -> CatTheme {
    CatTheme {
        bg_void: h("#232634"),                                             // crust
        bg_panel: h("#303446"),                                            // base
        bg_editor: h("#292c3c"),                                           // mantle
        bg_tab_idle: h("#414559"),                                         // surface0
        bg_tab_active: h("#51576d"),                                       // surface1
        fg_main: h("#c6d0f5"),                                             // text
        fg_dim: h("#838ba7"),                                              // overlay1
        fg_gutter: h("#51576d"),                                           // surface1
        accent_paw: h("#e78284"),                                          // red
        accent_eye: h("#e5c890"),                                          // yellow
        accent_fur: h("#a6d189"),                                          // green
        selection_bg: Color32::from_rgba_premultiplied(140, 170, 238, 60), // blue a=0.24
    }
}

// catppuccin macchiato - exact values from palette v1.8.0
fn catppuccin_macchiato() -> CatTheme {
    CatTheme {
        bg_void: h("#181924"),                                             // crust
        bg_panel: h("#24273a"),                                            // base
        bg_editor: h("#1e2030"),                                           // mantle
        bg_tab_idle: h("#363a4f"),                                         // surface0
        bg_tab_active: h("#494d64"),                                       // surface1
        fg_main: h("#cad3f5"),                                             // text
        fg_dim: h("#8087a2"),                                              // overlay1
        fg_gutter: h("#494d64"),                                           // surface1
        accent_paw: h("#ed8796"),                                          // red
        accent_eye: h("#eed49f"),                                          // yellow
        accent_fur: h("#a6da95"),                                          // green
        selection_bg: Color32::from_rgba_premultiplied(138, 173, 244, 60), // blue a=0.24
    }
}

// catppuccin latte - exact values from palette v1.8.0
fn catppuccin_latte() -> CatTheme {
    CatTheme {
        bg_void: h("#dce0e8"),                                            // crust
        bg_panel: h("#eff1f5"),                                           // base
        bg_editor: h("#e6e9ef"),                                          // mantle
        bg_tab_idle: h("#ccd0da"),                                        // surface0
        bg_tab_active: h("#bcc0cc"),                                      // surface1
        fg_main: h("#4c4f69"),                                            // text
        fg_dim: h("#8c8fa1"),                                             // overlay1
        fg_gutter: h("#bcc0cc"),                                          // surface1
        accent_paw: h("#d20f39"),                                         // red
        accent_eye: h("#df8e1d"),                                         // yellow
        accent_fur: h("#40a02b"),                                         // green
        selection_bg: Color32::from_rgba_premultiplied(30, 102, 245, 60), // blue a=0.24
    }
}

impl Default for CatTheme {
    fn default() -> Self {
        kittywrite_original()
    }
}

impl CatTheme {
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "kittywrite" => kittywrite_original(),
            "mocha" => catppuccin_mocha(),
            "frappe" | "frappé" => catppuccin_frappe(),
            "macchiato" => catppuccin_macchiato(),
            "latte" => catppuccin_latte(),
            _ => kittywrite_original(),
        }
    }

    pub fn list() -> &'static [&'static str] {
        &["kittywrite", "mocha", "frappe", "macchiato", "latte"]
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
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
        ctx.set_style(style);
    }
}
