use rand::rngs::ThreadRng;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use std::cell::RefCell;

mod js_interop;
mod filesystem;
mod channels;
mod commands;
mod input_history;

use js_interop::{add_output, clear_output, scroll_to_bottom};
use filesystem::{Manifest, DirPath, FilePath, VIRTUAL_FS};
use filesystem::helpers::fetch_text;
use channels::{handle_editor_message, handle_pretty_message, EDITOR_CHANNEL, PRETTY_CHANNEL};
use commands::process_command;
use commands::builtin::pretty::open_pretty_page;
use input_history::INPUT_HISTORY;

// Handler for next input - determines what function receives the next user input
#[derive(Clone)]
enum NextInputHandler {
    None,
    PrettyConfirm { filepath: String, path_arg: String },
}

thread_local! {
    static NEXT_INPUT_HANDLER: RefCell<NextInputHandler> = RefCell::new(NextInputHandler::None);
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
    use web_sys::{BroadcastChannel, MessageEvent};

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

// Write a file to the virtual filesystem (called from JavaScript/editor)
#[wasm_bindgen]
pub fn write_file(path: &str, content: String) -> Result<(), JsValue> {
    let filepath = filesystem::CURRENT_DIR.with(|cd| FilePath::parse(path, &cd.borrow()));

    VIRTUAL_FS.with(|vfs| {
        vfs.borrow_mut().write_file(&filepath, content);
    });

    Ok(())
}

// Read a file from the virtual filesystem (called from JavaScript)
// Returns the content type: "InMemory:<content>", "ToFetch:<url>", or "NotFound"
#[wasm_bindgen]
pub fn read_file(path: &str) -> String {
    let filepath = filesystem::CURRENT_DIR.with(|cd| FilePath::parse(path, &cd.borrow()));

    VIRTUAL_FS.with(|vfs| {
        match vfs.borrow().get_content(&filepath) {
            Some(filesystem::Content::InMemory(content)) => format!("InMemory:{}", content),
            Some(filesystem::Content::ToFetch) => format!("ToFetch:{}", filepath.to_url()),
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
                if let filesystem::Content::InMemory(file_content) = content {
                    let mut path_parts = Vec::new();
                    for component in &dirpath.0 {
                        match component {
                            filesystem::NextDir::In(name) => path_parts.push(name.clone()),
                            filesystem::NextDir::Out => path_parts.push("..".to_string()),
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

/// Handle arrow up key - returns previous input from history, or empty string if at beginning
#[wasm_bindgen]
pub fn handle_arrow_up() -> String {
    INPUT_HISTORY.with(|history| {
        history.borrow_mut().arrow_up().unwrap_or_default()
    })
}

/// Handle arrow down key - returns next input from history, or empty string if at end
#[wasm_bindgen]
pub fn handle_arrow_down() -> String {
    INPUT_HISTORY.with(|history| {
        history.borrow_mut().arrow_down().unwrap_or_default()
    })
}

/// Main entry point from JavaScript - handles input and manages display
#[wasm_bindgen]
pub async fn handle_input(user_input: &str) {
    let user_input = user_input.trim();

    // Add to history (skips empty inputs internally)
    INPUT_HISTORY.with(|history| {
        history.borrow_mut().add_input(user_input.to_string());
    });

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
        return;
    }

    let result = process_command(user_input).await;

    // Display output
    if !result.is_empty() {
        for line in result.lines() {
            add_output(line);
        }
    }
}
