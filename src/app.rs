use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{CornerRadius, Frame, Margin};
use egui_phosphor::regular as ph;

use crate::editor::{detect_language, EditorTab};
use crate::filetree::FileTree;
use crate::highlighter::{self, Highlighter};
use crate::lua_engine::LuaEngine;
use crate::theme::CatTheme;

const HL_DEBOUNCE_MS: u64 = 150;

#[derive(Default)]
struct FrameActions {
    new_tab: bool,
    open_file: bool,
    open_folder: bool,
    save: bool,
    save_as: bool,
    close_tab: Option<usize>,
    switch_tab: Option<usize>,
    quit: bool,
    toggle_find: bool,
    toggle_replace: bool,
    toggle_file_tree: bool,
    toggle_quick_open: bool,
    toggle_settings: bool,
    find_next: bool,
    find_prev: bool,
    replace_one: bool,
    replace_all_matches: bool,
}

fn compute_matches(haystack: &str, needle: &str) -> Vec<usize> {
    if needle.is_empty() {
        return vec![];
    }
    let mut v = Vec::new();
    let mut pos = 0;
    while let Some(off) = haystack[pos..].find(needle) {
        v.push(pos + off);
        pos += off + needle.len().max(1);
    }
    v
}

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

pub struct KittyWriteApp {
    tabs: Vec<EditorTab>,
    active_tab: usize,

    hl: Arc<Highlighter>,
    theme: CatTheme,
    lua: LuaEngine,
    status: String,

    show_about: bool,
    show_settings: bool,
    show_lua_console: bool,
    lua_input: String,
    lua_output: String,

    show_find: bool,
    show_replace_bar: bool,
    find_query: String,
    replace_query: String,
    find_matches: Vec<usize>,
    find_match_idx: usize,
    find_cache_key: (String, usize),

    file_tree: FileTree,
    show_file_tree: bool,

    pending_indent: Option<(usize, String, usize)>,
    pending_cursor: Option<(usize, Option<usize>)>,

    last_edit_instant: Instant,
    content_generation: u64,

    recent_files: Vec<String>,
    show_quick_open: bool,
    quick_open_query: String,
    quick_open_selected: usize,

    git_diff_lines: std::collections::HashSet<usize>,
    git_diff_file: Option<std::path::PathBuf>,
    last_theme_name: String,
}

impl KittyWriteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let lua = LuaEngine::new();
        let theme = CatTheme::from_name(&lua.config.theme);
        let theme_name = lua.config.theme.clone();
        theme.apply(&cc.egui_ctx);

        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        let mut style = (*cc.egui_ctx.style()).clone();
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(13.0));
        cc.egui_ctx.set_style(style);

        Self {
            tabs: vec![EditorTab::new_empty()],
            active_tab: 0,
            hl: Arc::new(Highlighter::default()),
            theme,
            lua,
            status: "ready".to_string(),
            show_about: false,
            show_settings: false,
            show_lua_console: false,
            lua_input: String::new(),
            lua_output: String::new(),
            show_find: false,
            show_replace_bar: false,
            find_query: String::new(),
            replace_query: String::new(),
            find_matches: Vec::new(),
            find_match_idx: 0,
            find_cache_key: (String::new(), 0),
            file_tree: FileTree::default(),
            show_file_tree: false,
            pending_indent: None,
            pending_cursor: None,
            last_edit_instant: Instant::now() - Duration::from_secs(10),
            content_generation: 0,
            recent_files: load_recent_files(),
            show_quick_open: false,
            quick_open_query: String::new(),
            quick_open_selected: 0,
            git_diff_lines: std::collections::HashSet::new(),
            git_diff_file: None,
            last_theme_name: theme_name,
        }
    }

    fn new_tab(&mut self) {
        self.tabs.push(EditorTab::new_empty());
        self.active_tab = self.tabs.len() - 1;
        self.pending_indent = None;
    }

    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            match EditorTab::from_path(path) {
                Ok(tab) => {
                    self.status = format!("opened {}", tab.title);
                    let path_str = tab
                        .path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default();
                    self.tabs.push(tab);
                    self.active_tab = self.tabs.len() - 1;
                    self.pending_indent = None;
                    self.add_recent_file(path_str);
                }
                Err(e) => self.status = format!("open failed: {}", e),
            }
        }
    }

    fn open_file_path(&mut self, path: std::path::PathBuf) {
        if let Some(idx) = self
            .tabs
            .iter()
            .position(|t| t.path.as_deref() == Some(&path))
        {
            self.active_tab = idx;
            return;
        }
        match EditorTab::from_path(path) {
            Ok(tab) => {
                self.status = format!("opened {}", tab.title);
                let path_str = tab
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
                self.pending_indent = None;
                self.add_recent_file(path_str);
            }
            Err(e) => self.status = format!("open failed: {}", e),
        }
    }

    fn add_recent_file(&mut self, path: String) {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        if self.recent_files.len() > 20 {
            self.recent_files.truncate(20);
        }
        save_recent_files(&self.recent_files);
    }

    fn save_current(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let idx = self.active_tab;
        if self.tabs[idx].path.is_none() {
            if let Some(path) = rfd::FileDialog::new().save_file() {
                let title = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("untitled")
                    .to_string();
                let lang = detect_language(&path);
                self.tabs[idx].path = Some(path);
                self.tabs[idx].title = title;
                self.tabs[idx].language = lang;
            } else {
                return;
            }
        }
        match self.tabs[idx].save() {
            Ok(_) => {
                let n = self.tabs[idx].title.clone();
                self.status = format!("saved {}", n);
            }
            Err(e) => self.status = format!("save failed: {}", e),
        }
    }

    fn save_as(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.tabs[self.active_tab].path = None;
        self.save_current();
    }

    fn close_tab(&mut self, idx: usize) {
        self.pending_indent = None;
        if self.tabs.len() == 1 {
            self.tabs[0] = EditorTab::new_empty();
            self.active_tab = 0;
            return;
        }
        self.tabs.remove(idx);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }

    fn navigate_match(&mut self, forward: bool) {
        if self.find_matches.is_empty() {
            return;
        }
        let n = self.find_matches.len();
        if forward {
            self.find_match_idx = (self.find_match_idx + 1) % n;
        } else {
            self.find_match_idx = (self.find_match_idx + n - 1) % n;
        }
        let byte_start = self.find_matches[self.find_match_idx];
        let byte_end = byte_start + self.find_query.len();
        self.pending_cursor = Some((byte_end, Some(byte_start)));
    }

    fn replace_current(&mut self, content: &mut String) -> bool {
        if self.find_matches.is_empty() || self.find_query.is_empty() {
            return false;
        }
        let idx = self.find_match_idx.min(self.find_matches.len() - 1);
        let byte_start = self.find_matches[idx];
        let byte_end = byte_start + self.find_query.len();
        if byte_end > content.len() {
            return false;
        }
        content.replace_range(byte_start..byte_end, &self.replace_query);
        true
    }

    fn replace_all(&mut self, content: &mut String) -> usize {
        if self.find_query.is_empty() {
            return 0;
        }
        let old = content.clone();
        let count = compute_matches(&old, &self.find_query).len();
        *content = old.replace(&self.find_query, &self.replace_query);
        self.find_match_idx = 0;
        count
    }
}

impl eframe::App for KittyWriteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // check if theme changed via lua config
        if self.lua.config.theme != self.last_theme_name {
            self.theme = CatTheme::from_name(&self.lua.config.theme);
            self.theme.apply(ctx);
            self.last_theme_name = self.lua.config.theme.clone();
        }

        let mut act = FrameActions::default();
        let enter_pressed = ctx.input(|i| i.key_pressed(egui::Key::Enter));

        ctx.input(|i| {
            let ctrl = i.modifiers.ctrl;
            let shift = i.modifiers.shift;
            if ctrl && i.key_pressed(egui::Key::N) {
                act.new_tab = true;
            }
            if ctrl && !shift && i.key_pressed(egui::Key::O) {
                act.open_file = true;
            }
            if ctrl && shift && i.key_pressed(egui::Key::O) {
                act.open_folder = true;
            }
            if ctrl && !shift && i.key_pressed(egui::Key::S) {
                act.save = true;
            }
            if ctrl && shift && i.key_pressed(egui::Key::S) {
                act.save_as = true;
            }
            if ctrl && i.key_pressed(egui::Key::W) {
                act.close_tab = Some(self.active_tab);
            }
            if ctrl && !shift && i.key_pressed(egui::Key::F) {
                act.toggle_find = true;
            }
            if ctrl && shift && i.key_pressed(egui::Key::F) {
                act.toggle_replace = true;
            }
            if ctrl && i.key_pressed(egui::Key::H) {
                act.toggle_replace = true;
            }
            if ctrl && i.key_pressed(egui::Key::B) {
                act.toggle_file_tree = true;
            }
            if ctrl && !shift && i.key_pressed(egui::Key::P) {
                act.toggle_quick_open = true;
            }
            if ctrl && i.key_pressed(egui::Key::Comma) {
                act.toggle_settings = true;
            }
            if i.key_pressed(egui::Key::F3) && !shift {
                act.find_next = true;
            }
            if i.key_pressed(egui::Key::F3) && shift {
                act.find_prev = true;
            }
            if i.key_pressed(egui::Key::Escape) && self.show_find {
                act.toggle_find = true;
            }
            if i.key_pressed(egui::Key::Escape) && self.show_quick_open {
                act.toggle_quick_open = true;
            }
        });

        let theme = self.theme;
        let font_size = self.lua.config.font_size;
        let show_ln = self.lua.config.show_line_numbers;
        let line_height = self.lua.config.line_height;
        let match_count = self.find_matches.len();
        let match_idx = self.find_match_idx;

        egui::TopBottomPanel::top("menu_bar")
            .frame(
                Frame::none()
                    .fill(theme.bg_void)
                    .inner_margin(Margin::symmetric(6, 4)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.label(
                        egui::RichText::new("=^.^=")
                            .color(theme.accent_paw)
                            .size(13.0),
                    );
                    ui.add_space(6.0);

                    ui.menu_button("file", |ui| {
                        ui.set_min_width(260.0);
                        if ui
                            .button(format!("{}  new tab             ctrl+n", ph::PLUS))
                            .clicked()
                        {
                            act.new_tab = true;
                            ui.close_menu();
                        }
                        if ui
                            .button(format!("{}  open file…          ctrl+o", ph::FOLDER_OPEN))
                            .clicked()
                        {
                            act.open_file = true;
                            ui.close_menu();
                        }
                        if ui
                            .button(format!("{}  open folder…    ctrl+shift+o", ph::FOLDERS))
                            .clicked()
                        {
                            act.open_folder = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(format!("{}  save                ctrl+s", ph::FLOPPY_DISK))
                            .clicked()
                        {
                            act.save = true;
                            ui.close_menu();
                        }
                        if ui
                            .button(format!(
                                "{}  save as…        ctrl+shift+s",
                                ph::FLOPPY_DISK_BACK
                            ))
                            .clicked()
                        {
                            act.save_as = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(format!("{}  close tab           ctrl+w", ph::X))
                            .clicked()
                        {
                            act.close_tab = Some(self.active_tab);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(format!("{}  quit", ph::SIGN_OUT)).clicked() {
                            act.quit = true;
                        }
                    });

                    ui.menu_button("edit", |ui| {
                        ui.set_min_width(260.0);
                        if ui
                            .button(format!(
                                "{}  find               ctrl+f",
                                ph::MAGNIFYING_GLASS
                            ))
                            .clicked()
                        {
                            act.toggle_find = true;
                            ui.close_menu();
                        }
                        if ui
                            .button(format!(
                                "{}  find & replace      ctrl+h",
                                ph::ARROWS_CLOCKWISE
                            ))
                            .clicked()
                        {
                            act.toggle_replace = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new("   undo       ctrl+z")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new("   redo       ctrl+y")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("   cut        ctrl+x")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new("   copy       ctrl+c")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new("   paste      ctrl+v")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("   select all ctrl+a")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                    });

                    ui.menu_button("view", |ui| {
                        ui.set_min_width(260.0);
                        ui.checkbox(&mut self.lua.config.show_line_numbers, "line numbers");
                        ui.checkbox(&mut self.lua.config.word_wrap, "word wrap");
                        ui.checkbox(&mut self.lua.config.auto_indent, "auto indent");
                        ui.checkbox(&mut self.lua.config.auto_pair, "auto pair brackets");
                        ui.separator();
                        ui.label(
                            egui::RichText::new("font size")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.lua.config.font_size, 8.0..=32.0)
                                .step_by(1.0)
                                .suffix("px"),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("line height")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.lua.config.line_height, 1.0..=2.5)
                                .step_by(0.1)
                                .suffix("×"),
                        );
                    });

                    ui.menu_button("tools", |ui| {
                        ui.set_min_width(260.0);
                        if ui
                            .button(format!("{}  file explorer   ctrl+b", ph::TREE_STRUCTURE))
                            .clicked()
                        {
                            act.toggle_file_tree = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(format!("{}  lua console", ph::TERMINAL_WINDOW))
                            .clicked()
                        {
                            self.show_lua_console = !self.show_lua_console;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(format!("{}  settings", ph::GEAR)).clicked() {
                            self.show_settings = !self.show_settings;
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("help", |ui| {
                        ui.set_min_width(260.0);
                        if ui.button(format!("{}  about", ph::INFO)).clicked() {
                            self.show_about = true;
                            ui.close_menu();
                        }
                    });
                });
            });

        egui::TopBottomPanel::top("tab_bar")
            .frame(Frame::none().fill(theme.bg_void).inner_margin(Margin {
                left: 4,
                right: 4,
                top: 4,
                bottom: 0,
            }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for i in 0..self.tabs.len() {
                        let is_active = i == self.active_tab;
                        let bg = if is_active {
                            theme.bg_tab_active
                        } else {
                            theme.bg_tab_idle
                        };
                        let lc = if is_active {
                            theme.accent_eye
                        } else {
                            theme.fg_dim
                        };
                        let label = self.tabs[i].tab_label();
                        Frame::none()
                            .fill(bg)
                            .corner_radius(CornerRadius {
                                nw: 4,
                                ne: 4,
                                sw: 0,
                                se: 0,
                            })
                            .inner_margin(Margin {
                                left: 10,
                                right: 6,
                                top: 4,
                                bottom: 4,
                            })
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 5.0;
                                    let tr = ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(&label).color(lc).size(13.0),
                                        )
                                        .sense(egui::Sense::click()),
                                    );
                                    if tr.clicked() {
                                        act.switch_tab = Some(i);
                                    }
                                    let xr = ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(ph::X)
                                                .color(if is_active {
                                                    theme.fg_dim
                                                } else {
                                                    theme.fg_gutter
                                                })
                                                .size(13.0),
                                        )
                                        .sense(egui::Sense::click()),
                                    );
                                    if xr.clicked() {
                                        act.close_tab = Some(i);
                                    }
                                    if xr.hovered() {
                                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                                    }
                                });
                            });
                    }
                    let nr = ui.add(
                        egui::Label::new(
                            egui::RichText::new(ph::PLUS).color(theme.fg_dim).size(16.0),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if nr.clicked() {
                        act.new_tab = true;
                    }
                    if nr.hovered() {
                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                });
            });

        egui::TopBottomPanel::bottom("status_bar")
            .frame(
                Frame::none()
                    .fill(theme.bg_void)
                    .inner_margin(Margin::symmetric(10, 3)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let lang = self
                        .tabs
                        .get(self.active_tab)
                        .map(|t| t.language.as_str())
                        .unwrap_or("--")
                        .to_string();
                    let lines = self
                        .tabs
                        .get(self.active_tab)
                        .map(|t| t.line_count())
                        .unwrap_or(0);
                    ui.label(
                        egui::RichText::new(format!("{}  {}", ph::FILE_CODE, lang))
                            .color(theme.accent_fur)
                            .size(11.0),
                    );
                    ui.add(egui::Separator::default().vertical());
                    ui.label(
                        egui::RichText::new(format!("{}  {} lines", ph::LIST_NUMBERS, lines))
                            .color(theme.fg_dim)
                            .size(11.0),
                    );
                    if self.show_find && !self.find_query.is_empty() {
                        ui.add(egui::Separator::default().vertical());
                        let ms = if match_count == 0 {
                            "no matches".to_string()
                        } else {
                            format!("{}/{}", match_idx + 1, match_count)
                        };
                        ui.label(
                            egui::RichText::new(format!("{}  {}", ph::MAGNIFYING_GLASS, ms))
                                .color(if match_count == 0 {
                                    theme.accent_paw
                                } else {
                                    theme.accent_eye
                                })
                                .size(11.0),
                        );
                    }
                    ui.add(egui::Separator::default().vertical());
                    ui.label(
                        egui::RichText::new(&self.status)
                            .color(theme.fg_dim)
                            .size(11.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new("kittywrite 0.2.1")
                                .color(theme.accent_eye)
                                .size(11.0),
                        );
                    });
                });
            });

        if self.show_find {
            let mut close_find = false;
            egui::TopBottomPanel::bottom("find_bar")
                .frame(
                    Frame::none()
                        .fill(theme.bg_panel)
                        .inner_margin(Margin::symmetric(8, 5)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let close_r = ui.add(
                            egui::Label::new(
                                egui::RichText::new(ph::X).color(theme.fg_dim).size(14.0),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if close_r.clicked() {
                            close_find = true;
                        }
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(ph::MAGNIFYING_GLASS)
                                .color(theme.fg_dim)
                                .size(14.0),
                        );
                        let fin = ui.add(
                            egui::TextEdit::singleline(&mut self.find_query)
                                .desired_width(200.0)
                                .hint_text("search…"),
                        );
                        if fin.lost_focus() && enter_pressed {
                            act.find_next = true;
                        }

                        let up_r = ui.add(
                            egui::Label::new(
                                egui::RichText::new(ph::ARROW_UP)
                                    .size(14.0)
                                    .color(theme.fg_dim),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if up_r.on_hover_text("previous (shift+F3)").clicked() {
                            act.find_prev = true;
                        }
                        let dn_r = ui.add(
                            egui::Label::new(
                                egui::RichText::new(ph::ARROW_DOWN)
                                    .size(14.0)
                                    .color(theme.fg_dim),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if dn_r.on_hover_text("next (F3)").clicked() {
                            act.find_next = true;
                        }

                        if match_count > 0 {
                            ui.label(
                                egui::RichText::new(format!("{}/{}", match_idx + 1, match_count))
                                    .color(theme.accent_eye)
                                    .size(11.0),
                            );
                        } else if !self.find_query.is_empty() {
                            ui.label(
                                egui::RichText::new("no matches")
                                    .color(theme.accent_paw)
                                    .size(11.0),
                            );
                        }

                        if self.show_replace_bar {
                            ui.add_space(8.0);
                            ui.add(egui::Separator::default().vertical());
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(ph::ARROWS_CLOCKWISE)
                                    .color(theme.fg_dim)
                                    .size(14.0),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.replace_query)
                                    .desired_width(200.0)
                                    .hint_text("replace with…"),
                            );
                            if ui.button("replace").clicked() {
                                act.replace_one = true;
                            }
                            if ui.button("replace all").clicked() {
                                act.replace_all_matches = true;
                            }
                        }
                    });
                });
            if close_find {
                self.show_find = false;
            }
        }

        if self.show_file_tree {
            let mut open_path: Option<std::path::PathBuf> = None;
            egui::SidePanel::left("file_tree")
                .default_width(220.0)
                .min_width(140.0)
                .frame(
                    Frame::none()
                        .fill(theme.bg_panel)
                        .inner_margin(Margin::symmetric(4, 4)),
                )
                .show(ctx, |ui| {
                    open_path = self.file_tree.show(ui, &theme);
                });
            if let Some(path) = open_path {
                self.open_file_path(path);
            }
        }

        if self.show_quick_open {
            let mut close_quick_open = false;
            let mut open_file: Option<String> = None;
            egui::Window::new("quick open")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, -50.0])
                .fixed_size([400.0, 300.0])
                .frame(
                    egui::Frame::none()
                        .fill(theme.bg_panel)
                        .corner_radius(egui::CornerRadius::same(6))
                        .inner_margin(egui::Margin::same(8)),
                )
                .show(ctx, |ui| {
                    let input = ui.add(
                        egui::TextEdit::singleline(&mut self.quick_open_query)
                            .hint_text("type to search...")
                            .desired_width(f32::INFINITY),
                    );
                    input.request_focus();

                    ui.separator();

                    let query = self.quick_open_query.to_lowercase();
                    let matches: Vec<_> = self
                        .recent_files
                        .iter()
                        .filter(|f| {
                            query.is_empty()
                                || f.to_lowercase().contains(&query)
                                || f.rsplit('\\')
                                    .next()
                                    .unwrap_or(f)
                                    .to_lowercase()
                                    .contains(&query)
                                || f.rsplit('/')
                                    .next()
                                    .unwrap_or(f)
                                    .to_lowercase()
                                    .contains(&query)
                        })
                        .take(10)
                        .cloned()
                        .collect();

                    if matches.is_empty() {
                        ui.label(
                            egui::RichText::new("no recent files")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                    } else {
                        for (i, path) in matches.iter().enumerate() {
                            let name = path.rsplit(&['\\', '/'][..]).next().unwrap_or(path);
                            let is_selected = i == self.quick_open_selected;
                            let bg = if is_selected {
                                theme.bg_tab_active
                            } else {
                                theme.bg_void
                            };

                            let response = egui::Frame::none()
                                .fill(bg)
                                .inner_margin(Margin::symmetric(8, 4))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(ph::FILE_CODE)
                                                .color(theme.fg_dim)
                                                .size(12.0),
                                        );
                                        ui.add_space(4.0);
                                        ui.label(
                                            egui::RichText::new(name)
                                                .color(if is_selected {
                                                    theme.accent_eye
                                                } else {
                                                    theme.fg_main
                                                })
                                                .size(12.0),
                                        );
                                    });
                                })
                                .response;

                            if response.clicked() {
                                open_file = Some(path.clone());
                                close_quick_open = true;
                            }
                        }

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("enter to open  esc to close")
                                    .color(theme.fg_dim)
                                    .size(10.0),
                            );
                        });

                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) && !matches.is_empty() {
                            let idx = self.quick_open_selected.min(matches.len() - 1);
                            open_file = Some(matches[idx].clone());
                            close_quick_open = true;
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                            self.quick_open_selected =
                                (self.quick_open_selected + 1).min(matches.len() - 1);
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                            self.quick_open_selected = self.quick_open_selected.saturating_sub(1);
                        }
                    }

                    if input.lost_focus() && !ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        close_quick_open = true;
                    }
                });

            if let Some(path) = open_file {
                self.open_file_path(std::path::PathBuf::from(path));
            }
            if close_quick_open {
                self.show_quick_open = false;
            }
        }

        // handle tab switch BEFORE extracting content
        if let Some(i) = act.switch_tab {
            self.active_tab = i;
            self.pending_indent = None;
            ctx.request_repaint();
        }

        let has_tabs = !self.tabs.is_empty();
        let active = self.active_tab;

        // update git diff if file changed
        if has_tabs {
            let current_path = self.tabs[active].path.clone();
            if current_path != self.git_diff_file {
                self.git_diff_file = current_path.clone();
                if let Some(p) = current_path {
                    self.git_diff_lines = compute_git_diff(&p);
                }
            }
        }

        let (mut content, language, mut line_count) = if has_tabs {
            let c = std::mem::take(&mut self.tabs[active].content);
            let n = self.tabs[active].line_count();
            let l = self.tabs[active].language.clone();
            (c, l, n)
        } else {
            (String::new(), String::new(), 1)
        };

        let mut indent_applied = false;
        if has_tabs {
            if let Some((byte_pos, indent_str, cursor_before)) = self.pending_indent.take() {
                if byte_pos <= content.len() {
                    let char_count = indent_str.chars().count();
                    content.insert_str(byte_pos, &indent_str);
                    line_count += indent_str.chars().filter(|&c| c == '\n').count();
                    indent_applied = true;
                    let new_idx = cursor_before + char_count;
                    self.pending_cursor = Some((new_idx, None));
                }
            }
        }

        // compute find matches
        if self.show_find && !self.find_query.is_empty() {
            let key = (self.find_query.clone(), content.len());
            if key != self.find_cache_key {
                self.find_matches = compute_matches(&content, &self.find_query);
                self.find_match_idx = self
                    .find_match_idx
                    .min(self.find_matches.len().saturating_sub(1));
                self.find_cache_key = key;
            }
        } else {
            self.find_matches.clear();
        }

        if act.find_next {
            self.navigate_match(true);
        }
        if act.find_prev {
            self.navigate_match(false);
        }

        let find_query_snap = self.find_query.clone();
        let find_matches_snap = self.find_matches.clone();
        let find_idx_snap = self.find_match_idx;
        let content_len_before = content.len();

        let editing_fast = self.last_edit_instant.elapsed() < Duration::from_millis(HL_DEBOUNCE_MS);
        let use_plain = editing_fast;

        let mut editor_changed = false;
        let mut cursor_char: Option<usize> = None;
        let mut editor_id: Option<egui::Id> = None;

        let hl = Arc::clone(&self.hl);
        let ln_color = theme.fg_gutter;
        let content_gen = self.content_generation;

        egui::CentralPanel::default()
            .frame(Frame::none().fill(theme.bg_editor))
            .show(ctx, |ui| {
                if !has_tabs {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new("open a file  (ctrl+o)  or  new tab  (ctrl+n)")
                                .color(theme.fg_gutter)
                                .size(16.0),
                        );
                    });
                    return;
                }

                let mut layouter = {
                    let hl = Arc::clone(&hl);
                    let language = language.clone();
                    let fq = find_query_snap.clone();
                    let fm = find_matches_snap.clone();
                    move |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
                        let text = text.as_str();
                        let mut job = if use_plain {
                            highlighter::plain_highlight(text, font_size, line_height)
                        } else {
                            hl.highlight(text, &language, font_size, line_height, content_gen)
                        };
                        if !fq.is_empty() && !fm.is_empty() {
                            highlighter::apply_match_highlights(
                                &mut job,
                                &fm,
                                find_idx_snap,
                                fq.len(),
                            );
                        }
                        ui.fonts_mut(|f| f.layout_job(job))
                    }
                };

                egui::ScrollArea::both()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                            if show_ln {
                                // line numbers + git diff gutter combined
                                let total_lines = line_count + 2;
                                let mut ln_text = String::with_capacity(total_lines * 6);
                                for n in 1..=total_lines {
                                    if self.git_diff_lines.contains(&n) {
                                        ln_text.push_str(&format!("{:>4} │\n", n));
                                    } else {
                                        ln_text.push_str(&format!("{:>4}  \n", n));
                                    }
                                }
                                let ln_font = egui::FontId::monospace(font_size);
                                let (ln_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(48.0, total_lines as f32 * font_size * line_height),
                                    egui::Sense::hover(),
                                );
                                let painter = ui.painter();

                                // paint background
                                painter.rect_filled(ln_rect, 0.0, theme.bg_void);

                                // paint line numbers
                                painter.text(
                                    ln_rect.left_top() + egui::vec2(4.0, 0.0),
                                    egui::Align2::LEFT_TOP,
                                    &ln_text,
                                    ln_font.clone(),
                                    ln_color,
                                );

                                // paint green gutter bar for diff lines
                                let diff_color = egui::Color32::from_rgb(137, 180, 90);
                                let line_h = font_size * line_height;
                                for n in 1..=total_lines {
                                    if self.git_diff_lines.contains(&n) {
                                        let y = ln_rect.top() + (n - 1) as f32 * line_h;
                                        painter.rect_filled(
                                            egui::Rect::from_min_size(
                                                egui::pos2(ln_rect.right() - 6.0, y),
                                                egui::vec2(4.0, line_h),
                                            ),
                                            0.0,
                                            diff_color,
                                        );
                                    }
                                }

                                ui.add(egui::Separator::default().vertical());
                            }

                            let out = egui::TextEdit::multiline(&mut content)
                                .desired_width(f32::INFINITY)
                                .desired_rows(40)
                                .font(egui::FontId::monospace(font_size))
                                .layouter(&mut layouter)
                                .show(ui);

                            editor_changed = out.response.changed();
                            editor_id = Some(out.response.id);
                            cursor_char = out.cursor_range.as_ref().map(|cr| cr.primary.index);
                        });
                    });
            });

        if let (Some((primary_idx, secondary_idx)), Some(id)) =
            (self.pending_cursor.take(), editor_id)
        {
            let primary = egui::text::CCursor {
                index: primary_idx,
                prefer_next_row: false,
            };
            let secondary = egui::text::CCursor {
                index: secondary_idx.unwrap_or(primary_idx),
                prefer_next_row: false,
            };
            let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(primary, secondary)));
            state.store(ctx, id);
            ctx.request_repaint();
        }

        if has_tabs && editor_changed {
            if let Some(cursor) = cursor_char {
                if self.lua.config.auto_pair
                    && content.len() == content_len_before + 1
                    && cursor > 0
                {
                    let typed = content.chars().nth(cursor - 1);
                    let closer = typed.and_then(|c| match c {
                        '(' => Some(')'),
                        '[' => Some(']'),
                        '{' => Some('}'),
                        '"' => Some('"'),
                        '\'' => Some('\''),
                        '`' => Some('`'),
                        _ => None,
                    });
                    if let Some(cl) = closer {
                        let next = content.chars().nth(cursor);
                        let safe = matches!(
                            next,
                            None | Some('\n')
                                | Some('\r')
                                | Some('\t')
                                | Some(' ')
                                | Some(')')
                                | Some(']')
                                | Some('}')
                                | Some('"')
                                | Some('\'')
                                | Some('`')
                        );
                        if safe {
                            let byte_pos = char_to_byte(&content, cursor);
                            content.insert(byte_pos, cl);
                        }
                    }
                }

                if self.lua.config.auto_indent && enter_pressed {
                    let byte_cursor = char_to_byte(&content, cursor);
                    let text_before = &content[..byte_cursor];
                    let prev_line = if let Some(nl) =
                        text_before[..text_before.len().saturating_sub(1)].rfind('\n')
                    {
                        &text_before[nl + 1..text_before.len().saturating_sub(1)]
                    } else {
                        &text_before[..text_before.len().saturating_sub(1)]
                    };
                    let indent: String = prev_line
                        .chars()
                        .take_while(|&c| c == ' ' || c == '\t')
                        .collect();
                    let extra = if prev_line
                        .trim_end()
                        .ends_with(|c| matches!(c, '{' | '(' | '['))
                    {
                        "\t".to_string()
                    } else {
                        String::new()
                    };
                    let full_indent = indent + &extra;
                    if !full_indent.is_empty() {
                        self.pending_indent = Some((byte_cursor, full_indent, cursor));
                        ctx.request_repaint();
                    }
                }
            }
        }

        // replace actions
        if act.replace_one && has_tabs {
            if self.replace_current(&mut content) {
                self.find_cache_key = (String::new(), 0);
                self.navigate_match(true);
            }
        }
        if act.replace_all_matches && has_tabs {
            let count = self.replace_all(&mut content);
            if count > 0 {
                self.find_cache_key = (String::new(), 0);
                self.status = format!(
                    "replaced {} occurrence{}",
                    count,
                    if count == 1 { "" } else { "s" }
                );
            }
        }

        if has_tabs {
            self.tabs[active].content = content;
            if editor_changed || indent_applied {
                self.tabs[active].modified = true;
                self.tabs[active].update_line_count();
                self.find_cache_key = (String::new(), 0);
                self.content_generation = self.content_generation.wrapping_add(1);
            }
            if editor_changed {
                self.last_edit_instant = Instant::now();
            }
        }

        if self.show_about {
            egui::Window::new("about kittywrite")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(
                    Frame::window(&ctx.style())
                        .fill(theme.bg_panel)
                        .corner_radius(8),
                )
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("=^.^=")
                                .color(theme.accent_paw)
                                .size(36.0),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("kittywrite")
                                .color(theme.accent_eye)
                                .size(22.0)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("v0.2.1  ·  lightweight IDE")
                                .color(theme.fg_dim)
                                .size(13.0),
                        );
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);
                        for (k, v) in [
                            ("ui", "egui 0.33 / eframe"),
                            ("highlighting", "syntect 5.2"),
                            ("scripting", "mlua (lua 5.4)"),
                            ("dialogs", "rfd 0.17"),
                            ("icons", "phosphor 0.11"),
                        ] {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("{:>14}  ", k))
                                        .color(theme.fg_dim)
                                        .size(12.0),
                                );
                                ui.label(egui::RichText::new(v).color(theme.accent_fur).size(12.0));
                            });
                        }
                        ui.add_space(12.0);
                        if ui.button(format!("  {}  close  ", ph::X)).clicked() {
                            self.show_about = false;
                        }
                        ui.add_space(8.0);
                    });
                });
        }

        if self.show_settings {
            let mut open = true;
            egui::Window::new(format!("{} settings", ph::GEAR))
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .frame(
                    egui::Frame::none()
                        .fill(theme.bg_panel)
                        .corner_radius(egui::CornerRadius::same(6))
                        .inner_margin(egui::Margin::same(12)),
                )
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("appearance")
                            .color(theme.accent_eye)
                            .size(14.0)
                            .strong(),
                    );
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("theme").color(theme.fg_main).size(12.0));
                        egui::ComboBox::from_id_salt("theme_combo")
                            .selected_text(&self.lua.config.theme)
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for t in CatTheme::list() {
                                    let response =
                                        ui.selectable_label(self.lua.config.theme == *t, *t);
                                    if response.clicked() {
                                        self.lua.config.theme = t.to_string();
                                    }
                                }
                            });
                    });
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new("editor")
                            .color(theme.accent_eye)
                            .size(14.0)
                            .strong(),
                    );
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("font size")
                                .color(theme.fg_main)
                                .size(12.0),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.lua.config.font_size)
                                .range(8.0..=48.0)
                                .speed(0.5),
                        );
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("tab width")
                                .color(theme.fg_main)
                                .size(12.0),
                        );
                        ui.add(egui::DragValue::new(&mut self.lua.config.tab_width).range(1..=16));
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("line height")
                                .color(theme.fg_main)
                                .size(12.0),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.lua.config.line_height, 1.0..=2.5)
                                .show_value(false),
                        );
                    });
                    ui.add_space(8.0);
                    ui.checkbox(&mut self.lua.config.show_line_numbers, "line numbers");
                    ui.checkbox(&mut self.lua.config.word_wrap, "word wrap");
                    ui.checkbox(&mut self.lua.config.auto_indent, "auto indent");
                    ui.checkbox(&mut self.lua.config.auto_pair, "auto pair brackets");

                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new("available themes:")
                            .color(theme.fg_dim)
                            .size(11.0),
                    );
                    ui.label(
                        egui::RichText::new("kittywrite, mocha, frappe, macchiato, latte")
                            .color(theme.fg_dim)
                            .size(11.0),
                    );
                    ui.add_space(8.0);

                    if ui.button("  close  ").clicked() {
                        self.show_settings = false;
                    }
                });
            self.show_settings = open;
        }

        if self.show_lua_console {
            let mut open = true;
            egui::Window::new(format!("{} lua console", ph::TERMINAL_WINDOW))
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 320.0])
                .frame(
                    Frame::window(&ctx.style())
                        .fill(theme.bg_panel)
                        .corner_radius(6),
                )
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("run lua — access config via the kittywrite table")
                            .color(theme.fg_dim)
                            .size(11.0),
                    );
                    ui.add_space(4.0);
                    let mut out_display = self.lua_output.clone();
                    egui::ScrollArea::vertical()
                        .id_salt("lua_out_scroll")
                        .max_height(180.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut out_display)
                                    .code_editor()
                                    .desired_width(f32::INFINITY),
                            );
                        });
                    ui.separator();
                    ui.horizontal(|ui| {
                        let inp = ui.add(
                            egui::TextEdit::singleline(&mut self.lua_input)
                                .code_editor()
                                .desired_width(ui.available_width() - 56.0)
                                .hint_text("kittywrite.font_size = 16"),
                        );
                        let exec = (inp.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                            || ui.button("run").clicked();
                        if exec {
                            let code = self.lua_input.drain(..).collect::<String>();
                            match self.lua.exec(&code) {
                                Ok(out) => {
                                    self.lua_output.push_str(&format!("> {}\n", code));
                                    if !out.is_empty() {
                                        self.lua_output.push_str(&format!("{}\n", out));
                                    }
                                }
                                Err(e) => {
                                    self.lua_output
                                        .push_str(&format!("> {}\nerror: {}\n", code, e));
                                }
                            }
                        }
                    });
                });
            if !open {
                self.show_lua_console = false;
            }
        }

        if act.new_tab {
            self.new_tab();
        }
        if act.open_file {
            self.open_file();
        }
        if act.open_folder {
            self.file_tree.open_folder();
            self.show_file_tree = true;
        }
        if act.save {
            self.save_current();
        }
        if act.save_as {
            self.save_as();
        }
        if let Some(i) = act.close_tab {
            self.close_tab(i);
        }
        if act.toggle_find {
            self.show_find = !self.show_find;
            if !self.show_find {
                self.show_replace_bar = false;
            }
        }
        if act.toggle_replace {
            self.show_find = true;
            self.show_replace_bar = !self.show_replace_bar;
        }
        if act.toggle_file_tree {
            self.show_file_tree = !self.show_file_tree;
        }
        if act.toggle_settings {
            self.show_settings = !self.show_settings;
        }
        if act.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if act.toggle_quick_open {
            self.show_quick_open = !self.show_quick_open;
            if self.show_quick_open {
                self.quick_open_query.clear();
                self.quick_open_selected = 0;
            }
        }
    }
}

fn recent_files_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(dir.join("recent.txt"))
}

fn load_recent_files() -> Vec<String> {
    let path = match recent_files_path() {
        Some(p) => p,
        None => return Vec::new(),
    };
    std::fs::read_to_string(&path)
        .map(|s| s.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default()
}

fn save_recent_files(files: &[String]) {
    let path = match recent_files_path() {
        Some(p) => p,
        None => return,
    };
    let content = files.join("\n");
    let _ = std::fs::write(path, content);
}

fn compute_git_diff(path: &std::path::Path) -> std::collections::HashSet<usize> {
    let mut lines = std::collections::HashSet::new();

    let dir = match path.parent() {
        Some(d) => d,
        None => return lines,
    };

    let filename = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return lines,
    };

    let output = std::process::Command::new("git")
        .args(["diff", "--unified=0", "--", filename])
        .current_dir(dir)
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return lines,
    };

    if !output.status.success() {
        return lines;
    }

    let diff = String::from_utf8_lossy(&output.stdout);

    let mut current_line: usize = 1;
    for line in diff.lines() {
        if line.starts_with("@@") {
            // parse @@ -old_start,old_count +new_start,new_count @@
            // we want new_start for the new file line numbers
            if let Some(after) = line.strip_prefix("@@ -") {
                if let Some(plus_part) = after.split(" +").nth(1) {
                    if let Some(start) = plus_part
                        .split(',')
                        .next()
                        .and_then(|s| s.parse::<usize>().ok())
                    {
                        current_line = start;
                    }
                }
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            // added line in new file
            lines.insert(current_line);
            current_line += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            // deleted line - don't increment, it doesn't exist in new file
            lines.insert(current_line);
        } else if line.starts_with(' ') || line.is_empty() {
            // context line - exists in both files
            current_line += 1;
        }
    }

    lines
}
