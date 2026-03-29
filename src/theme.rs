use egui::{Color32, CornerRadius, Stroke, Visuals};

#[derive(Clone, Copy)]
pub struct CatTheme {
    pub bg_void: Color32,
    pub bg_panel: Color32,
    pub bg_editor: Color32,
    pub bg_tab_idle: Color32,
    pub bg_tab_active: Color32,

    // text
    pub fg_main: Color32,
    pub fg_dim: Color32,
    pub fg_gutter: Color32,

    pub accent_paw: Color32,
    pub accent_eye: Color32,
    pub accent_fur: Color32,

    pub selection_bg: Color32,
}

impl Default for CatTheme {
    fn default() -> Self {
        Self {
            bg_void: Color32::from_rgb(11, 11, 18),
            bg_panel: Color32::from_rgb(17, 17, 28),
            bg_editor: Color32::from_rgb(15, 15, 24),
            bg_tab_idle: Color32::from_rgb(21, 21, 34),
            bg_tab_active: Color32::from_rgb(30, 30, 50),

            fg_main: Color32::from_rgb(212, 207, 226),
            fg_dim: Color32::from_rgb(105, 100, 125),
            fg_gutter: Color32::from_rgb(55, 52, 75),

            accent_paw: Color32::from_rgb(218, 138, 162),
            accent_eye: Color32::from_rgb(248, 188, 75),
            accent_fur: Color32::from_rgb(172, 152, 208),

            selection_bg: Color32::from_rgba_premultiplied(85, 65, 135, 65),
        }
    }
}

impl CatTheme {
    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        let mut vis = Visuals::dark();

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
