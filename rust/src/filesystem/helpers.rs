use rand::random_range;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, console::log_1};
use crate::filesystem::cave_of_dice::path_in_cave_of_dice;
use crate::filesystem::{ABYSS_FS, CURRENT_DIR, Contents, Directories, NextDir};

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

fn is_dice_file_name(file_name: &str) -> Option<u8> {
    if file_name.chars().nth(0) == Some('d') {
        match file_name[1..].split('.').collect::<Vec<_>>().as_slice() {
            [x, "txt"] => x.parse().ok(),
            _ => None
        }
    } else {
        None
    }
}

// Read content from a Content variant
async fn read_content_at(content: Option<&Content>, filepath: &FilePath) -> Result<String, String> {
    match content {
        Some(Content::InMemory(text)) => Ok(text.clone()),
        Some(Content::ToFetch) => {
            if path_in_cave_of_dice(&filepath.dir) && let Some(n) = is_dice_file_name(&filepath.file) {
                Ok(format!("You rolled a {}", random_range(1..=n)))
            } else {
                fetch_text(&filepath.to_url()).await
            }
        },
        None => Err(format!("{}: No such file", filepath.to_string())),
    }
}

// Get file content (fetch if needed)
pub async fn get_file_content(filepath: &FilePath) -> Result<String, String> {
    let contents = get_contents(&filepath.dir).await;
    read_content_at(contents.get(&filepath.file), filepath).await
}

// Helper to get current directory path as string
pub fn get_current_dir_string() -> String {
    super::CURRENT_DIR.with(|cd| cd.borrow().to_string())
}

// Helper to check if a directory exists in virtual filesystem or abyss
pub async fn dir_exists(path: &DirPath) -> bool {
    // Root always exists
    if path.0.is_empty() {
        return true;
    }

    // Get parent directory and final component
    match (path.super_dir(), path.final_component()) {
        (Some(parent), Some(dirname)) => {
            let directories = get_directories(&parent).await;
            directories.contains(dirname)
        }
        _ => false, // Invalid path structure
    }
}

// Helper to check if a file exists in virtual filesystem or abyss
pub async fn file_exists(filepath: &FilePath) -> bool {
    let contents = get_contents(&filepath.dir).await;
    contents.contains(&filepath.file)
}

// List files and directories in current directory
pub async fn list_directory(path: &DirPath) -> Vec<String> {
    let mut entries = Vec::new();

    // Get directories and add with trailing /
    let directories = get_directories(path).await;
    for dir in &directories.0 {
        entries.push(format!("{}/", dir));
    }

    // Get files
    let contents = get_contents(path).await;
    for filename in contents.0.keys() {
        entries.push(filename.clone());
    }

    entries.sort();
    entries
}

pub fn in_abyss() -> bool {
    CURRENT_DIR.with(|dir|
        path_in_abyss(&dir.borrow())
    )
}

pub fn path_in_abyss(path: &DirPath) -> bool {
    match path.0.first() {
        Some(NextDir::In(x)) if x == "abyss" => true,
        _ => false
    }
}

// Async helpers for abyss write operations

/// Remove a file from the abyss filesystem
pub async fn remove_file_abyss(filepath: &FilePath) -> Result<(), String> {
    path_in_cave_of_dice(&filepath.dir); // Initialize cave_of_dice if needed
    // Try cached path first
    match ABYSS_FS.with_borrow_mut(|afs| afs.sync_remove_file(filepath)) {
        Ok(_) => Ok(()),
        Err(_) => {
            // Fetch and retry with data
            let contents = get_contents(&filepath.dir).await;
            ABYSS_FS.with_borrow_mut(|afs|
                afs.sync_remove_file_with_data(filepath, contents)
            )
        }
    }
}

/// Remove a directory from the abyss filesystem
pub async fn remove_dir_abyss(dirpath: &DirPath) -> Result<(), String> {
    path_in_cave_of_dice(dirpath); // Initialize cave_of_dice if needed
    // Try cached path first
    match ABYSS_FS.with_borrow_mut(|afs| afs.sync_remove_dir(dirpath)) {
        Ok(_) => Ok(()),
        Err(_) => {
            // Fetch all needed data
            let contents = get_contents(dirpath).await;
            let directories = get_directories(dirpath).await;
            let parent = dirpath.super_dir().ok_or("Invalid path")?;
            let parent_dirs = get_directories(&parent).await;

            // Retry with data
            ABYSS_FS.with_borrow_mut(|afs|
                afs.sync_remove_dir_with_data(dirpath, contents, directories, parent_dirs)
            )
        }
    }
}

/// Create a directory in the abyss filesystem
pub async fn create_dir_abyss(dirpath: &DirPath) -> Result<(), String> {
    path_in_cave_of_dice(dirpath); // Initialize cave_of_dice if needed
    // Try cached path first
    match ABYSS_FS.with_borrow_mut(|afs| afs.sync_create_dir(dirpath)) {
        Ok(_) => Ok(()),
        Err(_) => {
            // Fetch parent directories
            let parent = dirpath.super_dir().ok_or("Invalid path")?;
            let parent_dirs = get_directories(&parent).await;

            // Retry with data
            ABYSS_FS.with_borrow_mut(|afs|
                afs.sync_create_dir_with_data(dirpath, parent_dirs)
            )
        }
    }
}

/// Write a file to the abyss filesystem
pub async fn write_file_abyss(filepath: &FilePath, content: String) {
    path_in_cave_of_dice(&filepath.dir); // Initialize cave_of_dice if needed
    // Try cached path first
    match ABYSS_FS.with_borrow_mut(|afs| afs.sync_write_file(filepath, content.clone())) {
        Ok(_) => {},
        Err(_) => {
            // Fetch contents
            let contents = get_contents(&filepath.dir).await;

            // Write with data
            ABYSS_FS.with_borrow_mut(|afs|
                afs.sync_write_file_with_data(filepath, contents, content)
            );
        }
    }
}

// assumes path is valid
pub async fn get_directories(path: &DirPath) -> Directories {
    if path_in_abyss(path) {
        path_in_cave_of_dice(path); // Initialize cave_of_dice if needed
        let msg = format!("{} is in abyss", path.to_string());
        log_1(&msg.into());

        match ABYSS_FS.with_borrow(|afs|
            afs.dirs.get(path).cloned()
        ) {
            Some(x) => x,
            None => Directories::from_file(
                &fetch_text(
                    &format!("content{}/!!directories.txt", path.to_string())
                ).await.unwrap()
            )
        }
    } else {
        let msg = format!("{} is not in abyss", path.to_string());
        log_1(&msg.into());
        Directories(
            VIRTUAL_FS
            .with_borrow(|vfs| vfs.list_subdirs_in_dir(path))
            .iter()
            .cloned()
            .collect()
        )
    }
}

// Assumes path is valid
pub async fn get_contents(path: &DirPath) -> Contents {
    if path_in_abyss(path) {
        path_in_cave_of_dice(path); // Initialize cave_of_dice if needed
        match ABYSS_FS.with_borrow(|afs|
            afs.files.get(path).cloned()
        ) {
            Some(x) => x,
            None => Contents::from_file(
                &fetch_text(
                    &format!("content{}/!!contents.txt", path.to_string())
                ).await.unwrap()
            )
        }
    } else {
        Contents(
            VIRTUAL_FS
            .with_borrow(|vfs| vfs.list_files_in_dir(path))
            .iter()
            .map(|file|
                VIRTUAL_FS.with_borrow(|vfs|
                    (
                        file.clone(),
                        vfs.get_content(
                            &FilePath {dir: path.clone(), file: file.clone()}
                        )
                        .cloned()
                        .unwrap()
                    )
                )
            )
            .collect()
        )
    }
}
