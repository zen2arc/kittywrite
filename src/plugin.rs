use mlua::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub enabled: bool,
}

struct LoadedPlugin {
    meta: PluginMeta,
    hooks: HashMap<String, LuaRegistryKey>,
    commands: HashMap<String, (String, LuaRegistryKey)>,
}

/// callbacks from the editor that plugins can use
pub struct EditorApi {
    pub get_content: Option<Rc<dyn Fn() -> String>>,
    pub set_content: Option<Rc<dyn Fn(String)>>,
    pub get_file_path: Option<Rc<dyn Fn() -> String>>,
    pub get_file_name: Option<Rc<dyn Fn() -> String>>,
    pub get_selection: Option<Rc<dyn Fn() -> String>>,
    pub set_selection: Option<Rc<dyn Fn(String)>>,
    pub get_cursor_line: Option<Rc<dyn Fn() -> usize>>,
    pub get_cursor_col: Option<Rc<dyn Fn() -> usize>>,
    pub set_cursor: Option<Rc<dyn Fn(usize, usize)>>,
    pub get_line_count: Option<Rc<dyn Fn() -> usize>>,
    pub get_line: Option<Rc<dyn Fn(usize) -> String>>,
    pub get_theme_name: Option<Rc<dyn Fn() -> String>>,
    pub get_font_size: Option<Rc<dyn Fn() -> f32>>,
}

impl Default for EditorApi {
    fn default() -> Self {
        Self {
            get_content: None,
            set_content: None,
            get_file_path: None,
            get_file_name: None,
            get_selection: None,
            set_selection: None,
            get_cursor_line: None,
            get_cursor_col: None,
            set_cursor: None,
            get_line_count: None,
            get_line: None,
            get_theme_name: None,
            get_font_size: None,
        }
    }
}

/// output from plugin hooks
#[derive(Clone, Debug)]
pub struct PluginOutput {
    pub plugin: String,
    pub message: String,
}

pub struct PluginManager {
    plugins: Vec<LoadedPlugin>,
    lua: Lua,
    pub editor_api: Rc<RefCell<EditorApi>>,
    pub outputs: Vec<PluginOutput>,
    pending_actions: Rc<RefCell<Vec<PluginAction>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            lua: Lua::new(),
            editor_api: Rc::new(RefCell::new(EditorApi::default())),
            outputs: Vec::new(),
            pending_actions: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn take_actions(&mut self) -> Vec<PluginAction> {
        std::mem::take(&mut *self.pending_actions.borrow_mut())
    }

    pub fn load_plugins(&mut self, plugin_dir: &std::path::Path) {
        self.plugins.clear();
        self.outputs.clear();

        let entries = match std::fs::read_dir(plugin_dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let init_lua = path.join("init.lua");
            if !init_lua.exists() {
                continue;
            }

            let plugin_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            match self.load_single_plugin(&init_lua, &plugin_name) {
                Ok(plugin) => {
                    eprintln!("plugin loaded: {}", plugin.meta.name);
                    self.plugins.push(plugin);
                }
                Err(e) => {
                    eprintln!("plugin load failed {}: {}", plugin_name, e);
                }
            }
        }

        // fire startup hook
        self.fire_hook("startup");
    }

    fn lua_err(e: impl std::fmt::Display) -> String {
        e.to_string()
    }

    fn load_single_plugin(
        &mut self,
        path: &std::path::Path,
        dir_name: &str,
    ) -> Result<LoadedPlugin, String> {
        let src = std::fs::read_to_string(path).map_err(|e| format!("cannot read: {}", e))?;

        let meta_out = Rc::new(RefCell::new(PluginMeta {
            name: dir_name.to_string(),
            version: "0.0.0".to_string(),
            description: String::new(),
            author: String::new(),
            enabled: true,
        }));
        let hooks_out: Rc<RefCell<HashMap<String, LuaRegistryKey>>> =
            Rc::new(RefCell::new(HashMap::new()));
        let commands_out: Rc<RefCell<HashMap<String, (String, LuaRegistryKey)>>> =
            Rc::new(RefCell::new(HashMap::new()));

        let meta_ref = meta_out.clone();
        let hooks_ref = hooks_out.clone();
        let commands_ref = commands_out.clone();

        let lua = &self.lua;

        // plugin constructor
        let plugin_ctor = lua
            .create_function(move |lua, meta_table: LuaTable| {
                let mut meta = meta_ref.borrow_mut();
                meta.name = meta_table.get::<String>("name").unwrap_or_default();
                meta.version = meta_table.get::<String>("version").unwrap_or_default();
                meta.description = meta_table.get::<String>("description").unwrap_or_default();
                meta.author = meta_table.get::<String>("author").unwrap_or_default();

                let hooks = hooks_ref.clone();
                let commands = commands_ref.clone();

                let on_fn = lua.create_function(
                    move |lua, (_self, event, cb): (LuaTable, String, LuaFunction)| {
                        let key = lua.create_registry_value(cb)?;
                        hooks.borrow_mut().insert(event, key);
                        Ok(())
                    },
                )?;

                let cmd_fn = lua.create_function(
                    move |lua, (_self, name, desc, cb): (LuaTable, String, String, LuaFunction)| {
                        let key = lua.create_registry_value(cb)?;
                        commands.borrow_mut().insert(name, (desc, key));
                        Ok(())
                    },
                )?;

                let plugin_table = lua.create_table()?;
                plugin_table.set("on", on_fn)?;
                plugin_table.set("command", cmd_fn)?;

                Ok(plugin_table)
            })
            .map_err(Self::lua_err)?;

        // kittywrite api - use shared actions list
        let actions = self.pending_actions.clone();

        let notify_fn = {
            let a = actions.clone();
            lua.create_function(move |_, msg: String| {
                a.borrow_mut().push(PluginAction::Notify(msg));
                Ok(())
            })
            .map_err(Self::lua_err)?
        };

        let log_fn = {
            let a = actions.clone();
            lua.create_function(move |_, msg: String| {
                a.borrow_mut().push(PluginAction::Log(msg));
                Ok(())
            })
            .map_err(Self::lua_err)?
        };

        let get_content_fn = {
            let api = self.editor_api.clone();
            lua.create_function(move |_, ()| {
                let api = api.borrow();
                match &api.get_content {
                    Some(f) => Ok(f()),
                    None => Ok(String::new()),
                }
            })
            .map_err(Self::lua_err)?
        };

        let get_file_path_fn = {
            let api = self.editor_api.clone();
            lua.create_function(move |_, ()| {
                let api = api.borrow();
                match &api.get_file_path {
                    Some(f) => Ok(f()),
                    None => Ok(String::new()),
                }
            })
            .map_err(Self::lua_err)?
        };

        let set_theme_fn = {
            let a = actions.clone();
            lua.create_function(move |_, name: String| {
                a.borrow_mut().push(PluginAction::SetTheme(name));
                Ok(())
            })
            .map_err(Self::lua_err)?
        };

        let get_theme_fn = {
            let api = self.editor_api.clone();
            lua.create_function(move |_, ()| {
                let api = api.borrow();
                match &api.get_theme_name {
                    Some(f) => Ok(f()),
                    None => Ok("kittywrite".to_string()),
                }
            })
            .map_err(Self::lua_err)?
        };

        let get_font_size_fn = {
            let api = self.editor_api.clone();
            lua.create_function(move |_, ()| {
                let api = api.borrow();
                match &api.get_font_size {
                    Some(f) => Ok(f() as f64),
                    None => Ok(14.0f64),
                }
            })
            .map_err(Self::lua_err)?
        };

        let set_font_size_fn = {
            let a = actions.clone();
            lua.create_function(move |_, size: f64| {
                a.borrow_mut().push(PluginAction::SetFontSize(size as f32));
                Ok(())
            })
            .map_err(Self::lua_err)?
        };

        let open_file_fn = {
            let a = actions.clone();
            lua.create_function(move |_, path: String| {
                a.borrow_mut().push(PluginAction::OpenFile(path));
                Ok(())
            })
            .map_err(Self::lua_err)?
        };

        let save_file_fn = {
            let a = actions.clone();
            lua.create_function(move |_, ()| {
                a.borrow_mut().push(PluginAction::SaveFile);
                Ok(())
            })
            .map_err(Self::lua_err)?
        };

        let new_file_fn = {
            let a = actions.clone();
            lua.create_function(move |_, ()| {
                a.borrow_mut().push(PluginAction::NewFile);
                Ok(())
            })
            .map_err(Self::lua_err)?
        };

        let set_content_fn = {
            let a = actions.clone();
            lua.create_function(move |_, content: String| {
                a.borrow_mut().push(PluginAction::SetContent(content));
                Ok(())
            })
            .map_err(Self::lua_err)?
        };

        let get_selection_fn = {
            let api = self.editor_api.clone();
            lua.create_function(move |_, ()| {
                let api = api.borrow();
                match &api.get_selection {
                    Some(f) => Ok(f()),
                    None => Ok(String::new()),
                }
            })
            .map_err(Self::lua_err)?
        };

        let set_selection_fn = {
            let a = actions.clone();
            lua.create_function(move |_, text: String| {
                a.borrow_mut().push(PluginAction::SetSelection(text));
                Ok(())
            })
            .map_err(Self::lua_err)?
        };

        let get_cursor_line_fn = {
            let api = self.editor_api.clone();
            lua.create_function(move |_, ()| {
                let api = api.borrow();
                match &api.get_cursor_line {
                    Some(f) => Ok(f() as i64),
                    None => Ok(1i64),
                }
            })
            .map_err(Self::lua_err)?
        };

        let get_cursor_col_fn = {
            let api = self.editor_api.clone();
            lua.create_function(move |_, ()| {
                let api = api.borrow();
                match &api.get_cursor_col {
                    Some(f) => Ok(f() as i64),
                    None => Ok(1i64),
                }
            })
            .map_err(Self::lua_err)?
        };

        let set_cursor_fn = {
            let a = actions.clone();
            lua.create_function(move |_, (line, col): (i64, i64)| {
                a.borrow_mut()
                    .push(PluginAction::SetCursor(line as usize, col as usize));
                Ok(())
            })
            .map_err(Self::lua_err)?
        };

        let get_line_count_fn = {
            let api = self.editor_api.clone();
            lua.create_function(move |_, ()| {
                let api = api.borrow();
                match &api.get_line_count {
                    Some(f) => Ok(f() as i64),
                    None => Ok(1i64),
                }
            })
            .map_err(Self::lua_err)?
        };

        let get_line_fn = {
            let api = self.editor_api.clone();
            lua.create_function(move |_, line: i64| {
                let api = api.borrow();
                match &api.get_line {
                    Some(f) => Ok(f(line as usize)),
                    None => Ok(String::new()),
                }
            })
            .map_err(Self::lua_err)?
        };

        let get_file_name_fn = {
            let api = self.editor_api.clone();
            lua.create_function(move |_, ()| {
                let api = api.borrow();
                match &api.get_file_name {
                    Some(f) => Ok(f()),
                    None => Ok(String::new()),
                }
            })
            .map_err(Self::lua_err)?
        };

        // file system
        let read_file_fn = lua
            .create_function(|_, path: String| {
                std::fs::read_to_string(&path).map_err(|e| mlua::Error::RuntimeError(e.to_string()))
            })
            .map_err(Self::lua_err)?;

        let write_file_fn = lua
            .create_function(|_, (path, content): (String, String)| {
                std::fs::write(&path, content).map_err(|e| mlua::Error::RuntimeError(e.to_string()))
            })
            .map_err(Self::lua_err)?;

        let file_exists_fn = lua
            .create_function(|_, path: String| Ok(std::path::Path::new(&path).exists()))
            .map_err(Self::lua_err)?;

        let read_dir_fn = lua
            .create_function(|_, path: String| {
                let entries: Vec<String> = match std::fs::read_dir(&path) {
                    Ok(dir) => dir
                        .filter_map(|e| e.ok())
                        .map(|e| e.path().to_string_lossy().to_string())
                        .collect(),
                    Err(_) => Vec::new(),
                };
                Ok(entries)
            })
            .map_err(Self::lua_err)?;

        // register globals
        let globals = lua.globals();
        globals.set("plugin", plugin_ctor).map_err(Self::lua_err)?;

        let kw = lua.create_table().map_err(Self::lua_err)?;
        // file operations
        kw.set("open_file", open_file_fn).map_err(Self::lua_err)?;
        kw.set("save_file", save_file_fn).map_err(Self::lua_err)?;
        kw.set("new_file", new_file_fn).map_err(Self::lua_err)?;
        // content
        kw.set("get_content", get_content_fn)
            .map_err(Self::lua_err)?;
        kw.set("set_content", set_content_fn)
            .map_err(Self::lua_err)?;
        kw.set("get_selection", get_selection_fn)
            .map_err(Self::lua_err)?;
        kw.set("set_selection", set_selection_fn)
            .map_err(Self::lua_err)?;
        // cursor
        kw.set("get_cursor_line", get_cursor_line_fn)
            .map_err(Self::lua_err)?;
        kw.set("get_cursor_col", get_cursor_col_fn)
            .map_err(Self::lua_err)?;
        kw.set("set_cursor", set_cursor_fn).map_err(Self::lua_err)?;
        // file info
        kw.set("get_file_path", get_file_path_fn)
            .map_err(Self::lua_err)?;
        kw.set("get_file_name", get_file_name_fn)
            .map_err(Self::lua_err)?;
        kw.set("get_line_count", get_line_count_fn)
            .map_err(Self::lua_err)?;
        kw.set("get_line", get_line_fn).map_err(Self::lua_err)?;
        // theme and font
        kw.set("get_theme", get_theme_fn).map_err(Self::lua_err)?;
        kw.set("set_theme", set_theme_fn).map_err(Self::lua_err)?;
        kw.set("get_font_size", get_font_size_fn)
            .map_err(Self::lua_err)?;
        kw.set("set_font_size", set_font_size_fn)
            .map_err(Self::lua_err)?;
        // ui
        kw.set("notify", notify_fn).map_err(Self::lua_err)?;
        kw.set("log", log_fn).map_err(Self::lua_err)?;
        globals.set("kittywrite", kw).map_err(Self::lua_err)?;

        globals
            .set("read_file", read_file_fn)
            .map_err(Self::lua_err)?;
        globals
            .set("write_file", write_file_fn)
            .map_err(Self::lua_err)?;
        globals
            .set("file_exists", file_exists_fn)
            .map_err(Self::lua_err)?;
        globals
            .set("read_dir", read_dir_fn)
            .map_err(Self::lua_err)?;

        // execute plugin
        lua.load(&src)
            .set_name(path.to_str().unwrap_or("plugin"))
            .exec()
            .map_err(|e| format!("lua error: {}", e))?;

        // keep kittywrite, cleanup only one-time setup
        let _ = globals.set("plugin", LuaNil);
        let _ = globals.set("read_file", LuaNil);
        let _ = globals.set("write_file", LuaNil);
        let _ = globals.set("file_exists", LuaNil);
        let _ = globals.set("read_dir", LuaNil);

        let meta = PluginMeta {
            name: meta_out.borrow().name.clone(),
            version: meta_out.borrow().version.clone(),
            description: meta_out.borrow().description.clone(),
            author: meta_out.borrow().author.clone(),
            enabled: true,
        };

        let hooks = std::mem::take(&mut *hooks_out.borrow_mut());
        let commands = std::mem::take(&mut *commands_out.borrow_mut());

        Ok(LoadedPlugin {
            meta,
            hooks,
            commands,
        })
    }

    pub fn fire_hook(&mut self, event: &str) {
        let names: Vec<String> = self.plugins.iter().map(|p| p.meta.name.clone()).collect();
        for (i, plugin) in self.plugins.iter().enumerate() {
            if !plugin.meta.enabled {
                continue;
            }
            if let Some(key) = plugin.hooks.get(event) {
                if let Ok(func) = self.lua.registry_value::<LuaFunction>(key) {
                    match func.call::<()>(()) {
                        Ok(_) => {}
                        Err(e) => {
                            self.outputs.push(PluginOutput {
                                plugin: names[i].clone(),
                                message: format!("error in {}: {}", event, e),
                            });
                        }
                    }
                }
            }
        }
    }

    pub fn fire_hook_str(&mut self, event: &str, arg: &str) {
        let names: Vec<String> = self.plugins.iter().map(|p| p.meta.name.clone()).collect();
        for (i, plugin) in self.plugins.iter().enumerate() {
            if !plugin.meta.enabled {
                continue;
            }
            if let Some(key) = plugin.hooks.get(event) {
                if let Ok(func) = self.lua.registry_value::<LuaFunction>(key) {
                    match func.call::<()>(arg.to_string()) {
                        Ok(_) => {}
                        Err(e) => {
                            self.outputs.push(PluginOutput {
                                plugin: names[i].clone(),
                                message: format!("error in {}: {}", event, e),
                            });
                        }
                    }
                }
            }
        }
    }

    pub fn fire_command(&mut self, name: &str) -> Option<Result<String, String>> {
        let plugin_name = self
            .plugins
            .iter()
            .find(|p| p.commands.contains_key(name))
            .map(|p| p.meta.name.clone());

        for plugin in &self.plugins {
            if !plugin.meta.enabled {
                continue;
            }
            if let Some((_, key)) = plugin.commands.get(name) {
                if let Ok(func) = self.lua.registry_value::<LuaFunction>(key) {
                    let result = func.call::<String>(());
                    match &result {
                        Ok(s) => {
                            if !s.is_empty() {
                                self.outputs.push(PluginOutput {
                                    plugin: plugin.meta.name.clone(),
                                    message: s.clone(),
                                });
                            }
                        }
                        Err(e) => {
                            self.outputs.push(PluginOutput {
                                plugin: plugin.meta.name.clone(),
                                message: format!("error: {}", e),
                            });
                        }
                    }
                    return Some(result.map_err(|e| e.to_string()));
                }
            }
        }
        None
    }

    pub fn list_commands(&self) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for plugin in &self.plugins {
            if !plugin.meta.enabled {
                continue;
            }
            for (name, (desc, _)) in &plugin.commands {
                out.push((name.clone(), desc.clone(), plugin.meta.name.clone()));
            }
        }
        out
    }

    pub fn list_plugins(&self) -> Vec<PluginMeta> {
        self.plugins.iter().map(|p| p.meta.clone()).collect()
    }

    pub fn has_plugins(&self) -> bool {
        !self.plugins.is_empty()
    }

    pub fn toggle_plugin(&mut self, name: &str) {
        if let Some(plugin) = self.plugins.iter_mut().find(|p| p.meta.name == name) {
            plugin.meta.enabled = !plugin.meta.enabled;
        }
    }

    pub fn clear_outputs(&mut self) {
        self.outputs.clear();
    }
}

pub enum PluginAction {
    OpenFile(String),
    SaveFile,
    NewFile,
    Notify(String),
    Log(String),
    SetTheme(String),
    SetFontSize(f32),
    SetContent(String),
    SetSelection(String),
    SetCursor(usize, usize),
}
