use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
use super::types::{DirPath, FilePath, Content};
use super::VIRTUAL_FS;

// Async fetch helper
pub async fn fetch_text(url: &str) -> Result<String, String> {
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

// Get file content (fetch if needed)
pub async fn get_file_content(filepath: &FilePath) -> Result<String, String> {
    let content_type = VIRTUAL_FS.with(|vfs| {
        vfs.borrow().get_content(filepath).cloned()
    });

    match content_type {
        Some(Content::InMemory(content)) => Ok(content),
        Some(Content::ToFetch) => {
            fetch_text(&filepath.to_url()).await
        }
        None => Err(format!("{}: No such file", filepath.to_string())),
    }
}

// Helper to get current directory path as string
pub fn get_current_dir_string() -> String {
    super::CURRENT_DIR.with(|cd| cd.borrow().to_string())
}

// Helper to check if a directory exists in virtual filesystem
pub fn dir_exists(path: &DirPath) -> bool {
    VIRTUAL_FS.with(|vfs| {
        vfs.borrow().dir_exists(path)
    })
}

// List files and directories in current directory
pub fn list_directory(path: &DirPath) -> Vec<String> {
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
