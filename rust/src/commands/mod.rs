use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use js_sys::{Uint8Array, Date};
use crate::filesystem::{DirPath, FilePath, CURRENT_DIR, VIRTUAL_FS};
use crate::filesystem::helpers::{get_file_content, get_current_dir_string, dir_exists, list_directory};
use crate::js_interop::{add_output, prompt_file_picker, trigger_download};

// Stub modules for future command organization
pub mod builtin;

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

// Export session helper (used by save-session command)
fn export_session() -> String {
    use serde_json::json;

    VIRTUAL_FS.with(|vfs| {
        let vfs_ref = vfs.borrow();
        let mut files = serde_json::Map::new();

        // Collect all InMemory files
        for (dirpath, dir_contents) in &vfs_ref.content {
            for (filename, content) in dir_contents {
                if let crate::filesystem::Content::InMemory(file_content) = content {
                    let mut path_parts = Vec::new();
                    for component in &dirpath.0 {
                        match component {
                            crate::filesystem::NextDir::In(name) => path_parts.push(name.clone()),
                            crate::filesystem::NextDir::Out => path_parts.push("..".to_string()),
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

// Import session helper (used by load-session command)
fn import_session(session_json: String) -> String {
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
                "help-verbose.txt"
            } else {
                "help.txt"
            };

            let filepath = FilePath::new(DirPath::root(), filename.to_string());

            match get_file_content(&filepath).await {
                Ok(content) => content,
                Err(e) => format!("Error loading {}: {}", filename, e),
            }
        }

        // Other content commands
        "about" | "contact" => {
            let filename = format!("{}.txt", parts[0]);
            let filepath = FilePath::new(DirPath::root(), filename.clone());

            match get_file_content(&filepath).await {
                Ok(content) => content,
                Err(e) => format!("Error loading {}: {}", filename, e),
            }
        }

        "pwd" => {
            get_current_dir_string()
        }

        "ls" => {
            let target_dir = if parts.len() > 1 {
                // ls with directory argument
                let target = parts[1];
                let new_path = CURRENT_DIR.with(|cd| DirPath::parse(target, &cd.borrow()));

                // Check if directory exists
                if !dir_exists(&new_path) {
                    return format!("ls: {}: No such directory", target);
                }

                new_path
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
            let new_path = CURRENT_DIR.with(|cd| DirPath::parse(target, &cd.borrow()));

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
            let filepath = CURRENT_DIR.with(|cd| FilePath::parse(path_arg, &cd.borrow()));

            match get_file_content(&filepath).await {
                Ok(content) => content,
                Err(_) => format!("cat: {}: No such file", path_arg),
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

            match get_file_content(&filepath).await {
                Ok(content) => {
                    let download_name = filepath.file.clone();
                    trigger_download(content.as_bytes(), "text/plain", &download_name);
                    format!("Downloading: {}", path_arg)
                }
                Err(_) => format!("save: {}: No such file", path_arg),
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
            let new_path = CURRENT_DIR.with(|cd| DirPath::parse(dir_arg, &cd.borrow()));

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
            let target_path = CURRENT_DIR.with(|cd| DirPath::parse(dir_arg, &cd.borrow()));

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

                crate::NEXT_INPUT_HANDLER.with(|h| {
                    *h.borrow_mut() = crate::NextInputHandler::PrettyConfirm {
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
