use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{BroadcastChannel, MessageEvent};
use std::cell::RefCell;
use crate::filesystem::{DirPath, FilePath, VIRTUAL_FS};
use crate::filesystem::helpers::{get_file_content, path_in_abyss, write_file_abyss};
use crate::js_interop::add_output;

thread_local! {
    pub static EDITOR_CHANNEL: RefCell<Option<BroadcastChannel>> = RefCell::new(None);
    pub static PRETTY_CHANNEL: RefCell<Option<BroadcastChannel>> = RefCell::new(None);
}

// Handle messages from editor
pub fn handle_editor_message(event: MessageEvent) {
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
                            // Write file to virtual filesystem
                            let filepath = FilePath::parse(&filename_str, &DirPath::root());

                            // Spawn async task to handle both abyss and regular files
                            wasm_bindgen_futures::spawn_local(async move {
                                if path_in_abyss(&filepath.dir) {
                                    // Handle abyss files
                                    write_file_abyss(&filepath, content_str).await;
                                } else {
                                    // Handle regular virtual filesystem
                                    VIRTUAL_FS.with(|vfs| {
                                        vfs.borrow_mut().write_file(&filepath, content_str);
                                    });
                                }

                                add_output(&format!("File saved: {}", filename_str));
                                add_output("\u{00A0}");
                            });
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
pub fn handle_pretty_message(event: MessageEvent) {
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

// Build a file_content message for BroadcastChannel
fn build_file_content_message(filename: &str, content: &str) -> js_sys::Object {
    let message = js_sys::Object::new();
    js_sys::Reflect::set(&message, &JsValue::from_str("action"), &JsValue::from_str("file_content")).ok();
    js_sys::Reflect::set(&message, &JsValue::from_str("filename"), &JsValue::from_str(filename)).ok();
    js_sys::Reflect::set(&message, &JsValue::from_str("content"), &JsValue::from_str(content)).ok();
    message
}

// Send file content via BroadcastChannel
fn send_file_content(filename: &str, to_editor: bool) {
    let filepath = FilePath::parse(filename, &DirPath::root());
    let filename = filename.to_string();

    let channel = if to_editor {
        EDITOR_CHANNEL.with(|ch| ch.borrow().clone())
    } else {
        PRETTY_CHANNEL.with(|ch| ch.borrow().clone())
    };

    if let Some(channel) = channel {
        // Spawn async task to get file content (works for both abyss and regular files)
        wasm_bindgen_futures::spawn_local(async move {
            match get_file_content(&filepath).await {
                Ok(content) => {
                    let message = build_file_content_message(&filename, &content);
                    channel.post_message(&message).ok();
                }
                Err(_) => {
                    // File not found, send empty content
                    let message = build_file_content_message(&filename, "");
                    channel.post_message(&message).ok();
                }
            }
        });
    }
}
