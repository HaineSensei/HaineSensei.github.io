use std::collections::{HashMap, HashSet};
use crate::filesystem::{FilePath, helpers::fetch_text};

use super::types::{DirPath, Content};

/// Error indicating that an operation requires data that isn't cached yet
#[derive(Debug)]
pub struct NeedsFetch;

#[derive(Clone)]
pub struct AbyssFileSystem {
    pub files: HashMap<DirPath, Contents>,
    pub dirs: HashMap<DirPath, Directories>,
}

#[derive(Clone)]
pub struct Contents(pub HashMap<String, Content>);

#[derive(Debug, Clone)]
pub struct Directories(pub HashSet<String>);

impl Contents {
    /// Parse !!contents.txt into Contents
    pub fn from_file(text: &str) -> Self {
        Contents(
            text.lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(|name| (name.to_string(), Content::ToFetch))
                .collect()
        )
    }

    /// Merge in-memory additions
    pub fn extend(&mut self, other: Contents) {
        self.0.extend(other.0);
    }

    pub fn new() -> Self {
        Contents(HashMap::new())
    }

    /// Get content by filename
    pub fn get(&self, filename: &str) -> Option<&Content> {
        self.0.get(filename)
    }
}

impl Directories {
    /// Parse !!directories.txt into Directories
    pub fn from_file(text: &str) -> Self {
        Directories(
            text.lines()
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .map(|s| s.to_string())
                .collect()
        )
    }

    /// Merge in-memory additions
    pub fn extend(&mut self, other: &Directories) {
        self.0.extend(other.0.clone());
    }

    pub fn new() -> Self {
        Directories(HashSet::new())
    }

    /// Check if directory name exists
    pub fn contains(&self, dirname: &str) -> bool {
        self.0.contains(dirname)
    }
}

impl AbyssFileSystem {
    pub fn new() -> Self {
        AbyssFileSystem {
            files: HashMap::new(),
            dirs: HashMap::new(),
        }
    }

    /// Try to remove a file using cached data only
    pub fn sync_remove_file(&mut self, filepath: &FilePath) -> Result<(), NeedsFetch> {
        if let Some(contents) = self.files.get_mut(&filepath.dir) {
            // Cached - modify in place
            contents.0.remove(&filepath.file);
            Ok(())
        } else {
            Err(NeedsFetch)
        }
    }

    /// Remove a file using provided contents data
    pub fn sync_remove_file_with_data(&mut self, filepath: &FilePath, mut contents: Contents) -> Result<(), String> {
        if contents.0.remove(&filepath.file).is_some() {
            self.files.insert(filepath.dir.clone(), contents);
            Ok(())
        } else {
            Err(format!("No such file: {}", filepath.to_string()))
        }
    }

    /// Try to remove a directory using cached data only
    pub fn sync_remove_dir(&mut self, dirpath: &DirPath) -> Result<(), NeedsFetch> {
        // Check if we have cached data for the directory itself
        let contents = self.files.get(dirpath).ok_or(NeedsFetch)?;
        let directories = self.dirs.get(dirpath).ok_or(NeedsFetch)?;

        // Check if empty
        if !contents.0.is_empty() || !directories.0.is_empty() {
            return Err(NeedsFetch); // Signal error through NeedsFetch to trigger re-check with fresh data
        }

        // Check if we have cached parent
        let parent = dirpath.super_dir().ok_or(NeedsFetch)?;
        let parent_dirs = self.dirs.get_mut(&parent).ok_or(NeedsFetch)?;

        // All cached - can proceed
        if let Some(dirname) = dirpath.final_component() {
            parent_dirs.0.remove(dirname);
            self.files.remove(dirpath);
            self.dirs.remove(dirpath);
            Ok(())
        } else {
            Err(NeedsFetch)
        }
    }

    /// Remove a directory using provided data
    pub fn sync_remove_dir_with_data(
        &mut self,
        dirpath: &DirPath,
        contents: Contents,
        directories: Directories,
        parent_dirs: Directories,
    ) -> Result<(), String> {
        // Check if directory is empty
        if !contents.0.is_empty() || !directories.0.is_empty() {
            return Err("Directory not empty".to_string());
        }

        // Get parent directory and directory name
        match (dirpath.super_dir(), dirpath.final_component()) {
            (Some(parent), Some(dirname)) => {
                let mut parent_dirs = parent_dirs;
                parent_dirs.0.remove(dirname);

                // TODO: handle case of rmdir /abyss when /abyss empty
                // Update cache
                self.dirs.insert(parent, parent_dirs);
                // Clean up entries for the removed directory
                self.files.remove(dirpath);
                self.dirs.remove(dirpath);

                Ok(())
            }
            _ => Err("Invalid path".to_string())
        }
    }

    /// Try to create a directory using cached data only
    pub fn sync_create_dir(&mut self, dirpath: &DirPath) -> Result<(), NeedsFetch> {
        // Get parent directory and directory name
        let parent = dirpath.super_dir().ok_or(NeedsFetch)?;
        let dir_name = dirpath.final_component().ok_or(NeedsFetch)?;

        // Check if we have cached parent
        let parent_dirs = self.dirs.get_mut(&parent).ok_or(NeedsFetch)?;

        // All cached - can proceed
        parent_dirs.0.insert(dir_name.to_string());

        // Initialize empty Contents and Directories for new directory
        self.files.insert(dirpath.clone(), Contents::new());
        self.dirs.insert(dirpath.clone(), Directories::new());

        Ok(())
    }

    /// Create a directory using provided parent directories data
    pub fn sync_create_dir_with_data(
        &mut self,
        dirpath: &DirPath,
        mut parent_dirs: Directories,
    ) -> Result<(), String> {
        match (dirpath.super_dir(), dirpath.final_component()) {
            (Some(parent), Some(dir_name)) => {
                parent_dirs.0.insert(dir_name.to_string());

                // Update cache
                self.dirs.insert(parent, parent_dirs);

                // Initialize empty Contents and Directories for new directory
                self.files.insert(dirpath.clone(), Contents::new());
                self.dirs.insert(dirpath.clone(), Directories::new());

                Ok(())
            }
            _ => Err("Invalid path".to_string())
        }
    }

    /// Try to write a file using cached data only
    pub fn sync_write_file(&mut self, filepath: &FilePath, content: String) -> Result<(), NeedsFetch> {
        if let Some(contents) = self.files.get_mut(&filepath.dir) {
            // Cached - modify in place
            contents.0.insert(filepath.file.clone(), Content::InMemory(content));
            Ok(())
        } else {
            Err(NeedsFetch)
        }
    }

    /// Write a file using provided contents data
    pub fn sync_write_file_with_data(&mut self, filepath: &FilePath, mut contents: Contents, content: String) {
        contents.0.insert(filepath.file.clone(), Content::InMemory(content));
        self.files.insert(filepath.dir.clone(), contents);
    }

    pub async fn get_contents(&self, dirpath: &DirPath) -> Contents {
        match self.files.get(dirpath) {
            Some(x) => x.clone(),
            None => Contents::from_file(
                &fetch_text(
                    &format!("{}/!!contents.txt", dirpath.to_string())
                ).await.unwrap()
            )
        }
    }

    pub async fn get_directories(&self, dirpath: &DirPath) -> Directories {
        match self.dirs.get(dirpath) {
            Some(x) => x.clone(),
            None => Directories::from_file(
                &fetch_text(
                    &format!("{}/!!directories.txt", dirpath.to_string())
                ).await.unwrap()
            )
        }
    }
}