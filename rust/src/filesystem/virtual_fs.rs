use std::collections::HashMap;
use super::types::{DirPath, FilePath, Content, Manifest, NextDir};

/// Virtual filesystem stored in WASM memory
pub struct VirtualFilesystem {
    pub content: HashMap<DirPath, HashMap<String, Content>>,
}

impl VirtualFilesystem {
    pub fn new() -> Self {
        Self {
            content: HashMap::new(),
        }
    }

    /// Initialize from manifest - loads all static files as ToFetch
    pub fn initialize_from_manifest(&mut self, manifest: &Manifest) {
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
    pub fn write_file(&mut self, filepath: &FilePath, content: String) {
        self.content
            .entry(filepath.dir.clone())
            .or_insert_with(HashMap::new)
            .insert(filepath.file.clone(), Content::InMemory(content));
    }

    /// Get content type for a file
    pub fn get_content(&self, filepath: &FilePath) -> Option<&Content> {
        self.content.get(&filepath.dir)?.get(&filepath.file)
    }

    /// Check if a file exists in the virtual filesystem
    pub fn file_exists(&self, filepath: &FilePath) -> bool {
        self.content
            .get(&filepath.dir)
            .and_then(|files| files.get(&filepath.file))
            .is_some()
    }

    /// Remove a file from the virtual filesystem
    pub fn remove_file(&mut self, filepath: &FilePath) -> bool {
        if let Some(files) = self.content.get_mut(&filepath.dir) {
            files.remove(&filepath.file).is_some()
        } else {
            false
        }
    }

    /// Create a directory
    pub fn create_dir(&mut self, dirpath: DirPath) {
        self.content.entry(dirpath).or_insert_with(HashMap::new);
    }

    /// Check if a directory exists
    pub fn dir_exists(&self, dirpath: &DirPath) -> bool {
        self.content.contains_key(dirpath)
    }

    /// Remove a directory (only if empty)
    pub fn remove_dir(&mut self, dirpath: &DirPath) -> Result<(), String> {
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
    pub fn list_files_in_dir(&self, dirpath: &DirPath) -> Vec<String> {
        if let Some(files) = self.content.get(dirpath) {
            let mut filenames: Vec<String> = files.keys().cloned().collect();
            filenames.sort();
            filenames
        } else {
            Vec::new()
        }
    }

    /// Get all immediate subdirectories of a given directory (returns just dir names)
    pub fn list_subdirs_in_dir(&self, dirpath: &DirPath) -> Vec<String> {
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
