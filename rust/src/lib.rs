use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, BroadcastChannel, MessageEvent};
use js_sys::{Uint8Array, Date};
use std::cell::RefCell;
use std::collections::HashMap;
use serde::Deserialize;

// External JavaScript functions that Rust can call
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = addOutput)]
    fn add_output(text: &str);

    #[wasm_bindgen(js_name = clearOutput)]
    fn clear_output();

    #[wasm_bindgen(js_name = promptFilePicker)]
    fn prompt_file_picker(accept: &str) -> js_sys::Promise;

    #[wasm_bindgen(js_name = triggerDownload)]
    fn trigger_download(content: &[u8], mime_type: &str, filename: &str);

    #[wasm_bindgen(js_name = scrollToBottom)]
    fn scroll_to_bottom();
}

// Handler for next input - determines what function receives the next user input
#[derive(Clone)]
enum NextInputHandler {
    None,
    PrettyConfirm { filepath: String, path_arg: String },
}

thread_local! {
    static CURRENT_DIR: RefCell<DirPath> = RefCell::new(DirPath::root());
    static VIRTUAL_FS: RefCell<VirtualFilesystem> = RefCell::new(VirtualFilesystem::new());
    static NEXT_INPUT_HANDLER: RefCell<NextInputHandler> = RefCell::new(NextInputHandler::None);
    static EDITOR_CHANNEL: RefCell<Option<web_sys::BroadcastChannel>> = RefCell::new(None);
    static PRETTY_CHANNEL: RefCell<Option<web_sys::BroadcastChannel>> = RefCell::new(None);
}

#[derive(Deserialize, Clone)]
struct FileEntry {
    name: String,
    path: String,
}

#[derive(Deserialize, Clone)]
struct Manifest {
    files: Vec<FileEntry>,
    directories: Vec<String>,
}

/// Content can either be in memory or needs to be fetched
#[derive(Clone)]
enum Content {
    InMemory(String),
    ToFetch,
}

/// Virtual filesystem stored in WASM memory
struct VirtualFilesystem {
    content: HashMap<DirPath, HashMap<String, Content>>,
}

impl VirtualFilesystem {
    fn new() -> Self {
        Self {
            content: HashMap::new(),
        }
    }

    /// Initialize from manifest - loads all static files as ToFetch
    fn initialize_from_manifest(&mut self, manifest: &Manifest) {
        // Add root directory
        self.content.insert(DirPath::root(), HashMap::new());

        // Add all directories from manifest
        for dir_str in &manifest.directories {
            let mut dir = DirPath::root();
            for component in dir_str.split('/').filter(|s| !s.is_empty()) {
                dir.cd(&NextDir::In(component.to_string()), true);
            }
            self.content.insert(dir, HashMap::new());
        }

        // Add all files from manifest as ToFetch
        for file_entry in &manifest.files {
            let mut dir = DirPath::root();
            for component in file_entry.path.split('/').filter(|s| !s.is_empty()) {
                dir.cd(&NextDir::In(component.to_string()), true);
            }

            self.content
                .entry(dir)
                .or_insert_with(HashMap::new)
                .insert(file_entry.name.clone(), Content::ToFetch);
        }
    }

    /// Write a file to the virtual filesystem (in memory)
    fn write_file(&mut self, filepath: &FilePath, content: String) {
        self.content
            .entry(filepath.dir.clone())
            .or_insert_with(HashMap::new)
            .insert(filepath.file.clone(), Content::InMemory(content));
    }

    /// Get content type for a file
    fn get_content(&self, filepath: &FilePath) -> Option<&Content> {
        self.content.get(&filepath.dir)?.get(&filepath.file)
    }

    /// Check if a file exists in the virtual filesystem
    fn file_exists(&self, filepath: &FilePath) -> bool {
        self.content
            .get(&filepath.dir)
            .and_then(|files| files.get(&filepath.file))
            .is_some()
    }

    /// Remove a file from the virtual filesystem
    fn remove_file(&mut self, filepath: &FilePath) -> bool {
        if let Some(files) = self.content.get_mut(&filepath.dir) {
            files.remove(&filepath.file).is_some()
        } else {
            false
        }
    }

    /// Create a directory
    fn create_dir(&mut self, dirpath: DirPath) {
        self.content.entry(dirpath).or_insert_with(HashMap::new);
    }

    /// Check if a directory exists
    fn dir_exists(&self, dirpath: &DirPath) -> bool {
        self.content.contains_key(dirpath)
    }

    /// Remove a directory (only if empty)
    fn remove_dir(&mut self, dirpath: &DirPath) -> Result<(), String> {
        // Check if directory has any files
        if let Some(files) = self.content.get(dirpath) {
            if !files.is_empty() {
                return Err("Directory not empty".to_string());
            }
        } else {
            return Err("Directory does not exist".to_string());
        }

        // Check if any subdirectories exist
        for dir in self.content.keys() {
            if dir.0.len() > dirpath.0.len() {
                let mut is_subdir = true;
                for (i, component) in dirpath.0.iter().enumerate() {
                    if dir.0[i] != *component {
                        is_subdir = false;
                        break;
                    }
                }
                if is_subdir {
                    return Err("Directory not empty".to_string());
                }
            }
        }

        self.content.remove(dirpath);
        Ok(())
    }

    /// List all files in a given directory (returns just filenames)
    fn list_files_in_dir(&self, dirpath: &DirPath) -> Vec<String> {
        if let Some(files) = self.content.get(dirpath) {
            let mut filenames: Vec<String> = files.keys().cloned().collect();
            filenames.sort();
            filenames
        } else {
            Vec::new()
        }
    }

    /// Get all immediate subdirectories of a given directory (returns just dir names)
    fn list_subdirs_in_dir(&self, dirpath: &DirPath) -> Vec<String> {
        let mut subdirs = std::collections::HashSet::new();

        for dir in self.content.keys() {
            // Check if this is an immediate subdirectory
            if dir.0.len() == dirpath.0.len() + 1 {
                let mut is_subdir = true;
                for (i, component) in dirpath.0.iter().enumerate() {
                    if dir.0[i] != *component {
                        is_subdir = false;
                        break;
                    }
                }

                if is_subdir {
                    if let NextDir::In(name) = &dir.0[dirpath.0.len()] {
                        subdirs.insert(name.clone());
                    }
                }
            }
        }

        let mut result: Vec<String> = subdirs.into_iter().collect();
        result.sort();
        result
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum NextDir {
    In(String),
    Out
}

// DirPath([In("usr"),Out,In("Documents")]) interpreted as /usr/../Documents
#[derive(Clone, PartialEq, Eq, Hash)]
struct DirPath(Vec<NextDir>);

#[derive(Clone, PartialEq, Eq, Hash)]
struct FilePath {
    dir: DirPath,
    file: String
}

impl FilePath {
    fn new(dir: DirPath, file: String) -> Self {
        Self { dir, file }
    }

    // Parse a path string into FilePath
    fn parse(path: &str, current_dir: &DirPath) -> Self {
        let parts: Vec<&str> = path.rsplitn(2, '/').collect();

        if parts.len() == 1 {
            // Just a filename, use current directory
            Self::new(current_dir.clone(), parts[0].to_string())
        } else {
            // Has directory component
            let filename = parts[0].to_string();
            let dir_part = parts[1];

            let dir = if path.starts_with('/') {
                // Absolute path
                let mut new_dir = DirPath::root();
                for component in dir_part.split('/').filter(|s| !s.is_empty()) {
                    match component {
                        "." => {},
                        ".." => new_dir.cd(&NextDir::Out, true),
                        name => new_dir.cd(&NextDir::In(name.to_string()), true),
                    }
                }
                new_dir
            } else {
                // Relative path
                let mut new_dir = current_dir.clone();
                for component in dir_part.split('/').filter(|s| !s.is_empty()) {
                    match component {
                        "." => {},
                        ".." => new_dir.cd(&NextDir::Out, true),
                        name => new_dir.cd(&NextDir::In(name.to_string()), true),
                    }
                }
                new_dir
            };

            Self::new(dir, filename)
        }
    }

    // Get full path as string (e.g., "/blog/making_this.md")
    fn to_string(&self) -> String {
        let dir_str = self.dir.to_string();
        if dir_str == "/" {
            format!("/{}", self.file)
        } else {
            format!("{}/{}", dir_str, self.file)
        }
    }

    // Get URL for fetching from content directory
    fn to_url(&self) -> String {
        let dir_str = self.dir.to_string();
        let path_component = if dir_str == "/" {
            "".to_string()
        } else {
            dir_str.trim_start_matches('/').to_string() + "/"
        };
        format!("./content/{}{}", path_component, self.file)
    }

    // Check if this file exists in the virtual filesystem
    fn exists(&self) -> bool {
        VIRTUAL_FS.with(|vfs| {
            vfs.borrow().file_exists(self)
        })
    }
}

impl DirPath {
    fn root() -> Self {
        Self(Vec::new())
    }

    fn normalised(&self, at_root: bool) -> Self {
        let mut out = Self::root();
        for v in &self.0 {
            out.cd(v, at_root);
        }
        out
    }

    fn normalise(&mut self, at_root: bool) {
        self.0 = self.normalised(at_root).0;
    }

    fn cd(&mut self, next: &NextDir, at_root: bool) {
        let DirPath(vec) = self;
        match next {
            NextDir::In(_) => {
                vec.push(next.clone());
            },
            NextDir::Out => {
                if !at_root {
                    match vec.pop() {
                        None => {
                            vec.push(NextDir::Out);
                        },
                        Some(NextDir::Out) => {
                            vec.push(NextDir::Out);
                            vec.push(NextDir::Out);
                        },
                        _ => {}
                    };
                } else {
                    vec.pop();
                }
            },
        }
    }

    fn to_string(&self) -> String {
        if self.0.is_empty() {
            return "/".to_string();
        }
        let mut result = String::new();
        for dir in &self.0 {
            match dir {
                NextDir::In(name) => {
                    result.push('/');
                    result.push_str(name);
                },
                NextDir::Out => {
                    result.push_str("/..");
                }
            }
        }
        result
    }

    fn concat(&self, relative: &Self, at_root: bool) -> Self {
        let mut out = self.clone();
        for v in &relative.0 {
            out.cd(v,at_root);
        }
        out
    }
}

// Async fetch helper
async fn fetch_text(url: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or("No window object")?;

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|_| format!("Failed to create request for {}", url))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| format!("Failed to fetch {}", url))?;

    let resp: Response = resp_value.dyn_into()
        .map_err(|_| "Response is not a Response object")?;

    if !resp.ok() {
        return Err(format!("Failed to fetch {}: HTTP {}", url, resp.status()));
    }

    let text_promise = resp.text().map_err(|_| "Failed to get response text")?;
    let text = JsFuture::from(text_promise)
        .await
        .map_err(|_| "Failed to read response text")?;

    text.as_string().ok_or_else(|| "Response text is not a string".to_string())
}

// Load manifest from server and initialize virtual filesystem
#[wasm_bindgen]
pub async fn load_manifest() -> Result<(), JsValue> {
    let manifest_text = fetch_text("./content/manifest.json")
        .await
        .map_err(|e| JsValue::from_str(&e))?;

    let manifest: Manifest = serde_json::from_str(&manifest_text)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse manifest: {}", e)))?;

    // Initialize virtual filesystem from manifest (manifest is then dropped)
    VIRTUAL_FS.with(|vfs| {
        vfs.borrow_mut().initialize_from_manifest(&manifest);
    });

    Ok(())
}

// Initialize BroadcastChannels for communication with editor and pretty viewer
#[wasm_bindgen]
pub fn initialize_broadcast_channels() -> Result<(), JsValue> {
    use wasm_bindgen::closure::Closure;

    // Create editor channel
    let editor_channel = BroadcastChannel::new("editor_channel")?;
    let editor_onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        handle_editor_message(event);
    });
    editor_channel.set_onmessage(Some(editor_onmessage.as_ref().unchecked_ref()));
    editor_onmessage.forget(); // Keep the closure alive

    // Create pretty channel
    let pretty_channel = BroadcastChannel::new("pretty_channel")?;
    let pretty_onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        handle_pretty_message(event);
    });
    pretty_channel.set_onmessage(Some(pretty_onmessage.as_ref().unchecked_ref()));
    pretty_onmessage.forget(); // Keep the closure alive

    // Store channels
    EDITOR_CHANNEL.with(|ch| {
        *ch.borrow_mut() = Some(editor_channel);
    });
    PRETTY_CHANNEL.with(|ch| {
        *ch.borrow_mut() = Some(pretty_channel);
    });

    Ok(())
}

// Handle messages from editor
fn handle_editor_message(event: MessageEvent) {
    let data = event.data();

    // Parse message data
    if let Ok(obj) = data.dyn_into::<js_sys::Object>() {
        let action = js_sys::Reflect::get(&obj, &JsValue::from_str("action")).ok();
        let filename = js_sys::Reflect::get(&obj, &JsValue::from_str("filename")).ok();
        let content = js_sys::Reflect::get(&obj, &JsValue::from_str("content")).ok();

        if let (Some(action), Some(filename)) = (action, filename) {
            let action_str = action.as_string().unwrap_or_default();
            let filename_str = filename.as_string().unwrap_or_default();

            match action_str.as_str() {
                "file_saved" => {
                    if let Some(content) = content {
                        if let Some(content_str) = content.as_string() {
                            // Write file to WASM virtual filesystem
                            let filepath = FilePath::parse(&filename_str, &DirPath::root());
                            VIRTUAL_FS.with(|vfs| {
                                vfs.borrow_mut().write_file(&filepath, content_str);
                            });
                            add_output(&format!("File saved: {}", filename_str));
                            add_output("&nbsp;");
                        }
                    }
                }
                "request_file" => {
                    send_file_content(&filename_str, true);
                }
                _ => {}
            }
        }
    }
}

// Handle messages from pretty viewer
fn handle_pretty_message(event: MessageEvent) {
    let data = event.data();

    if let Ok(obj) = data.dyn_into::<js_sys::Object>() {
        let action = js_sys::Reflect::get(&obj, &JsValue::from_str("action")).ok();
        let filename = js_sys::Reflect::get(&obj, &JsValue::from_str("filename")).ok();

        if let (Some(action), Some(filename)) = (action, filename) {
            let action_str = action.as_string().unwrap_or_default();
            let filename_str = filename.as_string().unwrap_or_default();

            if action_str == "request_file" {
                send_file_content(&filename_str, false);
            }
        }
    }
}

// Send file content via BroadcastChannel
fn send_file_content(filename: &str, to_editor: bool) {
    let filepath = FilePath::parse(filename, &DirPath::root());

    // Get content from virtual filesystem
    let content_result = VIRTUAL_FS.with(|vfs| {
        vfs.borrow().get_content(&filepath).cloned()
    });

    let channel = if to_editor {
        EDITOR_CHANNEL.with(|ch| ch.borrow().clone())
    } else {
        PRETTY_CHANNEL.with(|ch| ch.borrow().clone())
    };

    if let Some(channel) = channel {
        match content_result {
            Some(Content::InMemory(content)) => {
                // Send in-memory content
                let message = js_sys::Object::new();
                js_sys::Reflect::set(&message, &JsValue::from_str("action"), &JsValue::from_str("file_content")).ok();
                js_sys::Reflect::set(&message, &JsValue::from_str("filename"), &JsValue::from_str(filename)).ok();
                js_sys::Reflect::set(&message, &JsValue::from_str("content"), &JsValue::from_str(&content)).ok();
                channel.post_message(&message).ok();
            }
            Some(Content::ToFetch) => {
                // Need to fetch - spawn async task
                let filename = filename.to_string();
                let channel_clone = channel.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let filepath = FilePath::parse(&filename, &DirPath::root());
                    match fetch_text(&filepath.to_url()).await {
                        Ok(content) => {
                            let message = js_sys::Object::new();
                            js_sys::Reflect::set(&message, &JsValue::from_str("action"), &JsValue::from_str("file_content")).ok();
                            js_sys::Reflect::set(&message, &JsValue::from_str("filename"), &JsValue::from_str(&filename)).ok();
                            js_sys::Reflect::set(&message, &JsValue::from_str("content"), &JsValue::from_str(&content)).ok();
                            channel_clone.post_message(&message).ok();
                        }
                        Err(_) => {
                            // Send empty content for errors
                            let message = js_sys::Object::new();
                            js_sys::Reflect::set(&message, &JsValue::from_str("action"), &JsValue::from_str("file_content")).ok();
                            js_sys::Reflect::set(&message, &JsValue::from_str("filename"), &JsValue::from_str(&filename)).ok();
                            js_sys::Reflect::set(&message, &JsValue::from_str("content"), &JsValue::from_str("")).ok();
                            channel_clone.post_message(&message).ok();
                        }
                    }
                });
            }
            None => {
                // File not found, send empty content
                let message = js_sys::Object::new();
                js_sys::Reflect::set(&message, &JsValue::from_str("action"), &JsValue::from_str("file_content")).ok();
                js_sys::Reflect::set(&message, &JsValue::from_str("filename"), &JsValue::from_str(filename)).ok();
                js_sys::Reflect::set(&message, &JsValue::from_str("content"), &JsValue::from_str("")).ok();
                channel.post_message(&message).ok();
            }
        }
    }
}

// Write a file to the virtual filesystem (called from JavaScript/editor)
#[wasm_bindgen]
pub fn write_file(path: &str, content: String) -> Result<(), JsValue> {
    let filepath = CURRENT_DIR.with(|cd| FilePath::parse(path, &cd.borrow()));

    VIRTUAL_FS.with(|vfs| {
        vfs.borrow_mut().write_file(&filepath, content);
    });

    Ok(())
}

// Read a file from the virtual filesystem (called from JavaScript)
// Returns the content type: "InMemory:<content>", "ToFetch:<url>", or "NotFound"
#[wasm_bindgen]
pub fn read_file(path: &str) -> String {
    let filepath = CURRENT_DIR.with(|cd| FilePath::parse(path, &cd.borrow()));

    VIRTUAL_FS.with(|vfs| {
        match vfs.borrow().get_content(&filepath) {
            Some(Content::InMemory(content)) => format!("InMemory:{}", content),
            Some(Content::ToFetch) => format!("ToFetch:{}", filepath.to_url()),
            None => "NotFound".to_string(),
        }
    })
}

// Export all in-memory files as JSON (called from JavaScript)
#[wasm_bindgen]
pub fn export_session() -> String {
    use serde_json::json;

    VIRTUAL_FS.with(|vfs| {
        let vfs_ref = vfs.borrow();
        let mut files = serde_json::Map::new();

        // Collect all InMemory files
        for (dirpath, dir_contents) in &vfs_ref.content {
            for (filename, content) in dir_contents {
                if let Content::InMemory(file_content) = content {
                    let mut path_parts = Vec::new();
                    for component in &dirpath.0 {
                        match component {
                            NextDir::In(name) => path_parts.push(name.clone()),
                            NextDir::Out => path_parts.push("..".to_string()),
                        }
                    }

                    let full_path = if path_parts.is_empty() {
                        format!("/{}", filename)
                    } else {
                        format!("/{}/{}", path_parts.join("/"), filename)
                    };

                    files.insert(full_path, json!(file_content));
                }
            }
        }

        json!({
            "version": "1.0",
            "files": files
        }).to_string()
    })
}

// Import session from JSON (called from JavaScript)
// Returns number of files imported, or error message prefixed with "Error:"
#[wasm_bindgen]
pub fn import_session(session_json: String) -> String {
    use serde_json::Value;

    match serde_json::from_str::<Value>(&session_json) {
        Ok(session) => {
            // Check version
            if let Some(version) = session.get("version").and_then(|v| v.as_str()) {
                if version != "1.0" {
                    return format!("Error: Unsupported session version: {}", version);
                }
            } else {
                return "Error: Invalid session file: missing version".to_string();
            }

            // Get files object
            let files = match session.get("files").and_then(|f| f.as_object()) {
                Some(f) => f,
                None => return "Error: Invalid session file: missing or invalid files".to_string(),
            };

            let mut count = 0;

            // Import each file
            VIRTUAL_FS.with(|vfs| {
                for (path, content_value) in files {
                    if let Some(content_str) = content_value.as_str() {
                        // Parse the path
                        let filepath = FilePath::parse(path, &DirPath::root());

                        // Write to virtual filesystem
                        vfs.borrow_mut().write_file(&filepath, content_str.to_string());
                        count += 1;
                    }
                }
            });

            format!("Imported {} file(s)", count)
        }
        Err(e) => format!("Error: Failed to parse session file: {}", e),
    }
}

// Helper to get current directory path as string
fn get_current_dir_string() -> String {
    CURRENT_DIR.with(|cd| cd.borrow().to_string())
}

// Helper to check if a directory exists in virtual filesystem
fn dir_exists(path: &DirPath) -> bool {
    VIRTUAL_FS.with(|vfs| {
        vfs.borrow().dir_exists(path)
    })
}

// Helper to open pretty page in new tab
fn open_pretty_page(file_path: &str, path_arg: &str) -> String {
    let url = format!("./pretty.html?content={}", file_path);

    if let Some(window) = web_sys::window() {
        match window.open_with_url_and_target(&url, "_blank") {
            Ok(_) => format!("Opening {} in new tab...", path_arg),
            Err(_) => "Error: Failed to open new tab. Please check your browser's popup settings.".to_string()
        }
    } else {
        "Error: Could not access window object".to_string()
    }
}

// List files and directories in current directory
fn list_directory(path: &DirPath) -> Vec<String> {
    VIRTUAL_FS.with(|vfs| {
        let vfs_ref = vfs.borrow();
        let mut entries = Vec::new();

        // Add subdirectories with trailing /
        let subdirs = vfs_ref.list_subdirs_in_dir(path);
        for subdir in subdirs {
            entries.push(format!("{}/", subdir));
        }

        // Add files
        entries.extend(vfs_ref.list_files_in_dir(path));

        entries.sort();
        entries
    })
}

/// Main entry point from JavaScript - handles input and manages display
#[wasm_bindgen]
pub async fn handle_input(user_input: &str) {
    let user_input = user_input.trim();

    // Display the input
    add_output(&format!("> {}", user_input));

    // Dispatch based on current handler
    let handler = NEXT_INPUT_HANDLER.with(|h| h.borrow().clone());

    match handler {
        NextInputHandler::None => {
            process_normal_command(user_input).await;
        }
        NextInputHandler::PrettyConfirm { filepath, path_arg } => {
            handle_pretty_confirm(user_input, &filepath, &path_arg);
        }
    }

    // Add blank line after output (except for clear, handled in process_normal_command)
    add_output("&nbsp;");
    scroll_to_bottom();
}

/// Handle confirmation for pretty command
fn handle_pretty_confirm(user_input: &str, filepath: &str, path_arg: &str) {
    let response = if user_input.to_lowercase() == "y" || user_input.to_lowercase() == "yes" {
        open_pretty_page(filepath, path_arg)
    } else {
        "Cancelled.".to_string()
    };

    add_output(&response);

    // Clear handler - return to normal mode
    NEXT_INPUT_HANDLER.with(|h| *h.borrow_mut() = NextInputHandler::None);
}

/// Process a normal command (not a response to a prompt)
async fn process_normal_command(user_input: &str) {
    if user_input.is_empty() {
        // Do nothing for empty command
        return;
    }

    if user_input == "clear" {
        clear_output();
        // Note: Don't add blank line or scroll for clear
        // Early return prevents handle_input from adding &nbsp;
        return;
    }

    let result = process_command(user_input).await;

    // Display output
    if !result.is_empty() {
        for line in result.split('\n') {
            add_output(line);
        }
    }
}

/// Main command processor - handles all non-content commands
/// Add new commands here!
#[wasm_bindgen]
pub async fn process_command(command: &str) -> String {
    let parts: Vec<&str> = command.trim().split_whitespace().collect();

    if parts.is_empty() {
        return String::new();
    }

    match parts[0] {
        // Help command with optional -v flag
        "help" => {
            let filename = if parts.len() > 1 && parts[1] == "-v" {
                "help-verbose.txt".to_string()
            } else {
                "help.txt".to_string()
            };

            let filepath = FilePath::new(DirPath::root(), filename.clone());

            // Check if file exists and get content type
            let content_type = VIRTUAL_FS.with(|vfs| {
                vfs.borrow().get_content(&filepath).cloned()
            });

            match content_type {
                Some(Content::InMemory(content)) => content,
                Some(Content::ToFetch) => {
                    match fetch_text(&filepath.to_url()).await {
                        Ok(content) => content,
                        Err(e) => format!("Error loading {}: {}", filename, e),
                    }
                }
                None => format!("File not found: {}", filename),
            }
        }

        // Other content commands
        "about" | "contact" => {
            let filename = format!("{}.txt", parts[0]);
            let filepath = FilePath::new(DirPath::root(), filename.clone());

            // Check if file exists and get content type
            let content_type = VIRTUAL_FS.with(|vfs| {
                vfs.borrow().get_content(&filepath).cloned()
            });

            match content_type {
                Some(Content::InMemory(content)) => content,
                Some(Content::ToFetch) => {
                    match fetch_text(&filepath.to_url()).await {
                        Ok(content) => content,
                        Err(e) => format!("Error loading {}: {}", filename, e),
                    }
                }
                None => format!("File not found: {}", filename),
            }
        }

        "pwd" => {
            get_current_dir_string()
        }

        "ls" => {
            let target_dir = if parts.len() > 1 {
                // ls with directory argument
                let target = parts[1];

                if target == "/" {
                    DirPath::root()
                } else {
                    // Parse directory path
                    let mut new_path = if target.starts_with('/') {
                        // Absolute path
                        DirPath::root()
                    } else {
                        // Relative path
                        CURRENT_DIR.with(|cd| cd.borrow().clone())
                    };

                    // Process path components
                    for component in target.split('/').filter(|s| !s.is_empty()) {
                        match component {
                            "." => {},
                            ".." => new_path.cd(&NextDir::Out, true),
                            name => new_path.cd(&NextDir::In(name.to_string()), true),
                        }
                    }

                    // Check if directory exists
                    if !dir_exists(&new_path) {
                        return format!("ls: {}: No such directory", target);
                    }

                    new_path
                }
            } else {
                // ls with no arguments - use current directory
                CURRENT_DIR.with(|cd| cd.borrow().clone())
            };

            let entries = list_directory(&target_dir);

            if entries.is_empty() {
                "(empty directory)".to_string()
            } else {
                entries.join("\n")
            }
        }

        "cd" => {
            if parts.len() < 2 {
                // cd with no arguments goes to root
                CURRENT_DIR.with(|cd| {
                    *cd.borrow_mut() = DirPath::root();
                });
                return String::new();
            }

            let target = parts[1];

            // Handle special cases
            if target == "/" {
                CURRENT_DIR.with(|cd| {
                    *cd.borrow_mut() = DirPath::root();
                });
                return String::new();
            }

            // Parse path
            let mut new_path = if target.starts_with('/') {
                // Absolute path
                DirPath::root()
            } else {
                // Relative path
                CURRENT_DIR.with(|cd| cd.borrow().clone())
            };

            // Process path components
            for component in target.split('/').filter(|s| !s.is_empty()) {
                match component {
                    "." => {}, // Stay in current directory
                    ".." => new_path.cd(&NextDir::Out, true),
                    name => new_path.cd(&NextDir::In(name.to_string()), true),
                }
            }

            // Check if directory exists
            if dir_exists(&new_path) {
                CURRENT_DIR.with(|cd| {
                    *cd.borrow_mut() = new_path;
                });
                String::new()
            } else {
                format!("cd: {}: No such directory", target)
            }
        }

        "cat" => {
            if parts.len() < 2 {
                return "Usage: cat <filename>".to_string();
            }

            let path_arg = parts[1];

            // Parse the path (supports both "file.txt" and "path/to/file.txt")
            let filepath = CURRENT_DIR.with(|cd| FilePath::parse(path_arg, &cd.borrow()));

            // Check if file exists and get content type
            let content_type = VIRTUAL_FS.with(|vfs| {
                vfs.borrow().get_content(&filepath).cloned()
            });

            match content_type {
                Some(Content::InMemory(content)) => content,
                Some(Content::ToFetch) => {
                    // Fetch file content from server
                    match fetch_text(&filepath.to_url()).await {
                        Ok(content) => content,
                        Err(e) => format!("cat: Failed to read {}: {}", path_arg, e),
                    }
                }
                None => format!("cat: {}: No such file", path_arg),
            }
        }

        "hello" => {
            "Hello from Rust! This command was processed by WebAssembly.".to_string()
        }

        "info" => {
            "Rust WebAssembly Info:\n\
             - Compiled with wasm-bindgen\n\
             - Running in your browser\n\
             - Fast and efficient!".to_string()
        }

        "fib" => {
            if parts.len() < 2 {
                return "Usage: fib <number>".to_string();
            }

            match parts[1].parse::<u32>() {
                Ok(n) if n <= 93 => {
                    let result = fibonacci(n);
                    format!("fibonacci({}) = {}", n, result)
                }
                Ok(_) => "Please enter a number between 0 and 93".to_string(),
                Err(_) => "Usage: fib <number>".to_string(),
            }
        }

        "secret" => {
            if parts.len() < 2 {
                return "You found my secret hideout, good luck getting in though.".to_string()
            }
            "You will never find my true secrets!".to_string()
        }

        "echo" => {
            if parts.len() < 2 {
                return String::new();
            }
            // Join all parts after 'echo' with spaces
            parts[1..].join(" ")
        }

        "edit" => {
            if parts.len() < 2 {
                return "Usage: edit <filename>".to_string();
            }

            let path_arg = parts[1];
            let filepath = CURRENT_DIR.with(|cd| FilePath::parse(path_arg, &cd.borrow()));

            // Open editor for this file (create new or edit existing)
            let url = format!("./editor.html?file={}", filepath.to_string());

            if let Some(window) = web_sys::window() {
                match window.open_with_url_and_target(&url, "_blank") {
                    Ok(_) => format!("Opening editor for {}...", path_arg),
                    Err(_) => "Error: Failed to open editor. Please check your browser's popup settings.".to_string()
                }
            } else {
                "Error: Could not access window object".to_string()
            }
        }

        "load" => {
            if parts.len() < 2 {
                return "Usage: load <filename>\n\nOpens a file picker to load a file from your device into the virtual filesystem.".to_string();
            }

            let target_filename = parts[1].to_string();

            // Prompt for file picker (returns binary data)
            let file_data = JsFuture::from(prompt_file_picker(".kh,.txt,.md")).await;

            match file_data {
                Ok(data) if !data.is_null() && !data.is_undefined() => {
                    // Convert JsValue to Vec<u8>
                    let uint8_array = Uint8Array::new(&data);
                    let bytes = uint8_array.to_vec();

                    // Interpret as UTF-8 string
                    match String::from_utf8(bytes) {
                        Ok(content) => {
                            // Write to virtual filesystem
                            let filepath = CURRENT_DIR.with(|cd| FilePath::parse(&target_filename, &cd.borrow()));
                            VIRTUAL_FS.with(|vfs| {
                                vfs.borrow_mut().write_file(&filepath, content);
                            });
                            format!("Loaded file into: {}", target_filename)
                        }
                        Err(_) => "Error: File is not valid UTF-8 text".to_string(),
                    }
                }
                _ => "No file selected.".to_string(),
            }
        }

        "save" => {
            if parts.len() < 2 {
                return "Usage: save <filename>\n\nDownloads a file from the virtual filesystem to your device.".to_string();
            }

            let path_arg = parts[1];
            let filepath = CURRENT_DIR.with(|cd| FilePath::parse(path_arg, &cd.borrow()));

            // Get file content
            let content_type = VIRTUAL_FS.with(|vfs| {
                vfs.borrow().get_content(&filepath).cloned()
            });

            match content_type {
                Some(Content::InMemory(content)) => {
                    // Extract just the filename (not the full path) for download
                    let download_name = filepath.file.clone();
                    trigger_download(content.as_bytes(), "text/plain", &download_name);
                    format!("Downloading: {}", path_arg)
                }
                Some(Content::ToFetch) => {
                    // Need to fetch from server first
                    match fetch_text(&filepath.to_url()).await {
                        Ok(content) => {
                            let download_name = filepath.file.clone();
                            trigger_download(content.as_bytes(), "text/plain", &download_name);
                            format!("Downloading: {}", path_arg)
                        }
                        Err(e) => format!("Error fetching file: {}", e),
                    }
                }
                None => format!("save: {}: No such file", path_arg),
            }
        }

        "save-session" => {
            // Get session JSON from WASM
            let session_json = export_session();
            let session: serde_json::Value = serde_json::from_str(&session_json)
                .unwrap_or(serde_json::json!({"files": {}}));

            let file_count = session.get("files")
                .and_then(|f| f.as_object())
                .map(|obj| obj.len())
                .unwrap_or(0);

            if file_count == 0 {
                "No in-memory files to export.".to_string()
            } else {
                // Create filename with timestamp
                let timestamp = Date::new_0().to_iso_string().as_string().unwrap()
                    .replace(":", "-")
                    .replace(".", "-")
                    .chars()
                    .take(19)
                    .collect::<String>();
                let filename = format!("session-{}.json", timestamp);

                trigger_download(session_json.as_bytes(), "application/json", &filename);
                format!("Exported {} file(s) to: {}", file_count, filename)
            }
        }

        "load-session" => {
            // Prompt for file picker (returns binary data)
            let file_data = JsFuture::from(prompt_file_picker(".json")).await;

            match file_data {
                Ok(data) if !data.is_null() && !data.is_undefined() => {
                    // Convert JsValue to Vec<u8>
                    let uint8_array = Uint8Array::new(&data);
                    let bytes = uint8_array.to_vec();

                    // Interpret as UTF-8 string (JSON)
                    match String::from_utf8(bytes) {
                        Ok(session_json) => {
                            let result = import_session(session_json);
                            result
                        }
                        Err(_) => "Error: File is not valid UTF-8 text".to_string(),
                    }
                }
                _ => "No file selected.".to_string(),
            }
        }

        "rm" => {
            if parts.len() < 2 {
                return "Usage: rm <filename>".to_string();
            }

            let path_arg = parts[1];
            let filepath = CURRENT_DIR.with(|cd| FilePath::parse(path_arg, &cd.borrow()));

            VIRTUAL_FS.with(|vfs| {
                if vfs.borrow_mut().remove_file(&filepath) {
                    String::new()
                } else {
                    format!("rm: {}: No such file", path_arg)
                }
            })
        }

        "mkdir" => {
            if parts.len() < 2 {
                return "Usage: mkdir <directory>".to_string();
            }

            let dir_arg = parts[1];

            // Parse directory path
            let new_path = if dir_arg.starts_with('/') {
                // Absolute path
                let mut path = DirPath::root();
                for component in dir_arg.split('/').filter(|s| !s.is_empty()) {
                    match component {
                        "." => {},
                        ".." => path.cd(&NextDir::Out, true),
                        name => path.cd(&NextDir::In(name.to_string()), true),
                    }
                }
                path
            } else {
                // Relative path
                CURRENT_DIR.with(|cd| {
                    let mut path = cd.borrow().clone();
                    for component in dir_arg.split('/').filter(|s| !s.is_empty()) {
                        match component {
                            "." => {},
                            ".." => path.cd(&NextDir::Out, true),
                            name => path.cd(&NextDir::In(name.to_string()), true),
                        }
                    }
                    path
                })
            };

            VIRTUAL_FS.with(|vfs| {
                let mut vfs_mut = vfs.borrow_mut();
                if vfs_mut.dir_exists(&new_path) {
                    format!("mkdir: {}: Directory already exists", dir_arg)
                } else {
                    vfs_mut.create_dir(new_path);
                    String::new()
                }
            })
        }

        "rmdir" => {
            if parts.len() < 2 {
                return "Usage: rmdir <directory>".to_string();
            }

            let dir_arg = parts[1];

            // Parse directory path
            let target_path = if dir_arg.starts_with('/') {
                // Absolute path
                let mut path = DirPath::root();
                for component in dir_arg.split('/').filter(|s| !s.is_empty()) {
                    match component {
                        "." => {},
                        ".." => path.cd(&NextDir::Out, true),
                        name => path.cd(&NextDir::In(name.to_string()), true),
                    }
                }
                path
            } else {
                // Relative path
                CURRENT_DIR.with(|cd| {
                    let mut path = cd.borrow().clone();
                    for component in dir_arg.split('/').filter(|s| !s.is_empty()) {
                        match component {
                            "." => {},
                            ".." => path.cd(&NextDir::Out, true),
                            name => path.cd(&NextDir::In(name.to_string()), true),
                        }
                    }
                    path
                })
            };

            VIRTUAL_FS.with(|vfs| {
                match vfs.borrow_mut().remove_dir(&target_path) {
                    Ok(_) => String::new(),
                    Err(e) => format!("rmdir: {}: {}", dir_arg, e),
                }
            })
        }

        "pretty" => {
            if parts.len() < 2 {
                return "Usage: pretty <filename>".to_string();
            }

            let path_arg = parts[1];
            let filepath = CURRENT_DIR.with(|cd| FilePath::parse(path_arg, &cd.borrow()));

            // Check if file exists
            if !filepath.exists() {
                return format!("pretty: {}: No such file", path_arg);
            }

            // Check if it's a .md file
            let is_markdown = filepath.file.ends_with(".md");

            if is_markdown {
                // Open directly
                open_pretty_page(&filepath.to_string(), path_arg)
            } else {
                // Ask for confirmation - set handler for next input
                add_output(&format!("Warning: '{}' is not a markdown file. Render anyway? (y/n)", path_arg));

                NEXT_INPUT_HANDLER.with(|h| {
                    *h.borrow_mut() = NextInputHandler::PrettyConfirm {
                        filepath: filepath.to_string(),
                        path_arg: path_arg.to_string(),
                    };
                });

                String::new()  // No additional output, prompt already displayed
            }
        }

        // Add more commands here!

        _ => format!("Command not found: {}\nType 'help' for available commands.", command)
    }
}

/// Calculate fibonacci number (helper function)
fn fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => {
            let mut a = 0u64;
            let mut b = 1u64;
            for _ in 2..=n {
                let temp = a + b;
                a = b;
                b = temp;
            }
            b
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_path() {
        let path = DirPath::root();
        assert_eq!(path.to_string(), "/");
    }

    #[test]
    fn test_simple_path() {
        let mut path = DirPath::root();
        path.cd(&NextDir::In("usr".to_string()), true);
        path.cd(&NextDir::In("local".to_string()), true);
        assert_eq!(path.to_string(), "/usr/local");
    }

    #[test]
    fn test_path_with_out_at_root() {
        let mut path = DirPath::root();
        path.cd(&NextDir::In("usr".to_string()), true);
        path.cd(&NextDir::In("local".to_string()), true);
        path.cd(&NextDir::Out, true);
        path.cd(&NextDir::In("bin".to_string()), true);
        assert_eq!(path.to_string(), "/usr/bin");
    }

    #[test]
    fn test_path_with_out_not_at_root() {
        let mut path = DirPath::root();
        path.cd(&NextDir::In("usr".to_string()), false);
        path.cd(&NextDir::In("local".to_string()), false);
        path.cd(&NextDir::Out, false);
        path.cd(&NextDir::In("bin".to_string()), false);
        assert_eq!(path.to_string(), "/usr/bin");
    }

    #[test]
    fn test_normalization_at_root() {
        let path = DirPath(vec![
            NextDir::In("usr".to_string()),
            NextDir::Out,
            NextDir::In("Documents".to_string()),
        ]);
        let normalized = path.normalised(true);
        assert_eq!(normalized.to_string(), "/Documents");
    }

    #[test]
    fn test_normalization_not_at_root() {
        let path = DirPath(vec![
            NextDir::In("usr".to_string()),
            NextDir::Out,
            NextDir::In("Documents".to_string()),
        ]);
        let normalized = path.normalised(false);
        assert_eq!(normalized.to_string(), "/Documents");
    }

    #[test]
    fn test_out_beyond_root_at_root() {
        let mut path = DirPath::root();
        path.cd(&NextDir::Out, true); // Try to go up from root when at_root=true
        assert_eq!(path.to_string(), "/");
    }

    #[test]
    fn test_out_beyond_root_not_at_root() {
        let mut path = DirPath::root();
        path.cd(&NextDir::Out, false); // Try to go up from root when at_root=false
        assert_eq!(path.to_string(), "/..");
    }

    #[test]
    fn test_complex_path() {
        let mut path = DirPath::root();
        path.cd(&NextDir::In("home".to_string()), true);
        path.cd(&NextDir::In("user".to_string()), true);
        path.cd(&NextDir::In("docs".to_string()), true);
        path.cd(&NextDir::Out, true);
        path.cd(&NextDir::Out, true);
        path.cd(&NextDir::In("projects".to_string()), true);
        assert_eq!(path.to_string(), "/home/projects");
    }

    #[test]
    fn test_complex_path_not_at_root() {
        let mut path = DirPath::root();
        path.cd(&NextDir::In("home".to_string()), false);
        path.cd(&NextDir::In("user".to_string()), false);
        path.cd(&NextDir::Out, false);
        path.cd(&NextDir::Out, false);
        path.cd(&NextDir::Out, false);
        path.cd(&NextDir::In("projects".to_string()), false);
        assert_eq!(path.to_string(), "/../projects");
    }

    #[test]
    fn test_concat_simple() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("home".to_string()), true);
        base.cd(&NextDir::In("user".to_string()), true);

        let mut relative = DirPath::root();
        relative.cd(&NextDir::In("documents".to_string()), false);

        let result = base.concat(&relative, true);
        assert_eq!(result.to_string(), "/home/user/documents");
    }

    #[test]
    fn test_concat_with_out() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("home".to_string()), true);
        base.cd(&NextDir::In("user".to_string()), true);
        base.cd(&NextDir::In("projects".to_string()), true);

        let mut relative = DirPath::root();
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::In("documents".to_string()), false);

        let result = base.concat(&relative, true);
        assert_eq!(result.to_string(), "/home/user/documents");
    }

    #[test]
    fn test_concat_multiple_out_at_root() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("usr".to_string()), true);

        let mut relative = DirPath::root();
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::In("home".to_string()), false);

        let result = base.concat(&relative, true);
        assert_eq!(result.to_string(), "/home");
    }

    #[test]
    fn test_concat_multiple_out_not_at_root() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("usr".to_string()), false);

        let mut relative = DirPath::root();
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::In("home".to_string()), false);

        let result = base.concat(&relative, false);
        assert_eq!(result.to_string(), "/../home");
    }

    #[test]
    fn test_concat_empty_relative() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("home".to_string()), true);

        let relative = DirPath::root();

        let result = base.concat(&relative, true);
        assert_eq!(result.to_string(), "/home");
    }

    #[test]
    fn test_concat_complex_path() {
        let mut base = DirPath::root();
        base.cd(&NextDir::In("home".to_string()), true);
        base.cd(&NextDir::In("user".to_string()), true);
        base.cd(&NextDir::In("projects".to_string()), true);
        base.cd(&NextDir::In("rust".to_string()), true);

        let mut relative = DirPath::root();
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::Out, false);
        relative.cd(&NextDir::In("documents".to_string()), false);
        relative.cd(&NextDir::In("notes.txt".to_string()), false);

        let result = base.concat(&relative, true);
        assert_eq!(result.to_string(), "/home/user/documents/notes.txt");
    }
}
