use crate::commands::{Command, CommandData, export_session, import_session};
use crate::filesystem::{CURRENT_DIR, FilePath, VIRTUAL_FS};
use crate::filesystem::helpers::get_file_content;
use crate::js_interop::{prompt_file_picker, trigger_download};
use wasm_bindgen_futures::JsFuture;
use js_sys::{Uint8Array, Date};

pub struct Edit;
impl CommandData for Edit {
    fn name(&self) -> &str { "edit" }
}
impl Command for Edit {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: edit <filename>".to_string();
        }

        let path_arg = args[0];
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
}

pub struct Load;
impl CommandData for Load {
    fn name(&self) -> &str { "load" }
}
impl Command for Load {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: load <filename>\n\nOpens a file picker to load a file from your device into the virtual filesystem.".to_string();
        }

        let target_filename = args[0].to_string();

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
}

pub struct Save;
impl CommandData for Save {
    fn name(&self) -> &str { "save" }
}
impl Command for Save {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: save <filename>\n\nDownloads a file from the virtual filesystem to your device.".to_string();
        }

        let path_arg = args[0];
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
}

pub struct SaveSession;
impl CommandData for SaveSession {
    fn name(&self) -> &str { "save-session" }
}
impl Command for SaveSession {
    async fn execute(&self, _args: &[&str]) -> String {
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
}

pub struct LoadSession;
impl CommandData for LoadSession {
    fn name(&self) -> &str { "load-session" }
}
impl Command for LoadSession {
    async fn execute(&self, _args: &[&str]) -> String {
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
}
