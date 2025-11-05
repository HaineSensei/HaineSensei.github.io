use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};
use std::cell::RefCell;
use serde::Deserialize;

thread_local! {
    static CURRENT_DIR: RefCell<DirPath> = RefCell::new(DirPath::root());
    static MANIFEST: RefCell<Option<Manifest>> = RefCell::new(None);
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

#[derive(Clone)]
enum NextDir {
    In(String),
    Out
}

// DirPath([In("usr"),Out,In("Documents")]) interpreted as /usr/../Documents
#[derive(Clone)]
struct DirPath(Vec<NextDir>);

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

    // Check if this file exists in the manifest
    fn exists(&self) -> bool {
        let path_str = self.dir.to_string();
        let normalized = if path_str == "/" {
            ""
        } else {
            path_str.trim_start_matches('/')
        };

        MANIFEST.with(|m| {
            if let Some(ref manifest) = *m.borrow() {
                manifest.files.iter().any(|f| f.path == normalized && f.name == self.file)
            } else {
                false
            }
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

// Load manifest from server
#[wasm_bindgen]
pub async fn load_manifest() -> Result<(), JsValue> {
    let manifest_text = fetch_text("./content/manifest.json")
        .await
        .map_err(|e| JsValue::from_str(&e))?;

    let manifest: Manifest = serde_json::from_str(&manifest_text)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse manifest: {}", e)))?;

    MANIFEST.with(|m| {
        *m.borrow_mut() = Some(manifest);
    });

    Ok(())
}

// Helper to get current directory path as string
fn get_current_dir_string() -> String {
    CURRENT_DIR.with(|cd| cd.borrow().to_string())
}

// Helper to check if a directory exists in manifest
fn dir_exists(path: &DirPath) -> bool {
    let path_str = path.to_string();

    if path_str == "/" {
        return true; // Root always exists
    }

    // Remove leading slash for comparison
    let normalized = path_str.trim_start_matches('/');

    MANIFEST.with(|m| {
        if let Some(ref manifest) = *m.borrow() {
            manifest.directories.iter().any(|d| d == normalized)
        } else {
            false
        }
    })
}

// List files and directories in current directory
fn list_directory(path: &DirPath) -> Vec<String> {
    let path_str = path.to_string();
    let normalized = if path_str == "/" {
        ""
    } else {
        path_str.trim_start_matches('/')
    };

    MANIFEST.with(|m| {
        if let Some(ref manifest) = *m.borrow() {
            let mut entries = Vec::new();

            // Add subdirectories
            for dir in &manifest.directories {
                if normalized.is_empty() {
                    // At root - show top-level directories only
                    if !dir.contains('/') {
                        entries.push(format!("{}/", dir));
                    }
                } else {
                    // Show direct children only
                    if dir.starts_with(normalized) && dir != normalized {
                        let remainder = dir[normalized.len()..].trim_start_matches('/');
                        if !remainder.contains('/') {
                            entries.push(format!("{}/", remainder));
                        }
                    }
                }
            }

            // Add files in current directory
            for file in &manifest.files {
                if file.path == normalized {
                    entries.push(file.name.clone());
                }
            }

            entries.sort();
            entries.dedup();
            entries
        } else {
            vec![]
        }
    })
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
        // Legacy content commands - redirect to cat
        "help" | "about" | "contact" => {
            let filename = format!("{}.txt", parts[0]);
            let filepath = FilePath::new(DirPath::root(), filename.clone());

            if filepath.exists() {
                match fetch_text(&filepath.to_url()).await {
                    Ok(content) => content,
                    Err(e) => format!("Error loading {}: {}", filename, e),
                }
            } else {
                format!("File not found: {}", filename)
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

            // Check if file exists
            if !filepath.exists() {
                return format!("cat: {}: No such file", path_arg);
            }

            // Fetch file content
            match fetch_text(&filepath.to_url()).await {
                Ok(content) => content,
                Err(e) => format!("cat: Failed to read {}: {}", path_arg, e),
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
