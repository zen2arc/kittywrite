use std::path::PathBuf;

pub struct EditorTab {
    pub title: String,
    pub path: Option<PathBuf>,
    pub content: String,
    pub language: String,
    pub modified: bool,
    cached_line_count: usize,
}

impl EditorTab {
    pub fn new_empty() -> Self {
        Self {
            title: "untitled".to_string(),
            path: None,
            content: String::new(),
            language: "Plain Text".to_string(),
            modified: false,
            cached_line_count: 1,
        }
    }

    pub fn from_path(path: PathBuf) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let language = detect_language(&path);
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_string();
        let cached_line_count = content.lines().count().max(1);

        Ok(Self {
            title,
            path: Some(path),
            content,
            language,
            modified: false,
            cached_line_count,
        })
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(ref p) = self.path {
            std::fs::write(p, &self.content)?;
            self.modified = false;
        }
        Ok(())
    }

    pub fn tab_label(&self) -> String {
        if self.modified {
            format!("\u{00b7} {}", self.title)
        } else {
            self.title.clone()
        }
    }

    pub fn line_count(&self) -> usize {
        self.cached_line_count
    }

    pub fn update_line_count(&mut self) {
        self.cached_line_count = self.content.lines().count().max(1);
    }
}

pub fn detect_language(path: &PathBuf) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    match name.as_str() {
        "Dockerfile" => return "Dockerfile".to_string(),
        "Makefile" | "makefile" => return "Makefile".to_string(),
        _ => {}
    }

    match ext.as_str() {
        "rs" => "Rust",
        "py" | "pyw" | "pyi" => "Python",
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" => "TypeScript",
        "jsx" => "JavaScript",
        "tsx" => "TypeScript",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" => "C++",
        "cs" => "C#",
        "go" => "Go",
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "swift" => "Swift",
        "rb" => "Ruby",
        "php" => "PHP",
        "lua" => "Lua",
        "sh" | "bash" | "zsh" | "fish" => "Bash",
        "ps1" | "psm1" => "PowerShell",
        "html" | "htm" => "HTML",
        "css" => "CSS",
        "scss" | "sass" => "CSS",
        "json" => "JSON",
        "yaml" | "yml" => "YAML",
        "toml" => "TOML",
        "md" | "markdown" => "Markdown",
        "sql" => "SQL",
        "xml" | "svg" | "xhtml" => "XML",
        "r" => "R",
        "dart" => "Dart",
        "ex" | "exs" => "Elixir",
        "hs" | "lhs" => "Haskell",
        "ml" | "mli" => "OCaml",
        "clj" | "cljs" | "edn" => "Clojure",
        "scala" => "Scala",
        "nim" => "Nim",
        "zig" => "Zig",
        "v" => "V",
        "tf" | "hcl" => "Terraform",
        "proto" => "Protobuf",
        "graphql" | "gql" => "GraphQL",
        "vim" | "vimrc" => "VimL",
        _ => "Plain Text",
    }
    .to_string()
}
