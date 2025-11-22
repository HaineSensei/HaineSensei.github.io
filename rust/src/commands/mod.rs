use wasm_bindgen::prelude::*;
use crate::filesystem::file_paths::{SIMPLE_MANUAL_DIR_PATH, VERBOSE_MANUAL_DIR_PATH};
use crate::filesystem::{DirPath, FilePath, VIRTUAL_FS};

// Command implementations organized by type
pub mod builtin;
use builtin::*;


pub trait CommandData {
    fn name(&self) -> &str;

    fn manual(&self, verbose: bool) -> FilePath {
        FilePath::new(
            (*if verbose { &VERBOSE_MANUAL_DIR_PATH } else { &SIMPLE_MANUAL_DIR_PATH }).clone(),
            format!("{}.txt",self.name())
        )
    }
}

// Command trait - all commands implement this
pub trait Command : CommandData {
    async fn execute(&self, args: &[&str]) -> String;
}

// User-defined command (for future .kh file system)
pub struct UserDefined(String);

impl CommandData for UserDefined {
    fn name(&self) -> &str {
        &self.0
    }
}

// Export session helper (used by save-session command)
pub fn export_session() -> String {
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

/// Main command processor - handles all non-content commands
/// Add new commands here!
#[wasm_bindgen]
pub async fn process_command(command: &str) -> String {
    let parts: Vec<&str> = command.trim().split_whitespace().collect();

    if parts.is_empty() {
        return String::new();
    }

    match parts[0] {
        "help" => Help.execute(&parts[1..]).await,
        "about" => About.execute(&parts[1..]).await,
        "contact" => Contact.execute(&parts[1..]).await,

        "pwd" => Pwd.execute(&parts[1..]).await,

        "ls" => Ls.execute(&parts[1..]).await,
        "cd" => Cd.execute(&parts[1..]).await,
        "cat" => Cat.execute(&parts[1..]).await,

        "hello" => Hello.execute(&parts[1..]).await,

        "info" => Info.execute(&parts[1..]).await,

        "fib" => Fib.execute(&parts[1..]).await,
        "secret" => Secret.execute(&parts[1..]).await,

        "echo" => Echo.execute(&parts[1..]).await,

        "edit" => Edit.execute(&parts[1..]).await,
        "load" => Load.execute(&parts[1..]).await,
        "save" => Save.execute(&parts[1..]).await,
        "save-session" => SaveSession.execute(&parts[1..]).await,
        "load-session" => LoadSession.execute(&parts[1..]).await,

        "rm" => Rm.execute(&parts[1..]).await,
        "mkdir" => Mkdir.execute(&parts[1..]).await,
        "rmdir" => Rmdir.execute(&parts[1..]).await,

        "pretty" => Pretty.execute(&parts[1..]).await,

        // Add more commands here!

        _ => format!("Command not found: {}\nType 'help' for available commands.", command)
    }
}

pub fn command_data(name: &str) -> Box<dyn CommandData> {
    match name {
        "help" => Box::new(Help),

        // Other content commands
        "about" => Box::new(About),
        "contact" => Box::new(Contact),

        "pwd" => Box::new(Pwd),

        "ls" => Box::new(Ls),

        "cd" => Box::new(Cd),

        "cat" => Box::new(Cat),

        "hello" => Box::new(Hello),

        "info" => Box::new(Info),

        "fib" => Box::new(Fib),

        "secret" => Box::new(Secret),

        "echo" => Box::new(Echo),

        "edit" => Box::new(Edit),

        "load" => Box::new(Load),

        "save" => Box::new(Save),

        "save-session" => Box::new(SaveSession),

        "load-session" => Box::new(LoadSession),

        "rm" => Box::new(Rm),

        "mkdir" => Box::new(Mkdir),

        "rmdir" => Box::new(Rmdir),

        "pretty" => Box::new(Pretty),

        // Add more commands here!

        x => Box::new(UserDefined(x.to_string()))
    }
}
