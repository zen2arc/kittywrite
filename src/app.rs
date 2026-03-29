use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{Frame, Margin, Rounding};
use egui_phosphor::regular as ph;

use crate::editor::{detect_language, EditorTab};
use crate::filetree::FileTree;
use crate::highlighter::{self, Highlighter};
use crate::lua_engine::LuaEngine;
use crate::theme::CatTheme;

const HL_DEBOUNCE_BYTES: usize = 200_000;
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
}

impl KittyWriteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = CatTheme::default();
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
            lua: LuaEngine::new(),
            status: "ready".to_string(),
            show_about: false,
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
                    self.tabs.push(tab);
                    self.active_tab = self.tabs.len() - 1;
                    self.pending_indent = None;
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
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
                self.pending_indent = None;
            }
            Err(e) => self.status = format!("open failed: {}", e),
        }
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
            if i.key_pressed(egui::Key::F3) && !shift {
                act.find_next = true;
            }
            if i.key_pressed(egui::Key::F3) && shift {
                act.find_prev = true;
            }
            if i.key_pressed(egui::Key::Escape) && self.show_find {
                act.toggle_find = true;
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
                    .inner_margin(Margin::symmetric(6.0, 4.0)),
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
                left: 4.0,
                right: 4.0,
                top: 4.0,
                bottom: 0.0,
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
                            .rounding(Rounding {
                                nw: 4.0,
                                ne: 4.0,
                                sw: 0.0,
                                se: 0.0,
                            })
                            .inner_margin(Margin {
                                left: 10.0,
                                right: 6.0,
                                top: 4.0,
                                bottom: 4.0,
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
                    .inner_margin(Margin::symmetric(10.0, 3.0)),
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
                            egui::RichText::new("kittywrite 0.2.0")
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
                        .inner_margin(Margin::symmetric(8.0, 5.0)),
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
                        .inner_margin(Margin::symmetric(4.0, 4.0)),
                )
                .show(ctx, |ui| {
                    open_path = self.file_tree.show(ui, &theme);
                });
            if let Some(path) = open_path {
                self.open_file_path(path);
            }
        }

        let has_tabs = !self.tabs.is_empty();
        let active = self.active_tab;

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

        let large_file = content.len() > HL_DEBOUNCE_BYTES;
        let editing_fast = self.last_edit_instant.elapsed() < Duration::from_millis(HL_DEBOUNCE_MS);
        let use_plain = large_file && editing_fast;

        let mut editor_changed = false;
        let mut cursor_char: Option<usize> = None;
        let mut editor_galley: Option<Arc<egui::Galley>> = None;
        let mut editor_id: Option<egui::Id> = None;

        let hl = Arc::clone(&self.hl);
        let ln_color = theme.fg_gutter;

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
                    move |ui: &egui::Ui, text: &str, _wrap: f32| {
                        let mut job = if use_plain {
                            highlighter::plain_highlight(text, font_size, line_height)
                        } else {
                            hl.highlight(text, &language, font_size, line_height)
                        };
                        if !fq.is_empty() && !fm.is_empty() {
                            highlighter::apply_match_highlights(
                                &mut job,
                                &fm,
                                find_idx_snap,
                                fq.len(),
                            );
                        }
                        ui.fonts(|f| f.layout_job(job))
                    }
                };

                egui::ScrollArea::both()
                    .id_source("editor_scroll")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                            if show_ln {
                                ui.vertical(|ui| {
                                    ui.set_min_width(48.0);
                                    ui.set_max_width(48.0);
                                    ui.add_space(2.0);
                                    let mut ln_text = String::with_capacity((line_count + 2) * 5);
                                    for n in 1..=(line_count + 2) {
                                        ln_text.push_str(&format!("{:>4}\n", n));
                                    }
                                    ui.label(
                                        egui::RichText::new(ln_text)
                                            .color(ln_color)
                                            .font(egui::FontId::monospace(font_size)),
                                    );
                                });
                                ui.add(egui::Separator::default().vertical());
                            }

                            let out = egui::TextEdit::multiline(&mut content)
                                .id_source(("editor", active))
                                .desired_width(f32::INFINITY)
                                .desired_rows(40)
                                .code_editor()
                                .layouter(&mut layouter)
                                .show(ui);

                            editor_changed = out.response.changed();
                            editor_id = Some(out.response.id);
                            editor_galley = Some(out.galley.clone());
                            cursor_char =
                                out.cursor_range.as_ref().map(|cr| cr.primary.ccursor.index);
                        });
                    });
            });

        if let (Some(pending), Some(galley), Some(id)) = (
            self.pending_cursor.take(),
            editor_galley.as_ref(),
            editor_id,
        ) {
            let (primary_idx, secondary_idx) = pending;
            let primary = galley.from_ccursor(egui::text::CCursor {
                index: primary_idx,
                prefer_next_row: false,
            });
            let secondary = galley.from_ccursor(egui::text::CCursor {
                index: secondary_idx.unwrap_or(primary_idx),
                prefer_next_row: false,
            });
            if let Some(mut state) = egui::text_edit::TextEditState::load(ctx, id) {
                state
                    .cursor
                    .set_range(Some(egui::text::CursorRange { primary, secondary }));
                state.store(ctx, id);
                ctx.request_repaint();
            }
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
                        " ".repeat(self.lua.config.tab_width)
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
                        .rounding(Rounding::same(8.0)),
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
                            egui::RichText::new("v0.2.0  ·  lightweight IDE")
                                .color(theme.fg_dim)
                                .size(13.0),
                        );
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);
                        for (k, v) in [
                            ("ui", "egui 0.27 / eframe"),
                            ("highlighting", "syntect 5.2"),
                            ("scripting", "mlua (lua 5.4)"),
                            ("dialogs", "rfd 0.14"),
                            ("icons", "phosphor 0.5"),
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

        if self.show_lua_console {
            let mut open = true;
            egui::Window::new(format!("{} lua console", ph::TERMINAL_WINDOW))
                .open(&mut open)
                .resizable(true)
                .default_size([520.0, 320.0])
                .frame(
                    Frame::window(&ctx.style())
                        .fill(theme.bg_panel)
                        .rounding(Rounding::same(6.0)),
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
                        .id_source("lua_out_scroll")
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
        if let Some(i) = act.switch_tab {
            self.active_tab = i;
            self.pending_indent = None;
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
        if act.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
