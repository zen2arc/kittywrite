use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use egui_phosphor::regular as ph;

use crate::theme::CatTheme;

#[derive(Clone, Copy, PartialEq)]
pub enum GitStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
}

pub struct FileTree {
    pub root: Option<PathBuf>,
    expanded: HashSet<PathBuf>,
    rename_target: Option<PathBuf>,
    rename_buf: String,
    pub search_query: String,
    git_status: HashMap<PathBuf, GitStatus>,
    pub show_hidden: bool,
    collapsed_dirs: HashSet<PathBuf>,
}

impl Default for FileTree {
    fn default() -> Self {
        Self {
            root: None,
            expanded: HashSet::new(),
            rename_target: None,
            rename_buf: String::new(),
            search_query: String::new(),
            git_status: HashMap::new(),
            show_hidden: false,
            collapsed_dirs: HashSet::new(),
        }
    }
}

impl FileTree {
    pub fn open_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            self.expanded.insert(path.clone());
            self.root = Some(path);
            self.refresh_git_status();
        }
    }

    pub fn refresh_git_status(&mut self) {
        self.git_status.clear();
        if let Some(root) = self.root.clone() {
            self.scan_git_status(&root, &root);
        }
    }

    fn scan_git_status(&mut self, root: &Path, dir: &Path) {
        let output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(dir)
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.len() < 3 {
                    continue;
                }
                let status_code = &line[0..2];
                let path_str = line[3..].trim_start_matches('"').trim_end_matches('"');
                let full_path = dir.join(path_str);

                let status = match status_code {
                    " M" | "MM" => GitStatus::Modified,
                    "M " => GitStatus::Added,
                    " D" => GitStatus::Deleted,
                    "D " => GitStatus::Deleted,
                    "A " => GitStatus::Added,
                    "R " => GitStatus::Renamed,
                    "??" => GitStatus::Untracked,
                    _ => continue,
                };
                self.git_status.insert(full_path, status);
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, theme: &CatTheme) -> Option<PathBuf> {
        let mut clicked: Option<PathBuf> = None;

        // header
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("EXPLORER")
                    .color(theme.fg_dim)
                    .size(10.0)
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let refresh_btn = ui.add(
                    egui::Label::new(
                        egui::RichText::new(ph::ARROW_CLOCKWISE)
                            .color(theme.fg_dim)
                            .size(14.0),
                    )
                    .sense(egui::Sense::click()),
                );
                if refresh_btn.on_hover_text("refresh").clicked() {
                    self.refresh_git_status();
                }

                let new_file_btn = ui.add(
                    egui::Label::new(
                        egui::RichText::new(ph::FILE_PLUS)
                            .color(theme.fg_dim)
                            .size(16.0),
                    )
                    .sense(egui::Sense::click()),
                );
                if new_file_btn.on_hover_text("new file").clicked() {
                    if let Some(root) = &self.root {
                        self.new_file(root.clone());
                    }
                }

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

        // search bar
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_query)
                    .hint_text("search files...")
                    .desired_width(f32::INFINITY)
                    .font(egui::FontId::monospace(11.0)),
            );
            if !self.search_query.is_empty() {
                if ui.small_button("x").clicked() {
                    self.search_query.clear();
                }
            }
        });

        ui.add(egui::Separator::default().horizontal());

        // file list
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;

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
                    r.context_menu(|ui| {
                        if ui.button(format!("{}  new file", ph::FILE_PLUS)).clicked() {
                            self.new_file(root.clone());
                            ui.close_menu();
                        }
                        if ui
                            .button(format!("{}  new folder", ph::FOLDER_PLUS))
                            .clicked()
                        {
                            self.new_folder(root.clone());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .button(format!("{}  refresh git status", ph::ARROW_CLOCKWISE))
                            .clicked()
                        {
                            self.refresh_git_status();
                            ui.close_menu();
                        }
                    });

                    if expanded {
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

    fn matches_search(&self, name: &str) -> bool {
        if self.search_query.is_empty() {
            return true;
        }
        name.to_lowercase()
            .contains(&self.search_query.to_lowercase())
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
                        if !self.show_hidden && name.starts_with('.') {
                            return None;
                        }
                        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                        Some((e.path(), is_dir, name))
                    })
                    .filter(|(_, is_dir, name)| {
                        if self.search_query.is_empty() {
                            return true;
                        }
                        if self.matches_search(name) {
                            return true;
                        }
                        // keep directories if they might contain matching files
                        *is_dir
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

                // git status indicator for directories
                let git_indicator = self.get_dir_git_indicator(&path);
                let git_color = if !git_indicator.is_empty() {
                    theme.accent_eye
                } else {
                    theme.fg_dim
                };

                let r = ui.add(
                    egui::Label::new(
                        egui::RichText::new(format!(
                            "{} {} {}{}",
                            folder_icon, caret, name, git_indicator
                        ))
                        .color(git_color)
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
                r.context_menu(|ui| {
                    if ui.button(format!("{}  new file", ph::FILE_PLUS)).clicked() {
                        self.new_file(path.clone());
                        ui.close_menu();
                    }
                    if ui
                        .button(format!("{}  new folder", ph::FOLDER_PLUS))
                        .clicked()
                    {
                        self.new_folder(path.clone());
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(format!("{}  rename", ph::PENCIL_LINE)).clicked() {
                        self.start_rename(path.clone());
                        ui.close_menu();
                    }
                    if ui.button(format!("{}  delete", ph::TRASH)).clicked() {
                        self.delete_path(path.clone());
                        ui.close_menu();
                    }
                });
                if exp {
                    let id = egui::Id::new(path.display().to_string());
                    let path2 = path.clone();
                    ui.indent(id, |ui| {
                        self.show_dir(ui, &path2, theme, clicked);
                    });
                }
            } else {
                let file_icon = file_icon_for(&name);
                let is_renaming = self.rename_target.as_ref() == Some(&path);

                // git status
                let git_status = self.git_status.get(&path);
                let (status_icon, status_color) = match git_status {
                    Some(GitStatus::Modified) => (" M", theme.accent_eye),
                    Some(GitStatus::Added) => (" A", theme.accent_fur),
                    Some(GitStatus::Deleted) => (" D", theme.accent_paw),
                    Some(GitStatus::Renamed) => (" R", theme.accent_eye),
                    Some(GitStatus::Untracked) => (" ?", theme.fg_dim),
                    None => ("", theme.fg_main),
                };

                if is_renaming {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.rename_buf)
                            .desired_width(150.0)
                            .font(egui::FontId::monospace(12.0)),
                    );
                    if response.lost_focus() {
                        self.finish_rename();
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.finish_rename();
                    }
                } else if self.matches_search(&name) || !self.search_query.is_empty() {
                    let r = ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("{} {}  {}", status_icon, file_icon, name))
                                .color(status_color)
                                .size(12.0),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if r.clicked() {
                        *clicked = Some(path.clone());
                    }
                    r.context_menu(|ui| {
                        if ui.button(format!("{}  open", ph::FILE_TEXT)).clicked() {
                            *clicked = Some(path.clone());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(format!("{}  rename", ph::PENCIL_LINE)).clicked() {
                            self.start_rename(path.clone());
                            ui.close_menu();
                        }
                        if ui.button(format!("{}  delete", ph::TRASH)).clicked() {
                            self.delete_path(path.clone());
                            ui.close_menu();
                        }
                    });
                }
            }
        }
    }

    fn get_dir_git_indicator(&self, path: &Path) -> String {
        // check if any file in this dir has git status
        let has_changes = self.git_status.keys().any(|p| p.starts_with(path));
        if has_changes {
            " *".to_string()
        } else {
            String::new()
        }
    }

    fn new_file(&mut self, dir: PathBuf) {
        let path = dir.join("untitled.txt");
        if let Ok(()) = std::fs::write(&path, "") {
            self.expanded.insert(dir);
        }
    }

    fn new_folder(&mut self, dir: PathBuf) {
        let path = dir.join("new-folder");
        let _ = std::fs::create_dir(&path);
        self.expanded.insert(path);
    }

    fn start_rename(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        self.rename_buf = name;
        self.rename_target = Some(path);
    }

    fn finish_rename(&mut self) {
        if let (Some(target), name) = (&self.rename_target, &self.rename_buf) {
            if let Some(parent) = target.parent() {
                let new_path = parent.join(name);
                let _ = std::fs::rename(target, new_path);
                self.refresh_git_status();
            }
        }
        self.rename_target = None;
        self.rename_buf.clear();
    }

    fn delete_path(&mut self, path: PathBuf) {
        let metadata = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => return,
        };
        if metadata.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
            self.expanded.remove(&path);
        } else {
            let _ = std::fs::remove_file(&path);
        }
        self.refresh_git_status();
    }
}

fn file_icon_for(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    let base = name.to_lowercase();
    match base.as_str() {
        "cargo.toml" | "package.json" | "makefile" | "cmakelists.txt" => return ph::GEAR,
        "dockerfile" | "docker-compose.yml" => return ph::PACKAGE,
        _ => {}
    }
    match ext.as_str() {
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "h" | "cpp" | "cc" | "cxx" | "hpp"
        | "cs" | "go" | "java" | "kt" | "swift" | "rb" | "php" | "lua" | "sh" | "bash" | "ps1"
        | "zig" | "nim" | "v" | "dart" | "ex" | "exs" | "hs" | "ml" | "scala" | "clj" | "r"
        | "sql" => ph::FILE_CODE,
        "md" | "txt" | "rst" | "adoc" => ph::FILE_TEXT,
        "json" | "yaml" | "yml" | "toml" | "xml" | "html" | "css" | "scss" | "graphql"
        | "proto" => ph::FILE_CODE,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" => ph::FILE_IMAGE,
        "zip" | "tar" | "gz" | "xz" | "bz2" | "7z" | "rar" => ph::FILE_ZIP,
        "lock" => ph::LOCK_KEY,
        "gitignore" | "gitattributes" => ph::GIT_BRANCH,
        _ => ph::FILE,
    }
}
