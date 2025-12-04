use crate::commands::{Command, CommandData};
use crate::filesystem::{DirPath, FilePath, CURRENT_DIR, VIRTUAL_FS, ABYSS_FS, Contents, Directories, NextDir};
use crate::filesystem::helpers::{get_current_dir_string, get_file_content, dir_exists, list_directory, path_in_abyss};

pub struct Pwd;
impl CommandData for Pwd {
    fn name(&self) -> &str { "pwd" }
}
impl Command for Pwd {
    async fn execute(&self, _args: &[&str]) -> String {
        get_current_dir_string()
    }
}

pub struct Ls;
impl CommandData for Ls {
    fn name(&self) -> &str { "ls" }
}
impl Command for Ls {
    async fn execute(&self, args: &[&str]) -> String {
        let target_dir = if let Some(&target) = args.get(0) {
            // ls with directory argument
            let new_path = CURRENT_DIR.with(|cd| DirPath::parse(target, &cd.borrow()));

            // Check if directory exists
            if !dir_exists(&new_path).await {
                return format!("ls: {}: No such directory", target);
            }

            new_path
        } else {
            // ls with no arguments - use current directory
            CURRENT_DIR.with(|cd| cd.borrow().clone())
        };

        let entries = list_directory(&target_dir).await;

        if entries.is_empty() {
            "(empty directory)".to_string()
        } else {
            entries.join("\n")
        }
    }
}

pub struct Cd;
impl CommandData for Cd {
    fn name(&self) -> &str { "cd" }
}
impl Command for Cd {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            // cd with no arguments goes to root
            CURRENT_DIR.with(|cd| {
                *cd.borrow_mut() = DirPath::root();
            });
            return String::new();
        }

        let target = args[0];
        let new_path = CURRENT_DIR.with(|cd| DirPath::parse(target, &cd.borrow()));

        // Check if directory exists
        if dir_exists(&new_path).await {
            CURRENT_DIR.with(|cd| {
                *cd.borrow_mut() = new_path;
            });
            String::new()
        } else {
            format!("cd: {}: No such directory", target)
        }
    }
}

pub struct Cat;
impl CommandData for Cat {
    fn name(&self) -> &str { "cat" }
}
impl Command for Cat {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: cat <filename>".to_string();
        }

        let path_arg = args[0];
        let filepath = CURRENT_DIR.with(|cd| FilePath::parse(path_arg, &cd.borrow()));

        match get_file_content(&filepath).await {
            Ok(content) => content,
            Err(_) => format!("cat: {}: No such file", path_arg),
        }
    }
}

pub struct Rm;
impl CommandData for Rm {
    fn name(&self) -> &str { "rm" }
}
impl Command for Rm {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: rm <filename>".to_string();
        }

        let path_arg = args[0];
        let filepath = CURRENT_DIR.with(|cd| FilePath::parse(path_arg, &cd.borrow()));

        if path_in_abyss(&filepath.dir) {
            // Handle abyss files
            // TODO: get_or_fetch_contents, modify, cache back
            todo!("Implement rm for abyss files")
        } else {
            // Handle regular virtual filesystem
            VIRTUAL_FS.with(|vfs| {
                if vfs.borrow_mut().remove_file(&filepath) {
                    String::new()
                } else {
                    format!("rm: {}: No such file", path_arg)
                }
            })
        }
    }
}

pub struct Mkdir;
impl CommandData for Mkdir {
    fn name(&self) -> &str { "mkdir" }
}
impl Command for Mkdir {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: mkdir <directory>".to_string();
        }

        let dir_arg = args[0];
        let new_path = CURRENT_DIR.with(|cd| DirPath::parse(dir_arg, &cd.borrow()));

        // Check if directory already exists
        if dir_exists(&new_path).await {
            return format!("mkdir: {}: Directory already exists", dir_arg);
        }

        if path_in_abyss(&new_path) {
            // Handle abyss directories

            // Unnecessary: Abyss directories start with /abyss/
            let path_vec = &new_path.0;
            if path_vec.is_empty() {
                return format!("mkdir: Invalid path");
            }

            // Get parent path and directory name
            let parent = DirPath(path_vec[..path_vec.len()-1].to_vec());
            let dir_name = match path_vec.last() {
                Some(NextDir::In(name)) => name.clone(),
                _ => return format!("mkdir: Invalid path"),
            };

            ABYSS_FS.with(|afs| {
                let mut afs_mut = afs.borrow_mut();

                // Add to parent's directories

                // assumes parent directory exists without checking 
                // (should only work if it does exist)

                // Might not work for the directory /abyss/
                // if the user has deleted the full abyss.
                // Need to consider the details of how ABYSS_FS 
                // is structured at root:
                // Parent of /abyss/ is / which is not in abyss, 
                // so no entry in ABYSS_FS.
                let mut parent_dirs = afs_mut.dirs.get(&parent).cloned().unwrap_or_else(|| Directories::new());
                parent_dirs.0.insert(dir_name);
                afs_mut.dirs.insert(parent.clone(), parent_dirs);

                // Initialize empty Contents and Directories for new directory
                afs_mut.files.insert(new_path.clone(), Contents::new());
                afs_mut.dirs.insert(new_path, Directories::new());
            });

            String::new()
        } else {
            // Handle regular virtual filesystem
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
    }
}

pub struct Rmdir;
impl CommandData for Rmdir {
    fn name(&self) -> &str { "rmdir" }
}
impl Command for Rmdir {
    async fn execute(&self, args: &[&str]) -> String {
        if args.is_empty() {
            return "Usage: rmdir <directory>".to_string();
        }

        let dir_arg = args[0];
        let target_path = CURRENT_DIR.with(|cd| DirPath::parse(dir_arg, &cd.borrow()));

        if path_in_abyss(&target_path) {
            // Handle abyss directories
            // TODO: get_or_fetch contents/directories, check empty, remove from parent, remove entries
            todo!("Implement rmdir for abyss directories")
        } else {
            // Handle regular virtual filesystem
            VIRTUAL_FS.with(|vfs| {
                match vfs.borrow_mut().remove_dir(&target_path) {
                    Ok(_) => String::new(),
                    Err(e) => format!("rmdir: {}: {}", dir_arg, e),
                }
            })
        }
    }
}
