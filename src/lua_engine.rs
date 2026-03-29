use mlua::prelude::*;

#[derive(Clone)]
pub struct LuaConfig {
    pub font_size: f32,
    pub tab_width: usize,
    pub auto_indent: bool,
    pub auto_pair: bool,
    pub line_height: f32,
    pub word_wrap: bool,
    pub show_line_numbers: bool,
}

impl Default for LuaConfig {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            tab_width: 4,
            auto_indent: true,
            auto_pair: true,
            line_height: 1.0,
            word_wrap: false,
            show_line_numbers: true,
        }
    }
}

const BUILTIN_CONFIG: &str = r#"
kittywrite.font_size = 14
kittywrite.tab_width = 4
kittywrite.auto_indent = true
kittywrite.auto_pair = true
kittywrite.line_height = 1.0
kittywrite.word_wrap = false
kittywrite.show_line_numbers = true
"#;

pub struct LuaEngine {
    lua: Lua,
    pub config: LuaConfig,
}

impl LuaEngine {
    pub fn new() -> Self {
        let lua = Lua::new();
        let mut engine = Self {
            lua,
            config: LuaConfig::default(),
        };
        engine.setup_globals();
        engine.run_config();
        engine.read_config();
        engine
    }

    fn setup_globals(&self) {
        let globals = self.lua.globals();
        let kw = match self.lua.create_table() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("lua: cant create kittywrite table: {}", e);
                return;
            }
        };
        let _ = kw.set("font_size", 14.0_f64);
        let _ = kw.set("tab_width", 4_i64);
        let _ = kw.set("auto_indent", true);
        let _ = kw.set("auto_pair", true);
        let _ = kw.set("line_height", 1.0_f64);
        let _ = kw.set("word_wrap", false);
        let _ = kw.set("show_line_numbers", true);
        let _ = globals.set("kittywrite", kw);
    }

    fn run_config(&self) {
        let src = load_user_config().unwrap_or_else(|| BUILTIN_CONFIG.to_string());
        if let Err(e) = self.lua.load(&src).set_name("init.lua").exec() {
            eprintln!("lua config error: {}", e);
        }
    }

    fn read_config(&mut self) {
        let globals = self.lua.globals();
        let kw: LuaTable = match globals.get("kittywrite") {
            Ok(t) => t,
            Err(_) => return,
        };
        if let Ok(v) = kw.get::<&str, f64>("font_size") {
            self.config.font_size = (v as f32).clamp(8.0, 48.0);
        }
        if let Ok(v) = kw.get::<&str, i64>("tab_width") {
            self.config.tab_width = (v as usize).clamp(1, 16);
        }
        if let Ok(v) = kw.get::<&str, bool>("auto_indent") {
            self.config.auto_indent = v;
        }
        if let Ok(v) = kw.get::<&str, bool>("auto_pair") {
            self.config.auto_pair = v;
        }
        if let Ok(v) = kw.get::<&str, f64>("line_height") {
            self.config.line_height = (v as f32).clamp(1.0, 3.0);
        }
        if let Ok(v) = kw.get::<&str, bool>("word_wrap") {
            self.config.word_wrap = v;
        }
        if let Ok(v) = kw.get::<&str, bool>("show_line_numbers") {
            self.config.show_line_numbers = v;
        }
    }

    pub fn exec(&self, code: &str) -> Result<String, String> {
        let result = self.lua.load(code).eval::<LuaMultiValue>();
        match result {
            Ok(vals) => {
                let out: Vec<String> = vals.iter().map(|v| format!("{:?}", v)).collect();
                Ok(out.join("  "))
            }
            Err(_) => self
                .lua
                .load(code)
                .exec()
                .map(|_| String::new())
                .map_err(|e| e.to_string()),
        }
    }
}

impl Default for LuaEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn load_user_config() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let path = dir.join("init.lua");
    std::fs::read_to_string(path).ok()
}
