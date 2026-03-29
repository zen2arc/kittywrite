use std::collections::HashSet;
use std::path::{Path, PathBuf};

use egui_phosphor::regular as ph;

use crate::theme::CatTheme;

pub struct FileTree {
    pub root: Option<PathBuf>,
    expanded: HashSet<PathBuf>,
}

impl Default for FileTree {
    fn default() -> Self {
        Self {
            root: None,
            expanded: HashSet::new(),
        }
    }
}

impl FileTree {
    pub fn open_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.expanded.insert(path.clone());
            self.root = Some(path);
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, theme: &CatTheme) -> Option<PathBuf> {
        let mut clicked: Option<PathBuf> = None;

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("EXPLORER")
                    .color(theme.fg_dim)
                    .size(10.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let open_btn = ui.add(
                    egui::Label::new(
                        egui::RichText::new(ph::FOLDER_OPEN)
                            .color(theme.fg_dim)
                            .size(16.0),
                    )
                    .sense(egui::Sense::click()),
                );
                if open_btn.on_hover_text("open folder").clicked() {
                    self.open_folder();
                }
            });
        });
        ui.add(egui::Separator::default().horizontal());

        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;

                if let Some(root) = self.root.clone() {
                    let name = root
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("project")
                        .to_string();
                    let expanded = self.expanded.contains(&root);
                    let caret = if expanded {
                        ph::CARET_DOWN
                    } else {
                        ph::CARET_RIGHT
                    };

                    let r = ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("{} {} {}", ph::FOLDER_OPEN, caret, name))
                                .color(theme.accent_eye)
                                .size(12.0)
                                .strong(),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if r.clicked() {
                        if expanded {
                            self.expanded.remove(&root);
                        } else {
                            self.expanded.insert(root.clone());
                        }
                    }

                    if self.expanded.contains(&root) {
                        ui.indent("root_tree", |ui| {
                            self.show_dir(ui, &root, theme, &mut clicked);
                        });
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new(ph::FOLDER)
                                .color(theme.fg_gutter)
                                .size(32.0),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("no folder open")
                                .color(theme.fg_dim)
                                .size(11.0),
                        );
                        ui.add_space(8.0);
                        if ui
                            .button(format!("{}  open folder…", ph::FOLDER_OPEN))
                            .clicked()
                        {
                            self.open_folder();
                        }
                    });
                }
            });

        clicked
    }

    fn show_dir(
        &mut self,
        ui: &mut egui::Ui,
        dir: &Path,
        theme: &CatTheme,
        clicked: &mut Option<PathBuf>,
    ) {
        let entries = match std::fs::read_dir(dir) {
            Ok(rd) => {
                let mut v: Vec<(PathBuf, bool, String)> = rd
                    .filter_map(|e| e.ok())
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.starts_with('.') {
                            return None;
                        }
                        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        Some((e.path(), is_dir, name))
                    })
                    .collect();
                v.sort_by(|a, b| match (a.1, b.1) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.2.to_lowercase().cmp(&b.2.to_lowercase()),
                });
                v
            }
            Err(_) => return,
        };

        for (path, is_dir, name) in entries {
            if is_dir {
                let exp = self.expanded.contains(&path);
                let caret = if exp { ph::CARET_DOWN } else { ph::CARET_RIGHT };
                let folder_icon = if exp { ph::FOLDER_OPEN } else { ph::FOLDER };
                let r = ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!("{} {} {}", folder_icon, caret, name))
                            .color(theme.fg_dim)
                            .size(12.0),
                    )
                    .sense(egui::Sense::click()),
                );
                if r.clicked() {
                    if exp {
                        self.expanded.remove(&path);
                    } else {
                        self.expanded.insert(path.clone());
                    }
                }
                if self.expanded.contains(&path) {
                    let id = egui::Id::new(path.display().to_string());
                    let path2 = path.clone();
                    ui.indent(id, |ui| {
                        self.show_dir(ui, &path2, theme, clicked);
                    });
                }
            } else {
                let file_icon = file_icon_for(&name);
                let r = ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!("{}  {}", file_icon, name))
                            .color(theme.fg_main)
                            .size(12.0),
                    )
                    .sense(egui::Sense::click()),
                );
                if r.clicked() {
                    *clicked = Some(path);
                }
            }
        }
    }
}

fn file_icon_for(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "h" | "cpp" | "cc" | "cxx" | "hpp"
        | "cs" | "go" | "java" | "kt" | "swift" | "rb" | "php" | "lua" | "sh" | "bash" | "ps1"
        | "zig" | "nim" | "v" | "dart" | "ex" | "exs" | "hs" | "ml" | "scala" | "clj" | "r" => {
            ph::FILE_CODE
        }
        "md" | "txt" | "rst" | "adoc" => ph::FILE_TEXT,
        "json" | "yaml" | "yml" | "toml" | "xml" | "html" | "css" | "scss" | "sql" | "graphql"
        | "proto" => ph::FILE_CODE,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" => ph::FILE_IMAGE,
        "zip" | "tar" | "gz" | "xz" | "bz2" | "7z" | "rar" => ph::FILE_ZIP,
        _ => ph::FILE,
    }
}
