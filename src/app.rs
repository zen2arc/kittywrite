use std::sync::Arc;

use egui::{Frame, Margin, Rounding};

use crate::editor::{detect_language, EditorTab};
use crate::highlighter::Highlighter;
use crate::lua_engine::LuaEngine;
use crate::theme::CatTheme;

#[derive(Default)]
struct FrameActions {
    new_tab: bool,
    open_file: bool,
    save: bool,
    save_as: bool,
    close_tab: Option<usize>,
    switch_tab: Option<usize>,
    quit: bool,
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
}

impl KittyWriteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = CatTheme::default();
        theme.apply(&cc.egui_ctx);

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
        }
    }

    // tab management

    fn new_tab(&mut self) {
        self.tabs.push(EditorTab::new_empty());
        self.active_tab = self.tabs.len() - 1;
    }

    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            match EditorTab::from_path(path) {
                Ok(tab) => {
                    self.status = format!("opened {}", tab.title);
                    self.tabs.push(tab);
                    self.active_tab = self.tabs.len() - 1;
                }
                Err(e) => {
                    self.status = format!("open failed: {}", e);
                }
            }
        }
    }

    fn save_current(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let idx = self.active_tab;

        // no path yet, do save-as flow
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
                return; // user cancelled
            }
        }

        match self.tabs[idx].save() {
            Ok(_) => {
                let n = self.tabs[idx].title.clone();
                self.status = format!("saved {}", n);
            }
            Err(e) => {
                self.status = format!("save failed: {}", e);
            }
        }
    }

    fn save_as(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        self.tabs[self.active_tab].path = None; // force dialog
        self.save_current();
    }

    fn close_tab(&mut self, idx: usize) {
        if self.tabs.len() == 1 {
            // keep at least one tab, just reset it
            self.tabs[0] = EditorTab::new_empty();
            self.active_tab = 0;
            return;
        }
        self.tabs.remove(idx);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }
}

impl eframe::App for KittyWriteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut act = FrameActions::default();

        // keyboard shortcuts
        ctx.input(|i| {
            let ctrl = i.modifiers.ctrl;
            if ctrl && i.key_pressed(egui::Key::N) {
                act.new_tab = true;
            }
            if ctrl && i.key_pressed(egui::Key::O) {
                act.open_file = true;
            }
            if ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::S) {
                act.save = true;
            }
            if ctrl && i.modifiers.shift && i.key_pressed(egui::Key::S) {
                act.save_as = true;
            }
            if ctrl && i.key_pressed(egui::Key::W) {
                act.close_tab = Some(self.active_tab);
            }
        });

        let theme = self.theme;
        let font_size = self.lua.config.font_size;
        let show_ln = self.lua.config.show_line_numbers;

        // menu bar
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
                        if ui.button("new tab         ctrl+n").clicked() {
                            act.new_tab = true;
                            ui.close_menu();
                        }
                        if ui.button("open file...    ctrl+o").clicked() {
                            act.open_file = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("save            ctrl+s").clicked() {
                            act.save = true;
                            ui.close_menu();
                        }
                        if ui.button("save as...  ctrl+shift+s").clicked() {
                            act.save_as = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("close tab       ctrl+w").clicked() {
                            act.close_tab = Some(self.active_tab);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("quit").clicked() {
                            act.quit = true;
                        }
                    });

                    ui.menu_button("edit", |ui| {
                        ui.label(
                            egui::RichText::new("undo           ctrl+z")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new("redo           ctrl+y")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("cut            ctrl+x")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new("copy           ctrl+c")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new("paste          ctrl+v")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("select all     ctrl+a")
                                .color(theme.fg_dim)
                                .size(12.0),
                        );
                    });

                    ui.menu_button("view", |ui| {
                        ui.checkbox(&mut self.lua.config.show_line_numbers, "line numbers");
                        ui.checkbox(&mut self.lua.config.word_wrap, "word wrap");
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
                    });

                    ui.menu_button("tools", |ui| {
                        if ui.button("lua console").clicked() {
                            self.show_lua_console = !self.show_lua_console;
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("help", |ui| {
                        if ui.button("about").clicked() {
                            self.show_about = true;
                            ui.close_menu();
                        }
                    });
                });
            });

        // tab bar
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
                        let label_col = if is_active {
                            theme.accent_eye
                        } else {
                            theme.fg_dim
                        };
                        let tab_title = self.tabs[i].tab_label();

                        let tab_frame = Frame::none()
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
                            });

                        tab_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;

                                let title_resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(&tab_title).color(label_col).size(13.0),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if title_resp.clicked() {
                                    act.switch_tab = Some(i);
                                }

                                let x_resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new("\u{00d7}")
                                            .color(if is_active {
                                                theme.fg_dim
                                            } else {
                                                theme.fg_gutter
                                            })
                                            .size(15.0),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if x_resp.clicked() {
                                    act.close_tab = Some(i);
                                }
                                if x_resp.hovered() {
                                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            });
                        });
                    }

                    let new_resp = ui.add(
                        egui::Label::new(
                            egui::RichText::new("  +  ").color(theme.fg_dim).size(16.0),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if new_resp.clicked() {
                        act.new_tab = true;
                    }
                    if new_resp.hovered() {
                        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                });
            });

        // status bar
        {
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
            let status = self.status.clone();

            egui::TopBottomPanel::bottom("status_bar")
                .frame(
                    Frame::none()
                        .fill(theme.bg_void)
                        .inner_margin(Margin::symmetric(10.0, 3.0)),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(&lang)
                                .color(theme.accent_fur)
                                .size(11.0),
                        );
                        ui.add(egui::Separator::default().vertical());
                        ui.label(
                            egui::RichText::new(format!("{} lines", lines))
                                .color(theme.fg_dim)
                                .size(11.0),
                        );
                        ui.add(egui::Separator::default().vertical());
                        ui.label(egui::RichText::new(&status).color(theme.fg_dim).size(11.0));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new("kittywrite")
                                    .color(theme.accent_eye)
                                    .size(11.0),
                            );
                        });
                    });
                });
        }

        // main editor panel
        let has_tabs = !self.tabs.is_empty();
        let active = self.active_tab;

        let (mut content, language, line_count) = if has_tabs {
            let c = std::mem::take(&mut self.tabs[active].content);
            let n = self.tabs[active].line_count();
            let l = self.tabs[active].language.clone();
            (c, l, n)
        } else {
            (String::new(), String::new(), 1)
        };

        let mut editor_changed = false;
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
                    move |ui: &egui::Ui, text: &str, _wrap: f32| {
                        let job = hl.highlight(text, &language, font_size);
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
                                .desired_width(f32::INFINITY)
                                .desired_rows(40)
                                .code_editor()
                                .layouter(&mut layouter)
                                .show(ui);

                            if out.response.changed() {
                                editor_changed = true;
                            }
                        });
                    });
            });

        if has_tabs {
            self.tabs[active].content = content;
            if editor_changed {
                self.tabs[active].modified = true;
                self.tabs[active].update_line_count();
            }
        }

        // about window
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
                            egui::RichText::new("lightweight open-source text editor")
                                .color(theme.fg_dim)
                                .size(13.0),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new("license: MIT")
                                .color(theme.fg_dim)
                                .size(8.0),
                        );
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        let stack = [
                            ("editor", "kittywrite 0.1.0"),
                            ("ui", "egui 0.27 / eframe"),
                            ("highlighting", "syntect 5.2"),
                            ("scripting", "mlua (lua 5.4)"),
                            ("dialogs", "rfd 0.14"),
                        ];

                        for (k, v) in stack {
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
                        if ui.button("  close  ").clicked() {
                            self.show_about = false;
                        }
                        ui.add_space(8.0);
                    });
                });
        }

        // lua console
        if self.show_lua_console {
            let mut open = true;
            egui::Window::new("lua console")
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
                        egui::RichText::new(
                            "run lua snippets -- access editor config via the kittywrite table",
                        )
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

                    // input row
                    ui.horizontal(|ui| {
                        let inp = ui.add(
                            egui::TextEdit::singleline(&mut self.lua_input)
                                .code_editor()
                                .desired_width(ui.available_width() - 56.0)
                                .hint_text("kittywrite.font_size = 16"),
                        );

                        let enter =
                            inp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let run = ui.button("run").clicked();

                        if enter || run {
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
        }
        if act.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
